use crate::docker::multiplex::{encode_log_line, StreamType};
use crate::error::Error;
use crate::kubernetes::{JobManager, PodManager};
use futures::StreamExt;
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, StreamBody};
use hyper::{
    body::{Bytes, Frame},
    Response, StatusCode,
};
use std::sync::Arc;

pub type LogBody = UnsyncBoxBody<Bytes, std::io::Error>;

pub async fn handle_logs(
    container_id: String,
    follow: bool,
    tail_lines: Option<i64>,
    job_manager: Arc<JobManager>,
    pod_manager: Arc<PodManager>,
) -> Result<Response<LogBody>, Error> {
    // Get the job
    let job = job_manager.get_job(&container_id).await?;

    // Get the pod associated with the job
    let pod = pod_manager.get_pod_for_job(&job).await?;
    let pod_name = pod
        .metadata
        .name
        .as_ref()
        .ok_or_else(|| Error::Internal("Pod has no name".to_string()))?;

    if follow {
        // Stream logs in real-time
        let log_stream = pod_manager.stream_logs(pod_name, true).await?;

        let multiplexed_stream = log_stream.map(|result| {
            match result {
                Ok(bytes) => {
                    // Encode each chunk with Docker multiplex protocol
                    let encoded = encode_log_line(
                        StreamType::Stdout,
                        std::str::from_utf8(&bytes).unwrap_or(""),
                    );
                    Ok(Frame::data(Bytes::from(encoded)))
                }
                Err(e) => {
                    tracing::error!("Error streaming logs: {}", e);
                    Err(std::io::Error::other(e.to_string()))
                }
            }
        });

        let body = StreamBody::new(multiplexed_stream)
            .map_err(|e| std::io::Error::other(e.to_string()))
            .boxed_unsync();

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/vnd.docker.raw-stream")
            .body(body)
            .unwrap())
    } else {
        // Get logs once
        // Pods might be in Succeeded/Terminated state, but logs are still available.
        // However, if the pod is still creating, this will fail.
        let logs = match pod_manager.get_logs(pod_name, false, tail_lines).await {
            Ok(l) => l,
            Err(e) => {
                // If it's a "waiting to start" error, we can return an empty string or a specific message
                // instead of a 500 error, as this is a common race condition in Docker-on-K8s
                let err_str = e.to_string();
                if err_str.contains("waiting to start") || err_str.contains("ContainerCreating") {
                    tracing::warn!(
                        "Logs requested for pod {} but it is still creating",
                        pod_name
                    );
                    "".to_string()
                } else {
                    return Err(e);
                }
            }
        };

        // Split into lines and encode each with Docker multiplex protocol
        let mut encoded_logs = Vec::new();
        for line in logs.lines() {
            encoded_logs.extend_from_slice(&encode_log_line(StreamType::Stdout, line));
        }

        let body = Full::new(Bytes::from(encoded_logs))
            .map_err(|e| match e {})
            .boxed_unsync();

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/vnd.docker.raw-stream")
            .body(body)
            .unwrap())
    }
}
