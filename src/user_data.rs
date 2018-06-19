use serde_json;
use hyper;

use hyper::{Body, Request, Response, StatusCode};
use futures::{future, Future, Stream};

use {BoxFut, ErrorBody, APPLICATION_JSON};

pub fn register(req: Request<Body>) -> BoxFut {
    Box::new(req.into_body().concat2()
             .and_then(|body| {
                 let body: Result<serde_json::Value, serde_json::Error> = serde_json::from_slice(&body);
                 if let Err(err) = body {
                     return match err.classify() {
                         serde_json::error::Category::Syntax | serde_json::error::Category::Eof | serde_json::error::Category::Io => {
                             Box::new(future::ok(ErrorBody::NOT_JSON.to_response()))
                         },
                         serde_json::error::Category::Data => Box::new(future::ok(ErrorBody::BAD_JSON.to_response())),
                     }
                 }
                 let body = body.unwrap(); // errors are handled above
                 let session_id = "_session_id"; // TODO randomly generate this
                 let mut resp = Response::new(Body::from(json!({
                     "flows": [
                     {
                         "stages": [
                             "m.login.dummy"
                         ]
                     }
                     ],
                     "session": session_id
                 }).to_string()));
                 *resp.status_mut() = StatusCode::UNAUTHORIZED;
                 resp.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static(APPLICATION_JSON));
                 Box::new(future::ok(resp))
             }))
}
