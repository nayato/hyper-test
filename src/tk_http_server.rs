use url::form_urlencoded;
use std::cmp;
use std::ascii::AsciiExt;

use std::cell::RefCell;

use futures::{future, Stream, Future};
use futures::future::{FutureResult, ok, Either};

use tk_http::Status;
use tk_http::server::buffered::{Request, BufferedDispatcher};
use tk_http::server::{Encoder, EncoderDone, Config, Proto, Error};
use time::{self, Duration};
use std::fmt::Write;
use std::str::FromStr;
use hyper::Uri;

const INDEX_STR: &str = include_str!("lorem.txt");
static INDEX: &[u8] = include_bytes!("lorem.txt");
static SERVER_NAME: &str = "hyper";

pub fn service<S>(req: Request, mut e: Encoder<S>) -> FutureResult<EncoderDone<S>, Error>
{
    let uri = Uri::from_str(req.path()).unwrap(); // todo: bubble up error

    match (req.method(), uri.path()) {
        ("GET", "/plaintext") => {
            e.status(Status::Ok);
            let content = get_content_bytes(&uri);
            e.add_length(content.len() as u64).unwrap();
            e.add_header("Content-Type", "text/plain").unwrap();
            e.format_header("Date", time::now_utc().rfc822()).unwrap();
            e.add_header("Server", "tk_http").unwrap();
            if e.done_headers().unwrap() {
                e.write_body(content);
            }
        }
        ("GET", "/") => {
            e.status(Status::Ok);
            let content = get_content_bytes(&uri);
            e.add_length(content.len() as u64).unwrap();
            e.add_header("Content-Type", "text/plain").unwrap();
            add_date(&mut e);
            e.add_header("Server", "tk_http").unwrap();
            if e.done_headers().unwrap() {
                e.write_body(content);
            }
        }
        ("GET", "/json") => {
            e.status(Status::Ok);
            let content = get_content_str(&uri);
            let rep = TestResponse { message: content };
            let content = ::serde_json::to_vec(&rep).unwrap();
            e.add_length(content.len() as u64).unwrap();
            e.add_header("Content-Type", "application/json").unwrap();
            add_date(&mut e);
            e.add_header("Server", "tk_http").unwrap();
            if e.done_headers().unwrap() {
                e.write_body(&content);
            }
        }
        _ => {
            e.status(Status::NotFound);
            e.add_header("Server", "tk_http").unwrap();
            e.add_length(0);
            e.done_headers().unwrap();
        },
    }
    ok(e.done())
}

fn add_date<S>(e: &mut Encoder<S>) {
    CACHED.with(|cache| {
        let mut cache = cache.borrow_mut();
        let now = time::get_time();
        if now > cache.next_update {
            cache.update(now);
        }
        e.add_header("Date", cache.buffer()).unwrap();
    })
}

pub const DATE_VALUE_LENGTH: usize = 29;

struct CachedDate {
    bytes: [u8; DATE_VALUE_LENGTH],
    pos: usize,
    next_update: time::Timespec,
}

thread_local!(static CACHED: RefCell<CachedDate> = RefCell::new(CachedDate {
    bytes: [0; DATE_VALUE_LENGTH],
    pos: 0,
    next_update: time::Timespec::new(0, 0),
}));

impl CachedDate {
    fn buffer(&self) -> &[u8] {
        &self.bytes[..]
    }

    fn update(&mut self, now: time::Timespec) {
        self.pos = 0;
        write!(self, "{}", time::at_utc(now).rfc822()).unwrap();
        assert!(self.pos == DATE_VALUE_LENGTH);
        self.next_update = now + Duration::seconds(1);
        self.next_update.nsec = 0;
    }
}

impl ::std::fmt::Write for CachedDate {
    fn write_str(&mut self, s: &str) -> ::std::fmt::Result {
        let len = s.len();
        self.bytes[self.pos..self.pos + len].copy_from_slice(s.as_bytes());
        self.pos += len;
        Ok(())
    }
}

fn get_requested_size(uri: &Uri) -> usize {
    let query_size = uri.query()
        .and_then(|q| {
                form_urlencoded::parse(q.as_bytes())
                    .into_iter()
                    .find(|x| x.0.eq_ignore_ascii_case("size"))
            })
        .and_then(|x| x.1.parse::<usize>().ok())
        .unwrap_or(13); // Hello, world!
    cmp::min(
        INDEX.len(),
        query_size
    )
}

fn get_content_str(uri: &Uri) -> &str {
    &INDEX_STR[..get_requested_size(uri)]
}

fn get_content_bytes(uri: &Uri) -> &'static [u8] {
    &INDEX[..get_requested_size(uri)]
}

#[derive(Serialize)]
struct TestResponse<'a> {
    message: &'a str,
}
