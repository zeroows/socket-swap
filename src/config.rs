use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub socket_path: String,
    pub tcp_addr: Option<String>,
    pub namespace: String,
    pub ttl_seconds_after_finished: i32,
    pub active_deadline_seconds: i64,
    pub default_cpu_limit: String,
    pub default_memory_limit: String,
    pub default_cpu_request: String,
    pub default_memory_request: String,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            socket_path: env::var("DOCKER_SOCKET_PATH")
                .unwrap_or_else(|_| "/var/run/docker.sock".to_string()),
            tcp_addr: env::var("DOCKER_TCP_ADDR").ok(),
            namespace: env::var("KUBE_NAMESPACE").unwrap_or_else(|_| "default".to_string()),
            ttl_seconds_after_finished: env::var("JOB_TTL_SECONDS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            active_deadline_seconds: env::var("JOB_ACTIVE_DEADLINE_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),
            default_cpu_limit: env::var("DEFAULT_CPU_LIMIT").unwrap_or_else(|_| "500m".to_string()),
            default_memory_limit: env::var("DEFAULT_MEMORY_LIMIT")
                .unwrap_or_else(|_| "512Mi".to_string()),
            default_cpu_request: env::var("DEFAULT_CPU_REQUEST")
                .unwrap_or_else(|_| "100m".to_string()),
            default_memory_request: env::var("DEFAULT_MEMORY_REQUEST")
                .unwrap_or_else(|_| "128Mi".to_string()),
        }
    }
}
