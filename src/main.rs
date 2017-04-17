extern crate futures;
extern crate tokio_proto;
extern crate tokio_service;
extern crate hyper;
extern crate native_tls;
extern crate tokio_tls;
extern crate num_cpus;
#[macro_use]
extern crate mime;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate rustls;
extern crate tokio_rustls;
extern crate url;

use tokio_proto::TcpServer;
use futures::{future, Future, Stream};
use tokio_service::Service;
use hyper::server::{Http, Request, Response};
use hyper::Method::{Get, Post};
use hyper::header::{ContentLength, ContentType, Server};
use hyper::status::StatusCode::NotFound;
use std::net::SocketAddr;
use native_tls::{TlsAcceptor, Pkcs12};
use std::io::{Read, BufReader};
use std::sync::Arc;
use std::fs::File;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use url::form_urlencoded;
use std::ascii::AsciiExt;

const INDEX_STR: &str = "Hello, world!Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam venenatis odio leo, vehicula scelerisque ipsum sollicitudin ut. Sed sit amet lobortis quam, eget congue ligula. Nam vitae nulla nisl. Aliquam facilisis eros vel dui scelerisque dictum. Pellentesque euismod sit amet leo ac laoreet. Maecenas vel congue dui. Vestibulum tempus odio eu tempus ultrices. Ut ullamcorper euismod est. Suspendisse potenti. Curabitur malesuada mi ac erat elementum fermentum. Sed a gravida tortor, sit amet volutpat eros. Pellentesque malesuada eu turpis vel tempor. Vivamus ante ipsum, tincidunt quis sagittis sed, elementum sit amet sapien.Curabitur eleifend volutpat neque vitae venenatis. Maecenas laoreet maximus congue. Vestibulum luctus, odio quis imperdiet viverra, lacus tortor eleifend massa, eget dictum est augue et nisl. Integer neque dolor, fringilla sed neque nec, sodales imperdiet justo. Suspendisse bibendum hendrerit elit, eleifend pharetra arcu lobortis id. Donec elementum elit in convallis gravida. In velit felis, ornare sit amet quam luctus, pharetra fringilla eros. Mauris volutpat a urna eu maximus. Nunc mollis dapibus sem vitae venenatis. Mauris non varius magna, ut lacinia tellus.In hac habitasse platea dictumst. Aliquam vel ligula at massa fermentum cursus tincidunt et nulla. Mauris quis venenatis est. Ut odio ex, tempor facilisis porttitor in, ultricies at libero. Morbi a lacinia erat, ut ornare lacus. Nulla volutpat elementum pulvinar. Etiam pulvinar, ligula sed iaculis porttitor, est turpis semper nisi, hendrerit pharetra nisl nisi lobortis ante. Praesent vel suscipit massa, sed aliquet orci. Ut sit amet tellus eget velit iaculis gravida sed nec dui. Aliquam eget elit ac felis venenatis interdum vel sit amet dolor. Nullam accumsan erat nisi, sed vulputate libero vestibulum a. Pellentesque eu nisl ac tortor vehicula pulvinar at non massa. Donec dui lectus, mollis id gravida vitae, molestie vitae lorem.Nam sed hendrerit odio. Praesent mollis blandit diam a scelerisque. Aliquam ornare non lorem sed consectetur. Aliquam vel eros vehicula, malesuada ex sed, euismod odio. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam eu velit laoreet, accumsan risus vitae, luctus dui. Donec a felis nisi. Vestibulum dignissim consequat leo, a pretium risus pellentesque id. Maecenas lacinia pretium ligula. Duis id magna et felis posuere aliquet. Integer nibh velit, gravida eget erat non, aliquam dignissim enim. Suspendisse potenti.Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Integer dictum velit quis neque sodales lacinia. Nunc molestie id leo convallis tempor. Maecenas vel facilisis magna. Vestibulum eleifend vel nisl tristique lobortis. Fusce eu ipsum urna. Praesent in vulputate urna. Ut ultrices magna et mollis finibus. Fusce mollis dignissim posuere.Sed sagittis hendrerit nunc, vitae condimentum velit ultrices eget. Donec vitae mi non mauris egestas eleifend quis vel arcu. Suspendisse at consectetur urna, sed luctus ipsum. Maecenas libero tortor, dignissim eget eleifend id, ullamcorper et lorem. Etiam porta magna eu tempor venenatis. Integer tempor ante eu risus cursus, ac commodo lacus pharetra. Nulla tincidunt dui risus, a fermentum mauris dapibus et. Nam sit amet purus eget leo sodales imperdiet.Sed a erat ex. Nam condimentum dolor ac nibh rhoncus finibus non a felis. Curabitur eu interdum dui, ut blandit turpis. Donec interdum egestas sem in ultricies. Nulla facilisi. Fusce nec efficitur sem. Cras a eros eget magna consectetur sodales et nec arcu. Aenean maximus sagittis velit, et ultricies orci aliquet ut. Nullam in eros non ex placerat ultrices. Maecenas pellentesque semper urna, a placerat urna rutrum sed. Donec et enim eget mauris finibus laoreet.Vestibulum congue, justo quis pulvinar condimentum, metus ex tristique libero, nec scelerisque dui mauris sit amet dolor. Mauris eleifend turpis nisl, in ullamcorper eros sollicitudin in. Nullam efficitur auctor gravida. Cras ut tempor arcu, sit amet blandit tellus. Proin at neque egestas, consequat erat dignissim, facilisis nulla. Suspendisse blandit odio ut lorem pellentesque, vitae efficitur mi fermentum. Morbi suscipit lobortis nisl, vitae ultricies elit tempor vel. Nullam suscipit nunc a massa fringilla dapibus. Morbi placerat ex sed arcu elementum, quis blandit elit cursus. Fusce orci lorem, rhoncus vel quam a, sagittis imperdiet dolor. Nunc vitae rutrum massa. Nulla nec turpis pretium, blandit velit eu, sollicitudin elit. Vivamus ultrices dapibus massa, vel condimentum diam aliquam vel. Nunc eu gravida nulla.Nullam vulputate ullamcorper rhoncus. Curabitur gravida aliquet hendrerit. Maecenas tempus consequat dolor nec porttitor. Sed ornare convallis risus a rhoncus. Maecenas dui urna, placerat vel eleifend sed, vestibulum non quam. Donec rutrum nibh lorem, quis sodales ante finibus vel. Nunc sollicitudin magna elit, a rutrum ipsum pulvinar ut. Sed non sapien sit amet metus laoreet cursus ut a purus. Vestibulum at leo nec justo gravida ullamcorper. Duis lobortis arcu risus, placerat pretium dolor blandit in. Vestibulum vulputate nunc et metus lacinia hendrerit. Nullam arcu quam, dictum sed varius at, lobortis id lorem. Aliquam erat volutpat. Donec at lorem nisi. Suspendisse eget porta lacus.Quisque pulvinar turpis dictum, pellentesque nisl et, porta orci. Proin consequat fermentum nibh vitae vestibulum. Aliquam ac enim tortor. Morbi porta dui placerat, dapibus augue nec, fringilla felis. Sed imperdiet placerat dui nec dapibus. Donec mollis, elit ac volutpat varius, sem felis placerat ex, venenatis porttitor nunc nunc a nulla. Quisque imperdiet diam leo, bibendum malesuada arcu imperdiet sit amet. Sed eget luctus augue, a dapibus quam. Aliquam erat volutpat. Aenean imperdiet, felis non sagittis vehicula, erat ligula pharetra sem, sit amet pharetra eros leo vel sem. Pellentesque in iaculis risus. Integer nisl lacus, sodales at accumsan non, mollis ac dui.Curabitur luctus neque vitae justo blandit porttitor. Suspendisse at purus vitae felis egestas facilisis nec sagittis nulla. Suspendisse sodales, risus non gravida cursus, lacus elit tempor tellus, in maximus metus neque ac elit. Nam auctor egestas ornare. Nulla at viverra augue. Sed consequat turpis orci, non ullamcorper ligula molestie ut. Pellentesque lacinia, mi sit amet ornare hendrerit, nisi orci elementum magna, non tempus erat massa non mi. Proin euismod ex ac pulvinar consequat. Duis consectetur porta diam vel hendrerit.Etiam tincidunt massa vel turpis placerat volutpat. Nullam pellentesque ligula et enim tincidunt, at lobortis arcu sollicitudin. Nulla blandit nulla ipsum, a fermentum nisi pellentesque vitae. Sed sagittis laoreet posuere. Aenean at libero non est dignissim aliquam ut eu turpis. Mauris vehicula diam a turpis aliquam elementum. Nam congue fringilla eros, non elementum eros aliquam ac. Proin sodales condimentum nibh et auctor. Mauris orci ante, volutpat rhoncus pellentesque in, finibus eu erat. Nunc ac lacinia dolor. Maecenas orci enim, consequat in elementum eget, tempus ac nisl. Nullam consectetur leo lectus. Cras commodo ornare sem in scelerisque.Curabitur porttitor blandit dolor nec mollis. Duis a eros enim. Nulla facilisi. Vivamus faucibus mauris vel nulla interdum, vel congue velit vestibulum. Duis finibus nulla aliquet, faucibus velit eleifend, auctor massa. Proin neque risus, aliquam efficitur ante ac, blandit semper sem. Sed eleifend mauris nunc, at placerat ex tempus quis. Cras vulputate eros sed viverra porta.Curabitur vitae dolor eget est mattis ornare et quis augue. Praesent vitae suscipit quam. Vivamus vehicula pellentesque turpis, id posuere sapien congue a. In hac habitasse platea dictumst. Phasellus hendrerit accumsan lacus eu sodales. Mauris tempus scelerisque felis ac pretium. Vivamus tincidunt elit eget tellus suscipit gravida. Cras eu risus fermentum velit vestibulum pharetra. Phasellus malesuada lacus nec sapien sollicitudin, sed tempus velit hendrerit. Etiam vulputate ligula at purus rutrum convallis. Vivamus tempor, est vitae mattis consequat, ipsum ligula placerat justo, et porttitor felis erat bibendum nibh. Vestibulum in purus lorem nullam.";
static INDEX: &'static [u8] = b"Hello, world!Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam venenatis odio leo, vehicula scelerisque ipsum sollicitudin ut. Sed sit amet lobortis quam, eget congue ligula. Nam vitae nulla nisl. Aliquam facilisis eros vel dui scelerisque dictum. Pellentesque euismod sit amet leo ac laoreet. Maecenas vel congue dui. Vestibulum tempus odio eu tempus ultrices. Ut ullamcorper euismod est. Suspendisse potenti. Curabitur malesuada mi ac erat elementum fermentum. Sed a gravida tortor, sit amet volutpat eros. Pellentesque malesuada eu turpis vel tempor. Vivamus ante ipsum, tincidunt quis sagittis sed, elementum sit amet sapien.Curabitur eleifend volutpat neque vitae venenatis. Maecenas laoreet maximus congue. Vestibulum luctus, odio quis imperdiet viverra, lacus tortor eleifend massa, eget dictum est augue et nisl. Integer neque dolor, fringilla sed neque nec, sodales imperdiet justo. Suspendisse bibendum hendrerit elit, eleifend pharetra arcu lobortis id. Donec elementum elit in convallis gravida. In velit felis, ornare sit amet quam luctus, pharetra fringilla eros. Mauris volutpat a urna eu maximus. Nunc mollis dapibus sem vitae venenatis. Mauris non varius magna, ut lacinia tellus.In hac habitasse platea dictumst. Aliquam vel ligula at massa fermentum cursus tincidunt et nulla. Mauris quis venenatis est. Ut odio ex, tempor facilisis porttitor in, ultricies at libero. Morbi a lacinia erat, ut ornare lacus. Nulla volutpat elementum pulvinar. Etiam pulvinar, ligula sed iaculis porttitor, est turpis semper nisi, hendrerit pharetra nisl nisi lobortis ante. Praesent vel suscipit massa, sed aliquet orci. Ut sit amet tellus eget velit iaculis gravida sed nec dui. Aliquam eget elit ac felis venenatis interdum vel sit amet dolor. Nullam accumsan erat nisi, sed vulputate libero vestibulum a. Pellentesque eu nisl ac tortor vehicula pulvinar at non massa. Donec dui lectus, mollis id gravida vitae, molestie vitae lorem.Nam sed hendrerit odio. Praesent mollis blandit diam a scelerisque. Aliquam ornare non lorem sed consectetur. Aliquam vel eros vehicula, malesuada ex sed, euismod odio. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam eu velit laoreet, accumsan risus vitae, luctus dui. Donec a felis nisi. Vestibulum dignissim consequat leo, a pretium risus pellentesque id. Maecenas lacinia pretium ligula. Duis id magna et felis posuere aliquet. Integer nibh velit, gravida eget erat non, aliquam dignissim enim. Suspendisse potenti.Class aptent taciti sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Integer dictum velit quis neque sodales lacinia. Nunc molestie id leo convallis tempor. Maecenas vel facilisis magna. Vestibulum eleifend vel nisl tristique lobortis. Fusce eu ipsum urna. Praesent in vulputate urna. Ut ultrices magna et mollis finibus. Fusce mollis dignissim posuere.Sed sagittis hendrerit nunc, vitae condimentum velit ultrices eget. Donec vitae mi non mauris egestas eleifend quis vel arcu. Suspendisse at consectetur urna, sed luctus ipsum. Maecenas libero tortor, dignissim eget eleifend id, ullamcorper et lorem. Etiam porta magna eu tempor venenatis. Integer tempor ante eu risus cursus, ac commodo lacus pharetra. Nulla tincidunt dui risus, a fermentum mauris dapibus et. Nam sit amet purus eget leo sodales imperdiet.Sed a erat ex. Nam condimentum dolor ac nibh rhoncus finibus non a felis. Curabitur eu interdum dui, ut blandit turpis. Donec interdum egestas sem in ultricies. Nulla facilisi. Fusce nec efficitur sem. Cras a eros eget magna consectetur sodales et nec arcu. Aenean maximus sagittis velit, et ultricies orci aliquet ut. Nullam in eros non ex placerat ultrices. Maecenas pellentesque semper urna, a placerat urna rutrum sed. Donec et enim eget mauris finibus laoreet.Vestibulum congue, justo quis pulvinar condimentum, metus ex tristique libero, nec scelerisque dui mauris sit amet dolor. Mauris eleifend turpis nisl, in ullamcorper eros sollicitudin in. Nullam efficitur auctor gravida. Cras ut tempor arcu, sit amet blandit tellus. Proin at neque egestas, consequat erat dignissim, facilisis nulla. Suspendisse blandit odio ut lorem pellentesque, vitae efficitur mi fermentum. Morbi suscipit lobortis nisl, vitae ultricies elit tempor vel. Nullam suscipit nunc a massa fringilla dapibus. Morbi placerat ex sed arcu elementum, quis blandit elit cursus. Fusce orci lorem, rhoncus vel quam a, sagittis imperdiet dolor. Nunc vitae rutrum massa. Nulla nec turpis pretium, blandit velit eu, sollicitudin elit. Vivamus ultrices dapibus massa, vel condimentum diam aliquam vel. Nunc eu gravida nulla.Nullam vulputate ullamcorper rhoncus. Curabitur gravida aliquet hendrerit. Maecenas tempus consequat dolor nec porttitor. Sed ornare convallis risus a rhoncus. Maecenas dui urna, placerat vel eleifend sed, vestibulum non quam. Donec rutrum nibh lorem, quis sodales ante finibus vel. Nunc sollicitudin magna elit, a rutrum ipsum pulvinar ut. Sed non sapien sit amet metus laoreet cursus ut a purus. Vestibulum at leo nec justo gravida ullamcorper. Duis lobortis arcu risus, placerat pretium dolor blandit in. Vestibulum vulputate nunc et metus lacinia hendrerit. Nullam arcu quam, dictum sed varius at, lobortis id lorem. Aliquam erat volutpat. Donec at lorem nisi. Suspendisse eget porta lacus.Quisque pulvinar turpis dictum, pellentesque nisl et, porta orci. Proin consequat fermentum nibh vitae vestibulum. Aliquam ac enim tortor. Morbi porta dui placerat, dapibus augue nec, fringilla felis. Sed imperdiet placerat dui nec dapibus. Donec mollis, elit ac volutpat varius, sem felis placerat ex, venenatis porttitor nunc nunc a nulla. Quisque imperdiet diam leo, bibendum malesuada arcu imperdiet sit amet. Sed eget luctus augue, a dapibus quam. Aliquam erat volutpat. Aenean imperdiet, felis non sagittis vehicula, erat ligula pharetra sem, sit amet pharetra eros leo vel sem. Pellentesque in iaculis risus. Integer nisl lacus, sodales at accumsan non, mollis ac dui.Curabitur luctus neque vitae justo blandit porttitor. Suspendisse at purus vitae felis egestas facilisis nec sagittis nulla. Suspendisse sodales, risus non gravida cursus, lacus elit tempor tellus, in maximus metus neque ac elit. Nam auctor egestas ornare. Nulla at viverra augue. Sed consequat turpis orci, non ullamcorper ligula molestie ut. Pellentesque lacinia, mi sit amet ornare hendrerit, nisi orci elementum magna, non tempus erat massa non mi. Proin euismod ex ac pulvinar consequat. Duis consectetur porta diam vel hendrerit.Etiam tincidunt massa vel turpis placerat volutpat. Nullam pellentesque ligula et enim tincidunt, at lobortis arcu sollicitudin. Nulla blandit nulla ipsum, a fermentum nisi pellentesque vitae. Sed sagittis laoreet posuere. Aenean at libero non est dignissim aliquam ut eu turpis. Mauris vehicula diam a turpis aliquam elementum. Nam congue fringilla eros, non elementum eros aliquam ac. Proin sodales condimentum nibh et auctor. Mauris orci ante, volutpat rhoncus pellentesque in, finibus eu erat. Nunc ac lacinia dolor. Maecenas orci enim, consequat in elementum eget, tempus ac nisl. Nullam consectetur leo lectus. Cras commodo ornare sem in scelerisque.Curabitur porttitor blandit dolor nec mollis. Duis a eros enim. Nulla facilisi. Vivamus faucibus mauris vel nulla interdum, vel congue velit vestibulum. Duis finibus nulla aliquet, faucibus velit eleifend, auctor massa. Proin neque risus, aliquam efficitur ante ac, blandit semper sem. Sed eleifend mauris nunc, at placerat ex tempus quis. Cras vulputate eros sed viverra porta.Curabitur vitae dolor eget est mattis ornare et quis augue. Praesent vitae suscipit quam. Vivamus vehicula pellentesque turpis, id posuere sapien congue a. In hac habitasse platea dictumst. Phasellus hendrerit accumsan lacus eu sodales. Mauris tempus scelerisque felis ac pretium. Vivamus tincidunt elit eget tellus suscipit gravida. Cras eu risus fermentum velit vestibulum pharetra. Phasellus malesuada lacus nec sapien sollicitudin, sed tempus velit hendrerit. Etiam vulputate ligula at purus rutrum convallis. Vivamus tempor, est vitae mattis consequat, ipsum ligula placerat justo, et porttitor felis erat bibendum nibh. Vestibulum in purus lorem nullam.";
static SERVER_NAME: &'static str = "hyper";

