use http_body_util::Full;
use hyper::{body::Bytes, Response, StatusCode};
use serde_json::json;

pub async fn handle_image_create() -> Response<Full<Bytes>> {
    // In our Kubernetes shim model, we don't actually "pull" images to the shim.
    // Kubernetes will pull the image when the Job is created.
    // We return 200 OK to satisfy the Docker client that the "pull" was successful.
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from("{\"status\":\"Image is up to date\"}")))
        .unwrap()
}

pub async fn handle_image_inspect(image_name: String) -> Response<Full<Bytes>> {
    // Return a minimal valid image inspect response.
    // This satisfies clients that check if an image exists before running it.
    let response = json!({
        "Id": format!("sha256:{}", image_name), // Dummy ID
        "RepoTags": [image_name],
        "Config": {
            "Labels": {}
        },
        "RootFS": {
            "Type": "layers",
            "Layers": []
        }
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_handle_image_create() {
        let response = handle_image_create().await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "Image is up to date");
    }

    #[tokio::test]
    async fn test_handle_image_inspect() {
        let image_name = "busybox:latest";
        let response = handle_image_inspect(image_name.to_string()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["RepoTags"][0], image_name);
        assert!(json["Id"].as_str().unwrap().starts_with("sha256:"));
    }
}
