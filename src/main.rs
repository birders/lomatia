mod server_administration;
// mod session_management;
mod user_data;

use futures::future;
use hyper::rt::Future;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::json;
use std::borrow::Cow;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;

type EndpointFutureBox = Box<dyn Future<Item = Response<Body>, Error = Error> + Send>;
type DbPool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

mod error_code {
    pub const CHAT_LOMATIA_INVALID_PARAM: &str = "CHAT_LOMATIA_INVALID_PARAM";
    pub const CHAT_LOMATIA_INTERNAL_ERROR: &str = "CHAT_LOMATIA_INTERNAL_ERROR";
}

#[derive(Debug)]
pub struct ErrorBody {
    pub errcode: &'static str,
    pub error: Cow<'static, str>,
}
impl ErrorBody {
    const UNRECOGNIZED: ErrorBody = ErrorBody::new_static("M_UNRECOGNIZED", "Unrecognized request");
    const NOT_JSON: ErrorBody = ErrorBody::new_static("M_NOT_JSON", "Content not JSON");
    const BAD_JSON: ErrorBody = ErrorBody::new_static("M_BAD_JSON", "Invalid JSON body");
    const GUEST_ACCESS_FORBIDDEN: ErrorBody =
        ErrorBody::new_static("M_GUEST_ACCESS_FORBIDDEN", "Guest accounts are forbidden");
    const INVALID_USERNAME: ErrorBody = ErrorBody::new_static(
        "M_INVALID_USERNAME",
        "The desired user ID is not a valid user name",
    );
    const INTERNAL_ERROR: ErrorBody = ErrorBody::new_static(
        error_code::CHAT_LOMATIA_INTERNAL_ERROR,
        "Internal server error",
    );

    pub const fn new_static(errcode: &'static str, error: &'static str) -> ErrorBody {
        ErrorBody {
            errcode,
            error: Cow::Borrowed(error),
        }
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
impl ToString for ErrorBody {
    fn to_string(&self) -> String {
        json!({
            "errcode": self.errcode,
            "error": self.error
        })
        .to_string()
    }
}

#[derive(Debug)]
pub enum Error {
    Bcrypt(bcrypt::BcryptError),
    DB(tokio_postgres::Error),
    DBPool(bb8::RunError<tokio_postgres::Error>),
    CanceledFuture,
    Hyper(hyper::Error),
    UserFacing(ErrorBody),
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

impl From<bb8::RunError<tokio_postgres::Error>> for Error {
    fn from(err: bb8::RunError<tokio_postgres::Error>) -> Error {
        Error::DBPool(err)
    }
}

impl From<ErrorBody> for Error {
    fn from(err: ErrorBody) -> Error {
        Error::UserFacing(err)
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Error {
        Error::Hyper(err)
    }
}

impl From<bcrypt::BcryptError> for Error {
    fn from(err: bcrypt::BcryptError) -> Error {
        Error::Bcrypt(err)
    }
}

const APPLICATION_JSON: &'static str = "application/json";

fn tack_on<T, E, A>(res: Result<T, E>, addition: A) -> Result<(T, A), (E, A)> {
    match res {
        Ok(value) => Ok((value, addition)),
        Err(err) => Err((err, addition)),
    }
}

pub struct LMServer {
    cpupool: Arc<futures_cpupool::CpuPool>,
    db_pool: DbPool,
    hostname: Arc<String>,
}

impl Service for LMServer {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        Box::new(
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/_matrix/client/versions") => server_administration::versions(),
                (&Method::POST, "/_matrix/client/r0/register") => user_data::register(self, req),
                // (&Method::GET, "/_matrix/client/r0/login") => session_management::login_opts(),
                // (&Method::POST, "/_matrix/client/r0/login") => session_management::login(self, req),
                _ => {
                    let mut response =
                        Response::new(Body::from(ErrorBody::UNRECOGNIZED.to_string()));
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    response.headers_mut().insert(
                        hyper::header::CONTENT_TYPE,
                        hyper::header::HeaderValue::from_static(APPLICATION_JSON),
                    );
                    Box::new(future::ok(response))
                }
            }
            .or_else(|err| {
                if let Error::UserFacing(err) = err {
                    Ok(err.to_response())
                } else {
                    eprintln!("{:?}", err);
                    Ok(ErrorBody::INTERNAL_ERROR.to_response())
                }
            }),
        )
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
                .required(true),
        )
        .get_matches();

    let ip_address = IpAddr::from_str(matches.value_of("address").unwrap()).unwrap();
    let port = matches.value_of("port").unwrap().parse::<u16>().unwrap();
    let socket_addr = SocketAddr::new(ip_address, port);
    let cpupool = Arc::new(futures_cpupool::Builder::new().create());
    let db_params = matches.value_of("database-url").unwrap().to_owned();
    let hostname = Arc::new(socket_addr.to_string().to_owned());

    tokio::run(
        futures::future::lazy(move || {
            bb8::Pool::builder()
                .build(bb8_postgres::PostgresConnectionManager::new(
                    db_params,
                    tokio_postgres::NoTls,
                ))
                .map_err(|err| panic!("Failed to connect to database: {:?}", err))
                .and_then(move |db_pool| {
                    println!("Listening on http://{}...", socket_addr);

                    Server::bind(&socket_addr.to_owned()).serve(
                        move || -> future::FutureResult<LMServer, hyper::Error> {
                            future::ok(LMServer {
                                cpupool: cpupool.clone(),
                                db_pool: db_pool.clone(),
                                hostname: hostname.clone(),
                            })
                        },
                    )
                })
        })
        .map_err(|err| panic!("Server encountered a runtime error: {:?}", err)),
    );
}