struct HttpServer;

impl HttpServer {
    
    fn get_requested_size(&self, req: &Request) -> usize {
        std::cmp::min(
            INDEX.len(),
            req.query()
                .and_then(|q| form_urlencoded::parse(q.as_bytes()).into_iter().find(|x| x.0.eq_ignore_ascii_case("size")))
                .and_then(|x| x.1.parse::<usize>().ok())
                .unwrap_or(13)) // Hello, world!
    }
    
    fn get_content_str(&self, req: &Request) -> &str {
        &INDEX_STR[..self.get_requested_size(req)]
    }
    
    fn get_content_bytes(&self, req: &Request) -> &[u8] {
        &INDEX[..self.get_requested_size(req)]
    }
}

impl Service for HttpServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::error::Error;
    type Future = Box<Future<Item = Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/plaintext") |
            (&Get, "/") => {
                let content = self.get_content_bytes(&req);
                future::ok(Response::new()
                               .with_header(ContentLength(content.len() as u64))
                               .with_header(ContentType(mime!(Text / Plain)))
                               .with_header(Server::new(SERVER_NAME))
                               .with_body(INDEX))
                        .boxed()
            }
            (&Get, "/json") => {
                let content = self.get_content_str(&req);
                let rep = TestResponse { message: content };
                let rep_body = serde_json::to_vec(&rep).unwrap();
                future::ok(Response::new()
                               .with_header(ContentLength(rep_body.len() as u64))
                               .with_header(ContentType(mime!(Application / Json)))
                               .with_header(Server::new(SERVER_NAME))
                               .with_body(rep_body))
                        .boxed()
            }
            (&Post, "/echo") => {
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
                    .boxed()
            }
            _ => future::ok(Response::new().with_status(NotFound)).boxed(),
        }
    }
}

#[derive(Serialize)]
struct TestResponse<'a> {
    message: &'a str,
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
                                              let tls = tokio_tls::proto::Server::new(Http::new(),
                                                                                      acceptor);
                                              let mut tcp = TcpServer::new(tls, addr);
                                              tcp.threads(num_cpus::get());
                                              tcp.serve(|| Ok(HttpServer));
                                          });

    let mut config = ServerConfig::new();
    config.set_single_cert(load_certs("end.fullchain"), load_private_key("end.rsa"));
    let arc_config = Arc::new(config);

    let addr: SocketAddr = "0.0.0.0:9443".parse().unwrap();
    let rustls_thread =
        std::thread::spawn(move || {
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

