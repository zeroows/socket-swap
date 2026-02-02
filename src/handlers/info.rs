use crate::docker::VersionResponse;
use http_body_util::Full;
use hyper::{body::Bytes, Response, StatusCode};
use std::convert::Infallible;

pub async fn handle_ping() -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(Full::new(Bytes::from("OK")))
        .unwrap())
}

pub async fn handle_version() -> Result<Response<Full<Bytes>>, Infallible> {
    let version = VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: "1.41".to_string(),
        git_commit: "unknown".to_string(),
        go_version: "go1.19".to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        kernel_version: "unknown".to_string(),
        build_time: "2024-01-01T00:00:00.000000000+00:00".to_string(),
    };

    let json = serde_json::to_string(&version).unwrap();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_handle_ping() {
        let response = handle_ping().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, "OK");
    }

    #[tokio::test]
    async fn test_handle_version() {
        let response = handle_version().await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let version: VersionResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(version.api_version, "1.41");
        assert!(version.version.contains("socket-swap"));
    }
}
