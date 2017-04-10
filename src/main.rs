extern crate futures;
extern crate tokio_proto;
extern crate tokio_service;
extern crate hyper;
extern crate native_tls;
extern crate tokio_tls;
extern crate num_cpus;
#[macro_use] extern crate mime;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate rustls;
extern crate tokio_rustls;

use tokio_proto::TcpServer;
use futures::{future, Future, Stream};
use tokio_service::Service;
use hyper::server::{Http, Request, Response};
use hyper::Method::{Get, Post};
use hyper::header::{ContentLength, ContentType};
use hyper::status::StatusCode::NotFound;
use std::net::SocketAddr;
use native_tls::{TlsAcceptor, Pkcs12};
use std::io::{Read, BufReader};
use std::sync::Arc;
use std::fs::File;
use rustls::{ Certificate, PrivateKey, ServerConfig };
use rustls::internal::pemfile::{ certs, rsa_private_keys };

static INDEX: &'static [u8] = b"Hello, world!";

struct HttpServer;

impl Service for HttpServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::error::Error;
    type Future = Box<Future<Item = Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/plaintext") | (&Get, "/") => {
                future::ok(Response::new()
                        .with_header(ContentLength(INDEX.len() as u64))
                        .with_header(ContentType(mime!(Text/Plain)))
                        .with_body(INDEX))
                    .boxed()
            }
            (&Get, "/json") => {
                let rep = TestResponse { message: "Hello, world!" };
                let rep_body = serde_json::to_vec(&rep).unwrap();
                future::ok(Response::new()
                        .with_header(ContentLength(rep_body.len() as u64))
                        .with_header(ContentType(mime!(Application/Json)))
                        .with_body(rep_body))
                    .boxed()
            }
            (&Post, "/echo") => {
                req.body().collect()
                .and_then(|chunk| {
                    let mut buffer: Vec<u8> = Vec::new();
                    for i in chunk {
                        buffer.append(&mut i.to_vec());
                    }
                    Ok(buffer)
                })
                .map(|buffer| {
                    Response::new()
                        .with_header(ContentLength(buffer.len() as u64))
                        .with_body(buffer)
                }).boxed()
            }
            _ => future::ok(Response::new().with_status(NotFound)).boxed()
        }
    }
}

#[derive(Serialize)]
struct TestResponse<'a> {
    message: &'a str
}

fn main() {
    run().unwrap();
}

fn run() -> std::result::Result<(), std::io::Error> {
    println!("Go!");
    let s = "abc".to_string();
    println!("{}", s);

    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let http_thread = std::thread::spawn(move || {
        let mut tcp = TcpServer::new(Http::new(), addr);
        tcp.threads(num_cpus::get());
        tcp.serve(|| Ok(HttpServer));
    });

    let mut file = std::fs::File::open("identity.pfx")?;
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12)?;
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").expect("");
    let acceptor = TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();

    let addr: SocketAddr = "0.0.0.0:8443".parse().unwrap();
    let https_thread = std::thread::spawn(move || {
        let tls = tokio_tls::proto::Server::new(Http::new(), acceptor);
        let mut tcp = TcpServer::new(tls, addr);
        tcp.threads(num_cpus::get());
        tcp.serve(|| Ok(HttpServer));
    });

    let mut config = ServerConfig::new();
    config.set_single_cert(load_certs("end.fullchain"), load_private_key("end.rsa"));
    let arc_config = Arc::new(config);

    let addr: SocketAddr = "0.0.0.0:9443".parse().unwrap();
    let rustls_thread = std::thread::spawn(move || {
        let tls = tokio_rustls::proto::Server::new(Http::new(), arc_config);
        let mut tcp = TcpServer::new(tls, addr);
        tcp.threads(num_cpus::get());
        tcp.serve(|| Ok(HttpServer));
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