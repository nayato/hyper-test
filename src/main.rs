#![feature(conservative_impl_trait)]

extern crate futures;
extern crate tokio_proto;
extern crate tokio_service;
#[macro_use]
extern crate hyper;
extern crate native_tls;
extern crate tokio_tls;
extern crate num_cpus;
extern crate mime;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate rustls;
extern crate tokio_rustls;
extern crate url;

use tokio_proto::TcpServer;
use hyper::server::Http;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use native_tls::{TlsAcceptor, Pkcs12};
use std::io::{Read, BufReader};
use std::sync::Arc;
use std::fs::File;
use rustls::{Certificate, ServerConfig};
use rustls::internal::pemfile::certs;

mod http_server;

fn main() {
    run().unwrap();
}

fn run() -> std::result::Result<(), std::io::Error> {
    println!("Starting...");

    let any_ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let addr: SocketAddr = SocketAddr::new(any_ip, 8080);
    let http_thread = std::thread::spawn(move || {
                                             let mut tcp = TcpServer::new(Http::new(), addr);
                                             tcp.threads(num_cpus::get());
                                             tcp.serve(|| Ok(http_server::HttpServer));
                                         });

    let mut file = std::fs::File::open("identity.pfx")?;
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12)?;
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").expect("");
    let acceptor = TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();

    let addr: SocketAddr = SocketAddr::new(any_ip, 8443);
    let https_thread = std::thread::spawn(move || {
                                              let tls = tokio_tls::proto::Server::new(Http::new(),
                                                                                      acceptor);
                                              let mut tcp = TcpServer::new(tls, addr);
                                              tcp.threads(num_cpus::get());
                                              tcp.serve(|| Ok(http_server::HttpServer));
                                          });

    let mut config = ServerConfig::new();
    config.set_single_cert(load_certs("end.fullchain"), load_private_key("end.rsa"));
    let arc_config = Arc::new(config);

    let addr: SocketAddr = SocketAddr::new(any_ip, 9443);
    let rustls_thread =
        std::thread::spawn(move || {
                               let tls = tokio_rustls::proto::Server::new(Http::new(), arc_config);
                               let mut tcp = TcpServer::new(tls, addr);
                               tcp.threads(num_cpus::get());
                               tcp.serve(|| Ok(http_server::HttpServer));
                           });

    http_thread.join().unwrap();
    https_thread.join().unwrap();
    rustls_thread.join().unwrap();
    Ok(())
}

fn load_certs(path: &str) -> Vec<Certificate> {
    let res = certs(&mut BufReader::new(File::open(path).unwrap())).unwrap();
    assert!(res.len() > 0);
    res
}

fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    let keys = rustls::internal::pemfile::rsa_private_keys(&mut reader).unwrap();
    assert!(keys.len() == 1);
    keys[0].clone()
}
