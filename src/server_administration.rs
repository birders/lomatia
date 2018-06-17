extern crate serde_json;

/// Gets the versions of the specification supported by the server.
pub fn versions() -> serde_json::Value {
    json!({
        "versions": [
            "r0.3.0"
        ]
    })
}
