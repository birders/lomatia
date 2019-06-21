use bcrypt;
use hyper;
use serde_json;
use uuid;

use futures::{future, Future, IntoFuture, Stream};
use hyper::{Body, Request, Response};
use serde_derive::Deserialize;

use crate::user_data::{create_access_token, generate_device_id};
use crate::{error_code, tack_on, EndpointFutureBox, ErrorBody, LMServer, APPLICATION_JSON};

#[derive(Deserialize)]
struct LoginReqBody {
    #[serde(rename = "type")]
    type_: String,
    user: Option<String>,
    medium: Option<String>,
    // address: Option<String>,
    password: Option<String>,
    // token: Option<String>,
    device_id: Option<String>,
    // initial_device_display_name: Option<String>,
}

const INVALID_PASSWORD: ErrorBody =
    ErrorBody::new_static(error_code::M_FORBIDDEN, "Invalid password");

pub fn login(server: &LMServer, req: Request<Body>) -> EndpointFutureBox {
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

    let cpupool = server.cpupool.clone();
    let db_pool = server.db_pool.clone();
    let hostname = server.hostname.clone();
    Box::new(
        req.into_body()
            .concat2()
            .map_err(crate::Error::from)
            .and_then(move |body| {
                serde_json::from_slice(&body).map_err(|err| {
                    match err.classify() {
                        serde_json::error::Category::Syntax
                        | serde_json::error::Category::Eof
                        | serde_json::error::Category::Io => ErrorBody::NOT_JSON,
                        serde_json::error::Category::Data => ErrorBody::BAD_JSON,
                    }
                    .into()
                })
            })
            .and_then(move |body: LoginReqBody| -> EndpointFutureBox {
                if body.type_ == "m.login.password" {
                    if body.medium.is_some() {
                        return Box::new(future::err(
                            ErrorBody::new_static(error_code::M_UNKNOWN, "3pid is not supported")
                                .into(),
                        ));
                    }

                    let req_device_id = body.device_id;

                    Box::new(
                        body.user
                            .ok_or(ErrorBody::new_static(
                                error_code::M_UNKNOWN,
                                "Missing user parameter",
                            ))
                            .into_future()
                            .join(
                                body.password
                                    .ok_or(ErrorBody::new_static(
                                        error_code::M_UNKNOWN,
                                        "Missing password parameter",
                                    ))
                                    .into_future(),
                            )
                            .map_err(crate::Error::from)
                            .and_then(move |(username, password)| {
                                db_pool
                                    .run(|mut db| {
                                        db.prepare(
                                            "SELECT id, passhash FROM users WHERE localpart=$1",
                                        )
                                        .then(|res| tack_on(res, db))
                                        .and_then(
                                            move |(q, mut db)| {
                                                db.query(&q, &[&username])
                                                    .into_future()
                                                    .map(|(row, _)| (row, username))
                                                    .map_err(|(err, _)| err)
                                                    .then(|res| tack_on(res, db))
                                            },
                                        )
                                    })
                                    .map_err(crate::Error::from)
                                    .and_then(|(row, username)| {
                                        Ok((row.ok_or(INVALID_PASSWORD)?, username))
                                    })
                                    .and_then(move |(row, username)| {
                                        let user_id = row.get(0);
                                        let passhash: String = row.get(1);

                                        cpupool
                                            .spawn_fn(move || bcrypt::verify(password, &passhash))
                                            .map_err(crate::Error::from)
                                            .and_then(move |correct| {
                                                if correct {
                                                    Ok((user_id, username))
                                                } else {
                                                    Err(INVALID_PASSWORD.into())
                                                }
                                            })
                                    })
                                    .and_then(move |(user_id, username): (uuid::Uuid, String)| {
                                        let device_id =
                                            req_device_id.unwrap_or_else(generate_device_id);
                                        db_pool
                                            .run({
                                                let device_id = device_id.clone();
                                                move |db| {
                                                    create_access_token(db, user_id, device_id)
                                                }
                                            })
                                            .map_err(crate::Error::from)
                                            .map(move |token| {
                                                let mut resp = Response::new(
                                                    serde_json::json!({
                                                        "user_id": username,
                                                        "access_token": token,
                                                        "device_id": device_id,
                                                        "home_server": *hostname,
                                                    })
                                                    .to_string()
                                                    .into(),
                                                );

                                                resp.headers_mut().insert(
                                                    hyper::header::CONTENT_TYPE,
                                                    hyper::header::HeaderValue::from_static(
                                                        APPLICATION_JSON,
                                                    ),
                                                );

                                                resp
                                            })
                                    })
                            }),
                    )
                } else {
                    Box::new(future::err(
                        ErrorBody::new_static(error_code::M_UNKNOWN, "Unknown login type").into(),
                    ))
                }
            }),
    )
}
