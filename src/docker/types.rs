use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerCreateRequest {
    pub image: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub cmd: Option<Vec<String>>,
    #[serde(default)]
    pub entrypoint: Option<Vec<String>>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub host_config: Option<HostConfig>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    #[serde(default)]
    pub binds: Vec<String>,
    #[serde(default)]
    pub port_bindings: HashMap<String, Vec<PortBinding>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct PortBinding {
    pub host_ip: Option<String>,
    pub host_port: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerCreateResponse {
    pub id: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInspectResponse {
    pub id: String,
    pub created: String,
    pub path: String,
    pub args: Vec<String>,
    pub state: ContainerState,
    pub image: String,
    pub name: String,
    pub config: ContainerConfig,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerState {
    pub status: String,
    pub running: bool,
    pub paused: bool,
    pub restarting: bool,
    pub oom_killed: bool,
    pub dead: bool,
    pub pid: i32,
    pub exit_code: i32,
    pub error: String,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub image: String,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct VersionResponse {
    pub version: String,
    pub api_version: String,
    pub git_commit: String,
    pub go_version: String,
    pub os: String,
    pub arch: String,
    pub kernel_version: String,
    pub build_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerWaitResponse {
    pub status_code: i64,
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorDetail {
    pub message: String,
}
