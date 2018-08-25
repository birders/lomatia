extern crate futures;
extern crate hyper;
#[macro_use]
extern crate serde_json;
extern crate futures_cpupool;
extern crate qstring;
#[macro_use]
extern crate lazy_static;
extern crate bcrypt;
extern crate clap;
extern crate regex;
extern crate tokio_core;
extern crate tokio_postgres;
extern crate uuid;

mod server_administration;
// mod session_management;
mod user_data;

use futures::future;
use hyper::rt::Future;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::str::FromStr;
use std::sync::Arc;

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

mod error_code {
    pub const CHAT_LOMATIA_INVALID_PARAM:  &str = "CHAT_LOMATIA_INVALID_PARAM";
    pub const CHAT_LOMATIA_INTERNAL_ERROR: &str = "CHAT_LOMATIA_INTERNAL_ERROR";
}

struct ErrorBody<'a> {
    pub errcode: &'static str,
    pub error: &'a str,
}
impl<'a> ErrorBody<'a> {
    const UNRECOGNIZED: ErrorBody<'static> = ErrorBody {
        errcode: "M_UNRECOGNIZED",
        error: "Unrecognized request",
    };
    const NOT_JSON: ErrorBody<'static> = ErrorBody {
        errcode: "M_NOT_JSON",
        error: "Content not JSON",
    };
    const BAD_JSON: ErrorBody<'static> = ErrorBody {
        errcode: "M_BAD_JSON",
        error: "Invalid JSON body",
    };
    const GUEST_ACCESS_FORBIDDEN: ErrorBody<'static> = ErrorBody {
        errcode: "M_GUEST_ACCESS_FORBIDDEN",
        error: "Guest accounts are forbidden",
    };
    const INVALID_USERNAME: ErrorBody<'static> = ErrorBody {
        errcode: "M_INVALID_USERNAME",
        error: "The desired user ID is not a valid user name",
    };
    const INTERNAL_ERROR: ErrorBody<'static> = ErrorBody {
        errcode: error_code::CHAT_LOMATIA_INTERNAL_ERROR,
        error: "Internal server error",
    };

    pub fn new<'b>(errcode: &'static str, error: &'b str) -> ErrorBody<'b> {
        ErrorBody { errcode, error }
    }
    pub fn to_response(&self) -> Response<Body> {
        let mut resp = Response::new(Body::from(self.to_string()));
        *resp.status_mut() = match self.errcode {
            error_code::CHAT_LOMATIA_INTERNAL_ERROR => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        };
        resp.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_static(APPLICATION_JSON),
        );

        resp
    }
}
impl<'a> ToString for ErrorBody<'a> {
    fn to_string(&self) -> String {
        json!({
            "errcode": self.errcode,
            "error": self.error
        }).to_string()
    }
}

#[derive(Debug)]
enum Error {
    DB(tokio_postgres::Error),
    CanceledFuture,
}

impl From<futures::Canceled> for Error {
    fn from(_e: futures::Canceled) -> Error {
        Error::CanceledFuture
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(err: tokio_postgres::Error) -> Error {
        Error::DB(err)
    }
}

fn run_on_main<R, E: From<futures::Canceled>, F: 'static + Future<Item = R, Error = E> + Send>(
    remote: &tokio_core::reactor::Remote,
    f: impl FnOnce(&tokio_core::reactor::Handle) -> F + Send + 'static,
) -> Box<Future<Item = R, Error = E> + Send> {
    match remote.handle() {
        Some(handle) => Box::new(f(&handle)),
        None => {
            let (tx, rx) = futures::sync::oneshot::channel::<F>();
            remote.spawn(move |handle| {
                tx.send(f(handle)).ok();
                Ok(())
            });
            Box::new(rx.flatten())
        }
    }
}

const APPLICATION_JSON: &'static str = "application/json";

pub struct LMServer {
    cpupool: Arc<futures_cpupool::CpuPool>,
    db_params: tokio_postgres::params::ConnectParams,
    remote: tokio_core::reactor::Remote,
    hostname: Arc<String>,
}

impl Service for LMServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = BoxFut;

    fn call(&mut self, req: Request<Body>) -> BoxFut {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/_matrix/client/versions") => server_administration::versions(),
            (&Method::POST, "/_matrix/client/r0/register") => user_data::register(self, req),
            // (&Method::POST, "/_matrix/client/r0/login") => session_management::login(self, req),
            _ => {
                let mut response = Response::new(Body::from(ErrorBody::UNRECOGNIZED.to_string()));
                *response.status_mut() = StatusCode::BAD_REQUEST;
                response.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    hyper::header::HeaderValue::from_static(APPLICATION_JSON),
                );
                Box::new(future::ok(response))
            }
        }
    }
}

fn main() {
    let matches = clap::App::new("Lomatia")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A Matrix homeserver written in Rust")
        .arg(
            clap::Arg::with_name("address")
                .short("a")
                .long("address")
                .help("Sets the IP address used by the server")
                .takes_value(true)
                .default_value("127.0.0.1"),
        )
        .arg(
            clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Sets the port used by the server")
                .takes_value(true)
                .default_value("8448"),
        )
        .arg(
            clap::Arg::with_name("database-url")
                .long("database-url")
                .help("Sets the URL to the Postgres database")
                .takes_value(true)
                .env("DATABASE_URL")
                .required(true)
        )
        .get_matches();

    let ip_address = std::net::IpAddr::from_str(matches.value_of("address").unwrap()).unwrap();
    let port = matches.value_of("port").unwrap().parse::<u16>().unwrap();

    let address = (ip_address, port).into();

    let mut core = tokio_core::reactor::Core::new().unwrap();

    let cpupool = Arc::new(futures_cpupool::Builder::new().create());
    let db_params = tokio_postgres::params::IntoConnectParams::into_connect_params(
        matches.value_of("database-url").unwrap()
    ).unwrap();
    let hostname = Arc::new("localhost:8448".to_owned()); // TODO: adjust this
    let remote = core.remote();

    let server = Server::bind(&address)
        .serve(move || -> future::FutureResult<LMServer, hyper::Error> {
            future::ok(LMServer {
                cpupool: cpupool.clone(),
                db_params: db_params.clone(),
                remote: remote.clone(),
                hostname: hostname.clone(),
            })
        })
        .map_err(|e| eprintln!("Server error: {}", e));

    println!("Listening on http://{}...", address);
    core.run(server)
        .expect("Server encountered a runtime error");
}
