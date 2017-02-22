extern crate futures;
extern crate tokio_core;
extern crate tokio_pool;
extern crate num_cpus;
extern crate hyper;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate native_tls;
extern crate tokio_tls;

use tokio_core::net::TcpListener;
use tokio_core::io::Io;
use futures::Future;
use futures::Stream;
use hyper::server::{Service, Http};
use hyper::server;
use hyper::error;
use hyper::Method::{Get, Post};
use hyper::header::ContentLength;
use hyper::status::StatusCode::{NotFound, Created};
use std::net::{SocketAddr};
use std::hash::{Hash, Hasher};
use native_tls::{Pkcs12};
use std::fs::File;
use std::io::{Read};
use tokio_tls::{TlsAcceptorExt};
use std::sync::Arc;

fn main() {
    println!("Let's get on to it!");
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let (pool, join) = tokio_pool::TokioPool::new(num_cpus::get()).expect("Failed to create event loop");
    let pool = Arc::new(pool);
    let pool_ref = pool.clone();
    // Use the first pool worker to listen for connections
    pool.next_worker().spawn(move |handle| {
        // Bind a TCP listener to our address
        let listener = TcpListener::bind(&addr, handle).unwrap();
        // Listen for incoming clients
        listener.incoming().for_each(move |(socket, addr)| {
            pool_ref.next_worker().spawn(move |handle| {
                socket.set_nodelay(true).unwrap();
                handle_http(addr, socket, &handle);
                Ok(())
                // Do work with a client socket
            });

            Ok(())
        }).map_err(|_| ()) // todo: log errors
    });

    let mut file = File::open("identity.pfx").unwrap();
    let mut pkcs12 = vec![];
    file.read_to_end(&mut pkcs12).unwrap();
    let pkcs12 = Pkcs12::from_der(&pkcs12, "password").unwrap();
    let acceptor = native_tls::TlsAcceptor::builder(pkcs12).unwrap().build().unwrap();
    let acceptor = Arc::new(acceptor);
    let addr_tls: SocketAddr = "0.0.0.0:8443".parse().unwrap();
    // Use the first pool worker to listen for connections
    let pool_ref = pool.clone();
    pool.next_worker().spawn(move |handle| {
        // Bind a TCP listener to our address
        let listener = TcpListener::bind(&addr_tls, handle).unwrap();
        // Listen for incoming clients
        listener.incoming().for_each(move |(socket, addr)| {
            let acceptor = acceptor.clone();
            pool_ref.next_worker().spawn(move |elh| {
                socket.set_nodelay(true).unwrap();
                let remote = elh.remote().clone();
                let handshake = acceptor.accept_async(socket);
                let handling = handshake.and_then(move |socket| {
                        handle_http(addr, socket, &remote.handle().expect("remote->handle failed"));
                        Ok(())
                    });
                let handled = handling
                    .map_err(|e| {
                            println!("{}", e);
                            // todo: handle handshake errors
                            ()
                        });
                return handled;
                // Do work with a client socket
            });

            Ok(())
        }).map_err(|e| {
                println!("{}", e);
                ()
            }) // todo: log errors
    });

    join.join();
}

fn handle_http<I>(addr: SocketAddr, socket: I, handle: &tokio_core::reactor::Handle) where I: Io + 'static {
    let id = handle.id();
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    id.hash(&mut hasher);
    Http::new().bind_connection(&handle, socket, addr, HttpServer { thread_id: hasher.finish() });
}

static INDEX: &'static [u8] = b"Hello, world!";

struct HttpServer {
    thread_id: u64
}

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
                        .with_body(INDEX)
                        .with_status(Created))
                    .boxed()
            }
            (&Get, "/json") => {
                let rep = TestResponse { message: "Hello, world!".to_string(), wid: self.thread_id };
                let rep_body = serde_json::to_vec(&rep).unwrap();
                futures::future::ok(server::Response::new()
                        .with_header(ContentLength(rep_body.len() as u64))
                        .with_body(rep_body)
                        .with_status(Created))
                    .boxed()
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

#[derive(Serialize, Deserialize)]
struct TestResponse {
    message: String,
    wid: u64
}