mod config;
mod docker;
mod error;
mod handlers;
mod kubernetes;
mod server;

use config::Config;
use error::Result;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto::Builder;
use kubernetes::{JobManager, PodManager};
use server::Router;
use std::sync::Arc;
use tokio::net::UnixListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!(
        "Starting Socket Shim - version: {}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let config = Config::from_env();
    info!("Socket path: {}", config.socket_path);
    if let Some(ref tcp_addr) = config.tcp_addr {
        info!("TCP address: {}", tcp_addr);
    }
    info!("Namespace: {}", config.namespace);
    info!("TTL seconds: {}", config.ttl_seconds_after_finished);
    info!(
        "Active deadline seconds: {}",
        config.active_deadline_seconds
    );
    info!("Default CPU limit: {}", config.default_cpu_limit);
    info!("Default Memory limit: {}", config.default_memory_limit);
    info!("Default CPU request: {}", config.default_cpu_request);
    info!("Default Memory request: {}", config.default_memory_request);

    // Initialize Kubernetes clients
    info!("Initializing Kubernetes clients...");
    let job_manager = Arc::new(
        JobManager::new(
            config.namespace.clone(),
            config.ttl_seconds_after_finished,
            config.active_deadline_seconds,
            config.default_cpu_limit.clone(),
            config.default_memory_limit.clone(),
            config.default_cpu_request.clone(),
            config.default_memory_request.clone(),
        )
        .await?,
    );
    let pod_manager = Arc::new(PodManager::new(
        job_manager.client().clone(),
        config.namespace.clone(),
    ));

    // Spawn Unix socket listener
    let unix_job_manager = job_manager.clone();
    let unix_pod_manager = pod_manager.clone();
    let socket_path = config.socket_path.clone();

    // Only try to start Unix listener if we have permissions or if it's not in a protected path
    tokio::spawn(async move {
        if let Err(e) = start_unix_listener(socket_path, unix_job_manager, unix_pod_manager).await {
            error!("Unix socket listener failed: {}", e);
            info!("TIP: If running locally, you might need to set DOCKER_SOCKET_PATH to a local path you have write access to (e.g., ./docker.sock)");
        }
    });

    // If TCP address is configured, start TCP listener
    if let Some(tcp_addr) = config.tcp_addr {
        info!("Starting TCP listener on {}", tcp_addr);
        start_tcp_listener(tcp_addr, job_manager, pod_manager).await?;
    } else {
        info!("TCP listener not configured, running Unix socket only");
        // Keep running if only Unix socket
        std::future::pending::<()>().await;
    }

    Ok(())
}

async fn start_unix_listener(
    socket_path: String,
    job_manager: Arc<JobManager>,
    pod_manager: Arc<PodManager>,
) -> Result<()> {
    // Remove existing socket file if it exists
    if std::path::Path::new(&socket_path).exists() {
        std::fs::remove_file(&socket_path)?;
    }

    info!("Binding to Unix socket: {}", socket_path);
    let listener = UnixListener::bind(&socket_path)?;

    // Set socket permissions so other containers in the pod can access it
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&socket_path)?.permissions();
        perms.set_mode(0o666); // rw-rw-rw-
        std::fs::set_permissions(&socket_path, perms)?;
        info!("Socket permissions set to 0666");
    }

    info!("Unix socket listening on {}", socket_path);
    info!("Ready to accept Unix socket connections");

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let job_manager = job_manager.clone();
                let pod_manager = pod_manager.clone();
                tokio::spawn(handle_connection(stream, job_manager, pod_manager));
            }
            Err(e) => {
                error!("Error accepting Unix connection: {}", e);
            }
        }
    }
}

async fn start_tcp_listener(
    addr: String,
    job_manager: Arc<JobManager>,
    pod_manager: Arc<PodManager>,
) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("TCP listener ready on {}", addr);
    info!("Ready to accept TCP connections");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New TCP connection from {}", addr);
                let job_manager = job_manager.clone();
                let pod_manager = pod_manager.clone();
                tokio::spawn(handle_connection(stream, job_manager, pod_manager));
            }
            Err(e) => {
                error!("Error accepting TCP connection: {}", e);
            }
        }
    }
}

async fn handle_connection<S>(stream: S, job_manager: Arc<JobManager>, pod_manager: Arc<PodManager>)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let io = TokioIo::new(stream);
    let router = Arc::new(Router::new(job_manager, pod_manager));

    let service = service_fn(move |req| {
        let router = router.clone();
        async move {
            use http_body_util::BodyExt;
            match router.route(req).await {
                Ok(response) => Ok::<_, error::Error>(response),
                Err(e) => {
                    let status = match e {
                        error::Error::NotFound(_) => {
                            info!("Resource not found: {}", e);
                            hyper::StatusCode::NOT_FOUND
                        }
                        error::Error::BadRequest(_) => {
                            error!("Bad request: {}", e);
                            hyper::StatusCode::BAD_REQUEST
                        }
                        error::Error::Kube(kube::Error::Api(ref api_err))
                            if api_err.code == 409 =>
                        {
                            // Map Kubernetes "AlreadyExists" (409) to Docker "Conflict" (409)
                            info!("Resource already exists: {}", e);
                            hyper::StatusCode::CONFLICT
                        }
                        _ => {
                            error!("Error handling request: {}", e);
                            hyper::StatusCode::INTERNAL_SERVER_ERROR
                        }
                    };
                    let err_resp = handlers::containers::error_response(status, &e.to_string());
                    Ok(err_resp.map(|body| {
                        body.map_err(|e: std::convert::Infallible| match e {})
                            .boxed_unsync()
                    }))
                }
            }
        }
    });

    if let Err(err) = Builder::new(hyper_util::rt::TokioExecutor::new())
        .serve_connection(io, service)
        .await
    {
        error!("Error serving connection: {:?}", err);
    }
}
