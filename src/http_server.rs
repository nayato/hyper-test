use futures::{future, BoxFuture, Future, Stream};
use futures::future::Either;
use tokio_service::Service;
use hyper::server::{Request, Response};
use hyper::Method::{Get, Post};
use hyper::header::{ContentLength, ContentType, Server};
use hyper::StatusCode::NotFound;
use url::form_urlencoded;
use std::cmp;
use std::ascii::AsciiExt;
use std::ops::Deref;
use mime::{Mime, APPLICATION_JSON, TEXT_PLAIN};

const INDEX_STR: &str = include_str!("lorem.txt");
static INDEX: &[u8] = include_bytes!("lorem.txt");
static SERVER_NAME: &str = "hyper";

type SyncResponseFuture = future::FutureResult<Response, ::hyper::Error>;

pub struct HttpServer;

impl HttpServer {
    fn get_requested_size(&self, req: &Request) -> usize {
        cmp::min(
            INDEX.len(),
            req.query()
                .and_then(|q| {
                    form_urlencoded::parse(q.as_bytes())
                        .into_iter()
                        .find(|x| x.0.eq_ignore_ascii_case("size"))
                })
                .and_then(|x| x.1.parse::<usize>().ok())
                .unwrap_or(13),
        ) // Hello, world!
    }

    fn get_content_str(&self, req: &Request) -> &str {
        &INDEX_STR[..self.get_requested_size(req)]
    }

    fn get_content_bytes(&self, req: &Request) -> &'static [u8] {
        &INDEX[..self.get_requested_size(req)]
    }

    fn complete_response<T>(&self, content_type: Mime, content: T) -> Response
    where
        T: Into<::hyper::Body> + Deref<Target = [u8]>,
    {
        Response::new()
            .with_header(ContentLength(content.len() as u64))
            .with_header(ContentType(content_type))
            .with_header(Server::new(SERVER_NAME))
            .with_body(content)
    }
}


impl Service for HttpServer {
    type Request = Request;
    type Response = Response;
    type Error = ::hyper::error::Error;
    type Future = Either<SyncResponseFuture, BoxFuture<Self::Response, Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/plaintext") | (&Get, "/") => {
                let content = self.get_content_bytes(&req);
                Either::A(future::ok(self.complete_response(TEXT_PLAIN, content)))
            }
            (&Get, "/json") => {
                let content = self.get_content_str(&req);
                let rep = TestResponse { message: content };
                let rep_body = ::serde_json::to_vec(&rep).unwrap();
                Either::A(future::ok(
                    self.complete_response(APPLICATION_JSON, rep_body),
                ))
            }
            (&Post, "/echo") => Either::B(
                req.body()
                    .collect()
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
                            .with_header(Server::new(SERVER_NAME))
                            .with_body(buffer)
                    })
                    .boxed(),
            ),
            _ => Either::A(future::ok(Response::new().with_status(NotFound))),
        }
    }
}

#[derive(Serialize)]
struct TestResponse<'a> {
    message: &'a str,
}
