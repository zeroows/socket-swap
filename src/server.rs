use crate::error::Error;
use crate::handlers::{
    containers::{
        error_response, handle_create, handle_delete, handle_inspect, handle_start,
        handle_stop, handle_wait,
    },
    info::{handle_ping, handle_version},
    logs::handle_logs,
};
use crate::kubernetes::{JobManager, PodManager};
use http_body_util::BodyExt;
use hyper::body::Bytes;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use std::sync::Arc;

pub struct Router {
    job_manager: Arc<JobManager>,
    pod_manager: Arc<PodManager>,
}

pub type BoxBody = http_body_util::combinators::UnsyncBoxBody<Bytes, std::io::Error>;

fn box_body<B>(body: B) -> BoxBody
where
    B: hyper::body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    use http_body_util::BodyExt;
    body.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)).boxed_unsync()
}

impl Router {
    pub fn new(job_manager: Arc<JobManager>, pod_manager: Arc<PodManager>) -> Self {
        Router {
            job_manager,
            pod_manager,
        }
    }

    pub async fn route(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<BoxBody>, Error> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();
        let method_str = method.to_string();

        tracing::info!("{} {}", method, path);

        // Remove API version prefix if present (e.g., /v1.41/)
        let path = path
            .strip_prefix("/v1.41")
            .or_else(|| path.strip_prefix("/v1.40"))
            .or_else(|| path.strip_prefix("/v1.39"))
            .unwrap_or(&path)
            .to_string();

        let response = match (method, path.as_str()) {
            // Info endpoints
            (Method::GET, "/_ping") => {
                handle_ping().await.map_err(|_| Error::Internal("Infallible".to_string()))
                    .map(|r| r.map(box_body))?
            }
            (Method::GET, "/version") => {
                handle_version().await.map_err(|_| Error::Internal("Infallible".to_string()))
                    .map(|r| r.map(box_body))?
            }

            // Container lifecycle
            (Method::POST, "/containers/create") => {
                let body = req.collect().await?.to_bytes();
                handle_create(body, self.job_manager.clone()).await
                    .map(|r| r.map(box_body))?
            }

            (Method::POST, path) if path.starts_with("/containers/") && path.ends_with("/start") => {
                let container_id = extract_container_id(&path, "/start");
                handle_start(container_id).await
                    .map(|r| r.map(box_body))?
            }

            (Method::POST, path) if path.starts_with("/containers/") && path.ends_with("/stop") => {
                let container_id = extract_container_id(&path, "/stop");
                handle_stop(container_id, self.job_manager.clone()).await
                    .map(|r| r.map(box_body))?
            }

            (Method::DELETE, path) if path.starts_with("/containers/") => {
                let container_id = path.trim_start_matches("/containers/").to_string();
                handle_delete(container_id, self.job_manager.clone()).await
                    .map(|r| r.map(box_body))?
            }

            (Method::GET, path) if path.starts_with("/containers/") && path.ends_with("/json") => {
                let container_id = extract_container_id(&path, "/json");
                handle_inspect(container_id, self.job_manager.clone()).await
                    .map(|r| r.map(box_body))?
            }

            (Method::GET, path) if path.starts_with("/containers/") && path.ends_with("/logs") => {
                let container_id = extract_container_id(&path, "/logs");
                
                // Parse query parameters for follow and tail
                let query = req.uri().query().unwrap_or("");
                let follow = query.contains("follow=true") || query.contains("follow=1");
                let tail_lines = parse_tail_param(query);
                
                handle_logs(
                    container_id,
                    follow,
                    tail_lines,
                    self.job_manager.clone(),
                    self.pod_manager.clone(),
                )
                .await?
            }

            (Method::POST, path) if path.starts_with("/containers/") && path.ends_with("/wait") => {
                let container_id = extract_container_id(&path, "/wait");
                handle_wait(container_id, self.job_manager.clone()).await
                    .map(|r| r.map(box_body))?
            }

            // Unimplemented endpoints
            _ => error_response(
                StatusCode::NOT_FOUND,
                &format!("Endpoint not implemented: {} {}", method_str, path),
            ).map(box_body),
        };

        Ok(response)
    }
}

fn extract_container_id(path: &str, suffix: &str) -> String {
    path.trim_start_matches("/containers/")
        .trim_end_matches(suffix)
        .to_string()
}

fn parse_tail_param(query: &str) -> Option<i64> {
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("tail=") {
            if let Ok(n) = value.parse::<i64>() {
                return Some(n);
            }
        }
    }
    None
}

