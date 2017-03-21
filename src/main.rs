extern crate futures;
extern crate tokio_proto;
extern crate tokio_service;
extern crate hyper;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate native_tls;
extern crate tokio_tls;
extern crate num_cpus;

use tokio_proto::TcpServer;
use futures::Future;
use futures::Stream;
use tokio_service::Service;
use hyper::server;
use hyper::error;
use hyper::Method::{Get, Post};
use hyper::header::ContentLength;
use hyper::status::StatusCode::{NotFound};
use std::net::{SocketAddr};
use native_tls::{Pkcs12};
use std::fs::File;
use std::io::{Read};

fn main() {
    println!("Let's get on to it!");
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let http_thread = std::thread::spawn(move || {
        let mut tcp = TcpServer::new(server::Http::new(), addr);
        tcp.threads(num_cpus::get());
        tcp.serve(move || {
                Ok(HttpService { inner: HttpServer })
            });
    });

    let mut file = File::open("identity.pfx").unwrap();
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12).unwrap();
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").unwrap();
    let acceptor = native_tls::TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();
    let addr: SocketAddr = "0.0.0.0:8443".parse().unwrap();
    let https_thread = std::thread::spawn(move || {
        let tls = tokio_tls::proto::Server::new(server::Http::new(), acceptor);
        let mut tcp = TcpServer::new(tls, addr);
        tcp.serve(move || {
                Ok(HttpService { inner: HttpServer })
            });
    });

    http_thread.join().unwrap();
    https_thread.join().unwrap();
}

static INDEX: &'static [u8] = b"Hello, world!";

struct HttpServer;

impl Service for HttpServer {
    type Request = server::Request;
    type Response = server::Response;
    type Error = error::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
    fn call(&self, req: server::Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/plaintext") | (&Get, "/") => {
                futures::future::ok(server::Response::new()
                        .with_header(ContentLength(INDEX.len() as u64))
                        .with_body(INDEX))
                    .boxed()
            }
            (&Get, "/json") => {
                //let s = String::from_utf8(vec![b'X', 250]).unwrap(); // "Hello, world!".to_string()
                let rep = TestResponse { message: "Hello, world!".to_string() };
                let rep_body = serde_json::to_vec(&rep).unwrap();
                futures::future::ok(server::Response::new()
                        .with_header(ContentLength(rep_body.len() as u64))
                        .with_body(rep_body))
                    .boxed()
            }
            (&Post, "/echo") => {
                req.body().collect()
                .and_then(move |chunk| {
                    let mut buffer: Vec<u8> = Vec::new();
                    for i in chunk {
                        buffer.append(&mut i.to_vec());
                    }
                    Ok(buffer)
                })
                .map(move |buffer| {
                    server::Response::new()
                        .with_header(ContentLength(buffer.len() as u64))
                        .with_body(buffer)
                }).boxed()
            }
            (&Post, "/nodes") => {
                // Get all of the chunks streamed to us in our request
                // GitHub gives us a lot of data so there might be
                // more than one Chunk
                req.body().collect()
                // Then put them all into a single buffer for parsing
                .and_then(move |chunk| {
                    let mut buffer: Vec<u8> = Vec::new();
                    for i in chunk {
                        buffer.append(&mut i.to_vec());
                    }
                    Ok(buffer)
                })
                // If there is JSON do things with it
                // Send to the server that we got the data
                .map(move |buffer| {
                    if !buffer.is_empty() {
                        println!("{:#?}", buffer.len());
                    }

                    server::Response::new()
                }).boxed()

            }
            _ => {

                let mut res = server::Response::new();
                res.set_status(NotFound);
                futures::finished(res).boxed()

            }
        }
    }
}

struct HttpService<T> {
    inner: T
}

use tokio_proto::streaming::Message;
use hyper::server::{Request, Response};

impl<T, B> Service for HttpService<T>
    where T: Service<Request=Request, Response=Response<B>, Error=hyper::Error>,
          B: Stream<Error=hyper::Error>,
          B::Item: AsRef<[u8]>,
{
    type Request = Message<server::__ProtoRequest, tokio_proto::streaming::Body<hyper::Chunk, hyper::Error>>;
    type Response = Message<hyper::server::__ProtoResponse, B>;
    type Error = hyper::Error;
    type Future = futures::future::Map<T::Future, fn(Response<B>) -> Message<hyper::server::__ProtoResponse, B>>;

    fn call(&self, message: Self::Request) -> Self::Future {
        let req = Request::from(message);
        self.inner.call(req).map(Into::into)
    }
}

#[derive(Serialize)]
struct TestResponse {
    message: String
}