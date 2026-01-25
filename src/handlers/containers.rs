use crate::docker::{
    ContainerConfig, ContainerCreateRequest, ContainerCreateResponse, ContainerInspectResponse,
    ContainerState, ContainerWaitResponse, ErrorDetail, ErrorResponse,
};
use crate::error::Error;
use crate::kubernetes::JobManager;
use http_body_util::Full;
use hyper::{body::Bytes, Response, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_create(
    body: bytes::Bytes,
    job_manager: Arc<JobManager>,
) -> Result<Response<Full<Bytes>>, Error> {
    let create_req: ContainerCreateRequest = serde_json::from_slice(&body)?;

    // Generate a unique container ID
    let container_id = Uuid::new_v4().to_string().replace("-", "")[..12].to_string();

    // Convert labels to BTreeMap
    let labels: std::collections::BTreeMap<String, String> =
        create_req.labels.into_iter().collect();

    // Create Kubernetes Job configuration using Builder pattern
    let mut job_config_builder = crate::kubernetes::JobConfigBuilder::default();
    job_config_builder
        .container_id(container_id.clone())
        .image(create_req.image)
        .env(create_req.env)
        .labels(labels);

    if let Some(cmd) = create_req.cmd {
        job_config_builder.cmd(Some(cmd));
    }
    if let Some(entrypoint) = create_req.entrypoint {
        job_config_builder.entrypoint(Some(entrypoint));
    }
    if let Some(working_dir) = create_req.working_dir {
        job_config_builder.working_dir(Some(working_dir));
    }

    let job_config = job_config_builder
        .build()
        .map_err(|e| Error::Internal(format!("Failed to build job config: {}", e)))?;

    // Create Kubernetes Job
    job_manager.create_job(job_config).await?;

    let response = ContainerCreateResponse {
        id: container_id,
        warnings: vec![],
    };

    let json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
}

pub async fn handle_start(container_id: String) -> Result<Response<Full<Bytes>>, Error> {
    // Jobs start automatically in Kubernetes, so this is a no-op
    tracing::info!("Container {} start requested (no-op)", container_id);

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .unwrap())
}

pub async fn handle_stop(
    container_id: String,
    job_manager: Arc<JobManager>,
) -> Result<Response<Full<Bytes>>, Error> {
    // Stopping a container in our model means deleting the Job
    job_manager.delete_job(&container_id).await?;

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .unwrap())
}

pub async fn handle_delete(
    container_id: String,
    job_manager: Arc<JobManager>,
) -> Result<Response<Full<Bytes>>, Error> {
    job_manager.delete_job(&container_id).await?;

    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .unwrap())
}

pub async fn handle_inspect(
    container_id: String,
    job_manager: Arc<JobManager>,
) -> Result<Response<Full<Bytes>>, Error> {
    let job = job_manager.get_job(&container_id).await?;

    // Extract job information
    let job_name = job
        .metadata
        .name
        .as_ref()
        .unwrap_or(&"unknown".to_string())
        .clone();
    let created = job
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|t| t.0.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Get container spec from job
    let container_spec = job
        .spec
        .as_ref()
        .and_then(|spec| spec.template.spec.as_ref())
        .and_then(|pod_spec| pod_spec.containers.first());

    let image = container_spec
        .and_then(|c| c.image.as_ref())
        .unwrap_or(&"unknown".to_string())
        .clone();

    let cmd = container_spec
        .and_then(|c| c.args.as_ref())
        .cloned()
        .unwrap_or_default();

    let env = container_spec
        .and_then(|c| c.env.as_ref())
        .map(|envs| {
            envs.iter()
                .filter_map(|e| e.value.as_ref().map(|v| format!("{}={}", e.name, v)))
                .collect()
        })
        .unwrap_or_default();

    // Determine container state from Job status
    let (status, running, exit_code, started_at, finished_at) =
        if let Some(job_status) = &job.status {
            if job_status.succeeded.unwrap_or(0) > 0 {
                (
                    "exited".to_string(),
                    false,
                    0,
                    job_status
                        .start_time
                        .as_ref()
                        .map(|t| t.0.to_string())
                        .unwrap_or_default(),
                    job_status
                        .completion_time
                        .as_ref()
                        .map(|t| t.0.to_string())
                        .unwrap_or_default(),
                )
            } else if job_status.failed.unwrap_or(0) > 0 {
                (
                    "exited".to_string(),
                    false,
                    1,
                    job_status
                        .start_time
                        .as_ref()
                        .map(|t| t.0.to_string())
                        .unwrap_or_default(),
                    job_status
                        .completion_time
                        .as_ref()
                        .map(|t| t.0.to_string())
                        .unwrap_or_default(),
                )
            } else if job_status.active.unwrap_or(0) > 0 {
                (
                    "running".to_string(),
                    true,
                    0,
                    job_status
                        .start_time
                        .as_ref()
                        .map(|t| t.0.to_string())
                        .unwrap_or_default(),
                    "".to_string(),
                )
            } else {
                (
                    "created".to_string(),
                    false,
                    0,
                    "".to_string(),
                    "".to_string(),
                )
            }
        } else {
            (
                "created".to_string(),
                false,
                0,
                "".to_string(),
                "".to_string(),
            )
        };

    let state = ContainerState {
        status,
        running,
        paused: false,
        restarting: false,
        oom_killed: false,
        dead: false,
        pid: 0,
        exit_code,
        error: "".to_string(),
        started_at,
        finished_at,
    };

    let labels = job
        .metadata
        .labels
        .as_ref()
        .map(|l| l.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_else(HashMap::new);

    let response = ContainerInspectResponse {
        id: container_id.clone(),
        created,
        path: cmd.first().cloned().unwrap_or_default(),
        args: cmd.clone(),
        state,
        image: image.clone(),
        name: format!("/{}", job_name),
        config: ContainerConfig {
            image,
            cmd,
            env,
            labels,
        },
    };

    let json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
}

pub async fn handle_wait(
    container_id: String,
    job_manager: Arc<JobManager>,
) -> Result<Response<Full<Bytes>>, Error> {
    // Get the job
    let job = job_manager.get_job(&container_id).await?;

    // Check job status
    let (status_code, error) = if let Some(job_status) = &job.status {
        if job_status.succeeded.unwrap_or(0) > 0 {
            (0, None)
        } else if job_status.failed.unwrap_or(0) > 0 {
            (
                1,
                Some(ErrorDetail {
                    message: "Job failed".to_string(),
                }),
            )
        } else {
            // Job still running - in a real implementation, we'd watch for completion
            (0, None)
        }
    } else {
        (0, None)
    };

    let response = ContainerWaitResponse { status_code, error };

    let json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap())
}

pub fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let error = ErrorResponse {
        message: message.to_string(),
    };
    let json = serde_json::to_string(&error).unwrap();

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_start_no_op() {
        let response = handle_start("test-id".to_string()).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_error_response() {
        let response = error_response(StatusCode::BAD_REQUEST, "test error");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
