extern crate hyper;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let new_svc = || service_fn_ok(hello_world);

    let server = Server::bind(&addr)
        .serve(new_svc)
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}

fn hello_world(_req: Request<Body>) -> Response<Body> {
    Response::new(Body::from("Hello world!"))
}
