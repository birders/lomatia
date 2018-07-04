use bcrypt;
use hyper;
use qstring;
use serde_json;
use tokio_postgres;
use uuid;

use futures::{future, Future, Stream};
use hyper::{Body, Request, Response, StatusCode};
use regex::Regex;

use {error_code, run_on_main, BoxFut, ErrorBody, LMServer, APPLICATION_JSON};

pub fn login(server: &LMServer, req: Request<Body>) -> BoxFut {
    // Request will be of the form:
    // {
    //   "type": "m.login.password",
    //   "user": "<user_id or user localpart>",
    //   "password": "<password>"
    // }
    // or:
    // {
    //   "type": "m.login.password",
    //   "medium": "<The medium of the third party identifier. Must be 'email'>",
    //   "address": "<The third party address of the user>",
    //   "password": "<password>"
    // }
    // or:
    // {
    //   "type": "m.login.token",
    //   "token": "<login token>"
    // }
    //
    // Response should be of the form:
    // {
    //   "user_id": "<user_id>",
    //   "access_token": "<access_token>",
    //   "home_server": "<hostname>",
    //   "device_id": "<device_id>"
    // }
    Box::new(req.into_body().concat2().and_then(move |body| -> BoxFut {
        let body: Result<serde_json::Value, serde_json::Error> = serde_json::from_slice(&body);
        if let Err(err) = body {
            return match err.classify() {
                serde_json::error::Category::Syntax
                    | serde_json::error::Category::Eof
                    | serde_json::error::Category::Io => {
                        Box::new(future::ok(ErrorBody::NOT_JSON.to_response()))
                    }
                serde_json::error::Category::Data => {
                    Box::new(future::ok(ErrorBody::BAD_JSON.to_response()))
                }
            };
        };
        let body = body.unwrap();
    }))
}
