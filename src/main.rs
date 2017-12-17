#![feature(proc_macro, conservative_impl_trait, generators, vec_resize_default)]

#[macro_use]
extern crate futures_await as futures;
extern crate hyper;
extern crate mime;
extern crate native_tls;
extern crate net2;
extern crate num_cpus;
extern crate rustls;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_rustls;
extern crate tokio_tls;
extern crate url;
extern crate tk_http;
extern crate tk_listen;
extern crate time;
extern crate httpdate;

use futures::prelude::*;
use futures::future;
use tokio_core::reactor::Handle;
use tokio_core::net::{TcpListener, TcpStream};
use hyper::server::Http;
use tokio_tls::TlsAcceptorExt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use native_tls::{Pkcs12, TlsAcceptor};
use std::io::{BufReader, Read};
use std::sync::Arc;
use std::fs::File;
use rustls::{Certificate, ServerConfig};
use rustls::internal::pemfile::certs;

use tk_http::server::buffered::BufferedDispatcher;
use tk_http::server::{Config, Proto};
use tk_listen::ListenExt;

mod http_server;
mod tk_http_server;

fn main() {
    run().unwrap();
}

fn run() -> std::result::Result<(), std::io::Error> {
    let threads = num_cpus::get();
    println!("Starting on {} threads", threads);

    let any_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let addr: SocketAddr = SocketAddr::new(any_ip, 80);
    let http_threads = (0..threads).map(|_| {
        std::thread::spawn(move || {
            serve(addr, |socket| future::ok(socket))
        })
    });
    // let http_thread = std::thread::spawn(move || {
    //     let mut tcp = TcpServer::new(Http::new(), addr);
    //     tcp.threads(num_cpus::get());
    //     tcp.serve(|| Ok(http_server::HttpServer));
    // });

    let mut file = std::fs::File::open("gateway.tests.com.pfx")
        .expect("TLS certificate file must be present in current dir");
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12)
        .expect("could not read TLS cert file");
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").expect("Could not load TLS cert");
    let acceptor = TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();

    let addr: SocketAddr = SocketAddr::new(any_ip, 443);
    let https_threads = (0..threads).map(|_| {
        let acceptor = acceptor.clone();
        std::thread::spawn(move || {
            serve(addr, move |socket| init_tls(socket, acceptor.clone()))
        })
    });

    // let mut config = ServerConfig::new();
    // config.set_single_cert(load_certs("end.fullchain"), load_private_key("end.rsa"));
    // let arc_config = Arc::new(config);

    // let addr: SocketAddr = SocketAddr::new(any_ip, 9443);
    // let rustls_thread = std::thread::spawn(move || {
    //     let tls = tokio_rustls::proto::Server::new(Http::new(), arc_config);
    //     let mut tcp = TcpServer::new(tls, addr);
    //     tcp.threads(num_cpus::get());
    //     tcp.serve(|| Ok(http_server::HttpServer));
    // });


    let addr: SocketAddr = SocketAddr::new(any_ip, 10080);
    let cfg = Config::new().done();
    let tk_http_threads = (0..threads).map(|_| {
        let cfg = cfg.clone();
        std::thread::spawn(move || {
            tk_serve(addr, &cfg, |socket| future::ok(socket))
        })
    });

    let addr: SocketAddr = SocketAddr::new(any_ip, 10443);
    let tk_https_threads = (0..threads).map(|_| {
        let acceptor = acceptor.clone();
        let cfg = cfg.clone();
        std::thread::spawn(move || {
            tk_serve(addr, &cfg, move |socket| init_tls(socket, acceptor.clone()))
        })
    });

    for thread in https_threads
        .chain(http_threads)
        .chain(tk_http_threads)
        .chain(tk_https_threads)
        .collect::<Vec<_>>() {
        thread.join().unwrap();
    }
    // rustls_thread.join().unwrap();
    Ok(())
}

#[async]
fn init_tls(socket: TcpStream, acceptor: TlsAcceptor) -> std::io::Result<tokio_tls::TlsStream<TcpStream>> {
    let io = await!(acceptor.accept_async(socket).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))?;
    Ok(io)
}

fn load_certs(path: &str) -> Vec<Certificate> {
    let res = certs(&mut BufReader::new(File::open(path).unwrap())).unwrap();
    assert!(!res.is_empty());
    res
}

fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = File::open(filename).expect("Cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    let keys = rustls::internal::pemfile::rsa_private_keys(&mut reader).unwrap();
    assert_eq!(1, keys.len());
    keys[0].clone()
}

fn serve<F, Ft, Io>(addr: SocketAddr, augment_io: F)
where
    F: Fn(TcpStream) -> Ft,
    Ft: Future<Item = Io, Error=std::io::Error> + 'static,
    Io: tokio_io::AsyncRead + tokio_io::AsyncWrite + 'static,
{
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let listener = listener(&addr, &handle).unwrap();

    let io_stream = listener.incoming().and_then(|(socket, _addr)| augment_io(socket));
    let http = Http::<hyper::Chunk>::new();
    let serve = http.serve_incoming(io_stream, || Ok(http_server::HttpServer))
        .for_each(|conn| {
            handle.spawn(conn.map(|_| ()).map_err(|_e| eprintln!("{:?} ", _e)));
            Ok(())
        });
    core.run(serve).unwrap();
}

fn tk_serve<F, Ft, Io>(addr: SocketAddr, cfg: &Arc<Config>, augment_io: F)
where
    F: Fn(TcpStream) -> Ft,
    Ft: Future<Item = Io, Error=std::io::Error> + 'static,
    Io: tokio_io::AsyncRead + tokio_io::AsyncWrite + 'static,
{
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let listener = listener(&addr, &handle).unwrap();

    let io_stream = listener.incoming()
        //.sleep_on_error(Duration::from_millis(100), &handle)
        .and_then(|(socket, _addr)| augment_io(socket))
        .map(|socket|
            Proto::new(socket, &cfg,
                BufferedDispatcher::new(addr, &handle, move || {
                    move |r, e| {
                        tk_http_server::service(r, e)
                    }
                }),
                &handle)
            .map_err(|e| { println!("Connection error: {}", e); }))
        .listen(50000);
    core.run(io_stream).unwrap();
}

fn listener(addr: &SocketAddr, handle: &Handle) -> std::io::Result<TcpListener> {
    let listener = match *addr {
        SocketAddr::V4(_) => net2::TcpBuilder::new_v4()?,
        SocketAddr::V6(_) => net2::TcpBuilder::new_v6()?,
    };
    configure_tcp(&listener)?;
    listener.reuse_address(true)?;
    listener.bind(addr)?;
    listener
        .listen(1024)
        .and_then(|l| TcpListener::from_listener(l, addr, handle))
}

#[cfg(unix)]
fn configure_tcp(tcp: &net2::TcpBuilder) -> std::io::Result<()> {
    use net2::unix::*;
    tcp.reuse_port(true)?;
    Ok(())
}

#[cfg(windows)]
fn configure_tcp(_tcp: &net2::TcpBuilder) -> std::io::Result<()> {
    Ok(())
}
