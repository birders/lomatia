extern crate futures;
extern crate hyper;
#[macro_use]
extern crate serde_json;

mod server_administration;

use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct ErrorBody<'a> {
    pub errcode: &'static str,
    pub error: &'a str,
}
impl<'a> ErrorBody<'a> {
    const UNRECOGNIZED: ErrorBody<'static> = ErrorBody {
        errcode: "M_UNRECOGNIZED",
        error: "Unrecognized request",
    };
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

fn handle_request(req: Request<Body>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/_matrix/client/versions") => {
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                hyper::header::HeaderValue::from_static(APPLICATION_JSON),
            );
            *response.body_mut() = Body::from(server_administration::versions().to_string());
        }
        _ => {
            *response.status_mut() = StatusCode::BAD_REQUEST;
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                hyper::header::HeaderValue::from_static(APPLICATION_JSON),
            );
            *response.body_mut() = Body::from(ErrorBody::UNRECOGNIZED.to_string());
        }
    };

    Box::new(future::ok(response))
}

fn main() {
    let addr = ([127, 0, 0, 1], 8448).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(handle_request))
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}
