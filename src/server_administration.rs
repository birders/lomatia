use futures::future;
use hyper::{Body, Response, StatusCode};
use serde_json::json;

use crate::{EndpointFutureBox, APPLICATION_JSON};

/// Returns the versions of the specification supported by the server.
pub fn versions() -> EndpointFutureBox {
    let mut resp = Response::new(Body::from(
        json!({
            "versions": [
                "r0.3.0"
            ]
        })
        .to_string(),
    ));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static(APPLICATION_JSON),
    );
    Box::new(future::ok(resp))
}

#[cfg(test)]
mod tests {
    use futures::Future;

    #[test]
    fn versions() {
        let request = crate::server_administration::versions().wait();
        request.unwrap();
    }
}
