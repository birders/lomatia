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

lazy_static! {
    static ref VALID_USERNAME_RE: Regex = Regex::new("^[a-z-.=_/0-9]+$").unwrap();
}

const REGISTER_QUERY: &'static str =
    "INSERT INTO users (id, localpart, passhash) VALUES ($1, $2, $3)";

pub fn register(server: &LMServer, req: Request<Body>) -> BoxFut {
    let cpupool = server.cpupool.clone();
    let db_params = server.db_params.clone();
    let remote = server.remote.clone();

    let query = qstring::QString::from(req.uri().query().unwrap_or(""));
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
        }
        let body = body.unwrap(); // errors are handled above, so unwrap should be okay

        if let Some(auth) = body["auth"].as_object() {
            match query.get("kind") {
                Some("guest") => {
                    Box::new(future::ok(ErrorBody::GUEST_ACCESS_FORBIDDEN.to_response()))
                }
                Some("user") => {
                    let username = body["username"].as_str().unwrap_or("").to_owned();
                    if !VALID_USERNAME_RE.is_match(&username) {
                        return Box::new(future::ok(ErrorBody::INVALID_USERNAME.to_response()));
                    }
                    let password = body["password"].to_string();
                    println!("hashing password");
                    Box::new(
                        cpupool
                            .spawn_fn(move || {
                                println!("hashing...");
                                bcrypt::hash(&password, bcrypt::DEFAULT_COST)
                            })
                            .then(move |hash_res| -> BoxFut {
                                match hash_res {
                                    Ok(hash) => {
                                        println!("{:?}", hash);
                                        Box::new(
                                            run_on_main(&remote, |handle| {
                                                tokio_postgres::Connection::connect(
                                                    db_params,
                                                    tokio_postgres::TlsMode::None,
                                                    &handle,
                                                ).and_then(|db| db.prepare(REGISTER_QUERY))
                                                    .and_then(|(q, db)| {
                                                        let id = uuid::Uuid::new_v4();
                                                        let values: Vec<&tokio_postgres::types::ToSql> = vec![&id, &username, &hash];
                                                        db.query(&q, &values).and_then(|_| id)
                                                    })
                                                    .map_err(|(e, db)| e)
                                                    .map_err(::Error::from)
                                            }).then(
                                                |res| {
                                                    println!("{:?}", res);
                                                    Ok(ErrorBody::INTERNAL_ERROR.to_response())
                                                },
                                            ),
                                        )
                                    }
                                    Err(err) => {
                                        eprintln!("{:?}", err);
                                        Box::new(future::ok(
                                            ErrorBody::INTERNAL_ERROR.to_response(),
                                        ))
                                    }
                                }
                            }),
                    )
                }
                _ => Box::new(future::ok(
                    ErrorBody::new(
                        error_code::CHAT_LOMATIA_INVALID_PARAM,
                        &format!("Invalid 'kind' value, must be either 'guest' or 'user'"),
                    ).to_response(),
                )),
            }
        } else {
            let session_id = "_session_id"; // TODO randomly generate this
            let mut resp = Response::new(Body::from(
                json!({
                         "flows": [
                         {
                             "stages": [
                                 "m.login.dummy"
                             ]
                         }
                         ],
                         "session": session_id
                     }).to_string(),
            ));
            *resp.status_mut() = StatusCode::UNAUTHORIZED;
            resp.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                hyper::header::HeaderValue::from_static(APPLICATION_JSON),
            );
            Box::new(future::ok(resp))
        }
    }))
}
