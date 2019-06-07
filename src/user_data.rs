use futures::{future, Future, Stream};
use hyper::{Body, Request, Response, StatusCode};
use regex::Regex;
use serde_json::json;

use crate::{error_code, tack_on, EndpointFutureBox, ErrorBody, LMServer, APPLICATION_JSON};

const REGISTER_QUERY: &'static str =
    "INSERT INTO users (id, localpart, passhash) VALUES ($1, $2, $3)";

const NEW_TOKEN_QUERY: &'static str =
    "INSERT INTO tokens (id, user_id, created, device_id) VALUES ($1, $2, localtimestamp, $3)";

fn generate_access_token() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

fn generate_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn create_access_token(
    mut db: tokio_postgres::Client,
    user_id: uuid::Uuid,
    device_id: String,
) -> impl Future<
    Item = (String, tokio_postgres::Client),
    Error = (tokio_postgres::Error, tokio_postgres::Client),
> {
    let token = generate_access_token();
    db.prepare(NEW_TOKEN_QUERY)
        .then(|res| tack_on(res, db))
        .and_then(
            move |(q, mut db): (tokio_postgres::Statement, tokio_postgres::Client)| {
                db.execute(&q, &[&token, &user_id, &device_id])
                    .and_then(move |_| Ok(token.to_string()))
                    .then(|res| tack_on(res, db))
            },
        )
}

pub fn register(server: &LMServer, req: Request<Body>) -> EndpointFutureBox {
    let cpupool = server.cpupool.clone();
    let db_pool = server.db_pool.clone();
    let hostname = server.hostname.clone();

    let query = qstring::QString::from(req.uri().query().unwrap_or(""));
    // TODO: Move this into a helper function as it is used elsewhere
    Box::new(req.into_body().concat2().map_err(crate::Error::from).and_then(move |body| {
        serde_json::from_slice(&body)
            .map_err(|err| {
            match err.classify() {
                serde_json::error::Category::Syntax
                | serde_json::error::Category::Eof
                | serde_json::error::Category::Io => {
                    ErrorBody::NOT_JSON
                }
                serde_json::error::Category::Data => {
                    ErrorBody::BAD_JSON
                }
            }.into()
        })
    })
             .and_then(move |body: serde_json::Value| -> EndpointFutureBox {
                 if let Some(_auth) = body["auth"].as_object() {
                     match query.get("kind").unwrap_or("user") {
                         "guest" => Box::new(future::err(ErrorBody::GUEST_ACCESS_FORBIDDEN.into())),
                         "user" => {
                             let username = body["username"].as_str().unwrap_or("").to_owned();
                             let is_valid_username: Regex = Regex::new("^[a-z-.=_/0-9]+$").unwrap();
                             if !is_valid_username.is_match(&username) {
                                 return Box::new(future::err(ErrorBody::INVALID_USERNAME.into()));
                             }
                             let password = body["password"].to_string();
                             let req_device_id = body["device_id"].as_str().map(|x| x.to_owned());
                             println!("Hashing password...");
                             Box::new(
                                 cpupool
                                 .spawn_fn(move || bcrypt::hash(&password, bcrypt::DEFAULT_COST))
                                 .map_err(crate::Error::from)
                                 .and_then(move |hash| -> EndpointFutureBox {
                                     println!("{:?}", hash);
                                     Box::new(
                                         db_pool.run(|mut db| {
                                             db.prepare(REGISTER_QUERY)
                                                 .then(|res| tack_on(res, db))
                                                 .and_then(move |(q, mut db)| {
                                                     let id = uuid::Uuid::new_v4();
                                                     {
                                                         let values: Vec<&dyn tokio_postgres::types::ToSql> = vec![&id, &username, &hash];
                                                         db.execute(&q, &values)
                                                     }.and_then(move |_| Ok((id, username)))
                                                     .then(|res| tack_on(res, db))
                                                 })
                                             .and_then(
                                                 move |((user_id, username), db)| {
                                                     let device_id = req_device_id
                                                         .unwrap_or_else(|| {
                                                             generate_device_id()
                                                         });
                                                     create_access_token(db, user_id.clone(), device_id.clone())
                                                         .and_then(|(token, db)| Ok(((token, device_id, username), db)))
                                                 },
                                                 )
                                         })
                                     .map_err(crate::Error::from)
                                         .and_then(move |(token, device_id, username)| {
                                             Ok(Response::new(Body::from(
                                                         json!({
                                                             "user_id": username,
                                                             "access_token": token,
                                                             "device_id": device_id,
                                                             "home_server": *hostname
                                                         }).to_string(),
                                                         )))
                                         }))
                                 })
                                 )
                         }
                         _ => {
                             Box::new(future::err(
                                     ErrorBody::new_static(
                                         error_code::CHAT_LOMATIA_INVALID_PARAM,
                                         "Invalid 'kind' value, must be either 'guest' or 'user'",
                                         ).into()))
                         }
                     }
                 } else {
                     let session_id = "_session_id"; // TODO: Generate randomly
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
