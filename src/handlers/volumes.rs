use http_body_util::Full;
use hyper::{body::Bytes, Response, StatusCode};
use serde_json::json;

pub async fn handle_volume_inspect(name: String) -> Response<Full<Bytes>> {
    // Return a minimal valid volume inspect response.
    // This satisfies clients that check for volumes before or during container runs.
    let response = json!({
        "Name": name,
        "Driver": "local",
        "Mountpoint": format!("/var/lib/docker/volumes/{}/_data", name),
        "Labels": {},
        "Scope": "local"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))
        .expect("Failed to build volume inspect response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_handle_volume_inspect() {
        let volume_name = "test-volume";
        let response = handle_volume_inspect(volume_name.to_string()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("Failed to collect response body")
            .to_bytes();
        let json: serde_json::Value =
            serde_json::from_slice(&body).expect("Failed to parse JSON response");
        assert_eq!(json["Name"], volume_name);
        assert_eq!(json["Driver"], "local");
        assert!(json["Mountpoint"]
            .as_str()
            .expect("Mountpoint should be a string")
            .contains(volume_name));
    }
}
