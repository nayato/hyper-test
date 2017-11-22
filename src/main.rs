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

mod http_server;

fn main() {
    run().unwrap();
}

fn run() -> std::result::Result<(), std::io::Error> {
    let threads = num_cpus::get();
    println!("Starting on {} threads", threads);

    let any_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let addr: SocketAddr = SocketAddr::new(any_ip, 10080);
    let http_threads = (0..threads).map(|_| {
        std::thread::spawn(move || {
            println!("1.5");
            serve(addr, |socket| future::ok(socket))
        })
    });
    println!("2");
    // let http_thread = std::thread::spawn(move || {
    //     let mut tcp = TcpServer::new(Http::new(), addr);
    //     tcp.threads(num_cpus::get());
    //     tcp.serve(|| Ok(http_server::HttpServer));
    // });

    let mut file = std::fs::File::open("gateway.tests.com.pfx")
        .expect("TLS cert file must be present in current dir");
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12)
        .expect("could not read TLS cert file");
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").expect("could not load TLS cert");
    let acceptor = TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();
    println!("3");

    let addr: SocketAddr = SocketAddr::new(any_ip, 10443);
    let https_threads = (0..threads).map(|_| {
        let acceptor = acceptor.clone();
        std::thread::spawn(move || {
            println!("3.5");
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

    println!("4");
    for thread in https_threads.chain(http_threads).collect::<Vec<_>>() {
            println!("4.1");
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
    let keyfile = File::open(filename).expect("cannot open private key file");
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

    let server = listener.incoming().for_each(|(socket, _addr)| {
        let conn = augment_io(socket)
            .and_then(|io| {
                Http::<hyper::Chunk>::new()
                    .serve_connection(io, http_server::HttpServer).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            })
            .map(|_| ())
            .map_err(|_e| eprintln!("{:?}", _e));
        handle.spawn(conn);
        Ok(())
    });

    core.run(server).unwrap();
}

fn listener(addr: &SocketAddr, handle: &Handle) -> std::io::Result<TcpListener> {
    let listener = match *addr {
        SocketAddr::V4(_) => try!(net2::TcpBuilder::new_v4()),
        SocketAddr::V6(_) => try!(net2::TcpBuilder::new_v6()),
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
    println!("1.1");
    tcp.reuse_port(true)?;
    Ok(())
}

#[cfg(windows)]
fn configure_tcp(_tcp: &net2::TcpBuilder) -> std::io::Result<()> {
    println!("1");
    Ok(())
}
