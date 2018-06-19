use bcrypt;
use serde_json;
use hyper;
use qstring;

use hyper::{Body, Request, Response, StatusCode};
use futures::{future, Future, Stream};
use regex::Regex;

use {error_code, BoxFut, ErrorBody, LMServer, APPLICATION_JSON};

lazy_static! {
    static ref VALID_USERNAME_RE: Regex = Regex::new("^[a-z-.=_/0-9]+$").unwrap();
}

pub fn register(server: &LMServer, req: Request<Body>) -> BoxFut {
    let cpupool = server.cpupool.clone();
    let query = qstring::QString::from(req.uri().query().unwrap_or(""));
    Box::new(req.into_body().concat2()
             .and_then(move |body| -> BoxFut {
                 let body: Result<serde_json::Value, serde_json::Error> = serde_json::from_slice(&body);
                 if let Err(err) = body {
                     return match err.classify() {
                         serde_json::error::Category::Syntax | serde_json::error::Category::Eof | serde_json::error::Category::Io => {
                             Box::new(future::ok(ErrorBody::NOT_JSON.to_response()))
                         },
                         serde_json::error::Category::Data => Box::new(future::ok(ErrorBody::BAD_JSON.to_response())),
                     }
                 }
                 let body = body.unwrap(); // errors are handled above, so unwrap should be okay

                 if let Some(auth) = body["auth"].as_object() {
                     match query.get("kind") {
                         Some("guest") => Box::new(future::ok(ErrorBody::GUEST_ACCESS_FORBIDDEN.to_response())),
                         Some("user") => {
                             let username = body["username"].as_str().unwrap_or("");
                             if !VALID_USERNAME_RE.is_match(&username) {
                                 return Box::new(future::ok(ErrorBody::INVALID_USERNAME.to_response()));
                             }
                             let password = body["password"].to_string();
                             Box::new(cpupool.spawn_fn(move || {
                                 bcrypt::hash(&password, bcrypt::DEFAULT_COST)
                             })
                                      .and_then(|hash| {
                                          println!("{:?}", hash);
                                          Ok(ErrorBody::INTERNAL_ERROR.to_response())
                                      })
                                      .or_else(|err| {
                                          eprintln!("{:?}", err);
                                          Ok(ErrorBody::INTERNAL_ERROR.to_response())
                                      }))
                         },
                         _ => Box::new(future::ok(ErrorBody::new(error_code::CHAT_LOMATIA_INVALID_PARAM, &format!("Invalid 'kind' value, must be either 'guest' or 'user'")).to_response()))
                     }
                 }
                 else {
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
                 }
             }))
}
