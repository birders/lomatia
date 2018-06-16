extern crate futures;
extern crate hyper;

mod server_administration;

use futures::future;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn handle_request(req: Request<Body>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/_matrix/client/versions") => {
            server_administration::versions();
        }
/*
 *         (&Method::GET, "/") => {
 *             *response.body_mut() = Body::from("Try POSTing data to /echo");
 *         }
 * 
 *         (&Method::POST, "/echo") => {
 *             *response.body_mut() = req.into_body();
 *         }
 * 
 *         (&Method::POST, "/echo/uppercase") => {
 *             let mapping = req.into_body().map(|chunk| {
 *                 chunk
 *                     .iter()
 *                     .map(|byte| byte.to_ascii_uppercase())
 *                     .collect::<Vec<u8>>()
 *             });
 * 
 *             *response.body_mut() = Body::wrap_stream(mapping);
 *         }
 * 
 *         (&Method::POST, "/echo/reversed") => {
 *             let reversed = req.into_body().concat2().map(move |chunk| {
 *                 let body = chunk.iter().rev().cloned().collect::<Vec<u8>>();
 *                 *response.body_mut() = Body::from(body);
 *                 response
 *             });
 * 
 *             return Box::new(reversed);
 *         }
 */

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
