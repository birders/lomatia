extern crate futures;
extern crate hyper;
#[macro_use]
extern crate serde_json;
extern crate futures_cpupool;

mod server_administration;
mod user_data;

use std::sync::Arc;
use futures::future;
use hyper::rt::Future;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, Server, StatusCode};

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct ErrorBody<'a> {
    pub errcode: &'static str,
    pub error: &'a str
}
impl<'a> ErrorBody<'a> {
    const UNRECOGNIZED: ErrorBody<'static> = ErrorBody {
        errcode: "M_UNRECOGNIZED",
        error: "Unrecognized request"
    };
    const NOT_JSON: ErrorBody<'static> = ErrorBody {
        errcode: "M_NOT_JSON",
        error: "Content not JSON."
    };
    const BAD_JSON: ErrorBody<'static> = ErrorBody {
        errcode: "M_BAD_JSON",
        error: "Invalid JSON body."
    };
}
impl<'a> ErrorBody<'a> {
    pub fn to_response(&self) -> Response<Body> {
        let mut resp = Response::new(Body::from(self.to_string()));
        *resp.status_mut() = StatusCode::BAD_REQUEST;
        resp.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static(APPLICATION_JSON));

        resp
    }
}
impl<'a> ToString for ErrorBody<'a> {
    fn to_string(&self) -> String {
        json!({
            "errcode": self.errcode,
            "error": self.error
        }).to_string()
    }
}

const APPLICATION_JSON: &'static str = "application/json";

struct LMServer {
    cpupool: Arc<futures_cpupool::CpuPool>
}

impl Service for LMServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = BoxFut;

    fn call(&mut self, req: Request<Body>) -> BoxFut {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/_matrix/client/versions") => {
                let mut response = Response::new(Body::from(server_administration::versions().to_string()));
                *response.status_mut() = StatusCode::OK;
                response.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static(APPLICATION_JSON));
                Box::new(future::ok(response))
            }
            (&Method::POST, "/_matrix/client/r0/register") => {
                user_data::register(req)
            }
            _ => {
                let mut response = Response::new(Body::from(ErrorBody::UNRECOGNIZED.to_string()));
                *response.status_mut() = StatusCode::BAD_REQUEST;
                response.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static(APPLICATION_JSON));
                Box::new(future::ok(response))
            }
        }
    }
}

fn main() {
    let addr = ([127, 0, 0, 1], 8448).into();

    let cpupool = Arc::new(futures_cpupool::Builder::new().create());

    let server = Server::bind(&addr)
        .serve(move || -> future::FutureResult<LMServer, hyper::Error> {
            future::ok(LMServer {
                cpupool: cpupool.clone()
            })
        })
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}
