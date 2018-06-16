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

fn handle_request(req: Request<Body>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/_matrix/client/versions") => {
            *response.status_mut() = StatusCode::NOT_FOUND;
            *response.body_mut() = Body::from(server_administration::versions().to_string());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
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
