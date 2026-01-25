use crate::error::{Error, Result};
use derive_builder::Builder;
use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::{
    api::{Api, DeleteParams, ListParams, PostParams},
    Client,
};
use std::collections::BTreeMap;

#[derive(Builder)]
#[builder(setter(into))]
pub struct JobConfig {
    pub container_id: String,
    pub image: String,
    #[builder(default)]
    pub env: Vec<String>,
    #[builder(default)]
    pub cmd: Option<Vec<String>>,
    #[builder(default)]
    pub entrypoint: Option<Vec<String>>,
    #[builder(default)]
    pub working_dir: Option<String>,
    #[builder(default)]
    pub labels: BTreeMap<String, String>,
}

pub struct JobManager {
    client: Client,
    namespace: String,
    ttl_seconds: i32,
    cpu_limit: String,
    memory_limit: String,
    cpu_request: String,
    memory_request: String,
}

impl JobManager {
    pub async fn new(
        namespace: String,
        ttl_seconds: i32,
        cpu_limit: String,
        memory_limit: String,
        cpu_request: String,
        memory_request: String,
    ) -> Result<Self> {
        let client = Client::try_default().await?;
        Ok(JobManager {
            client,
            namespace,
            ttl_seconds,
            cpu_limit,
            memory_limit,
            cpu_request,
            memory_request,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn build_job(&self, config: JobConfig) -> Job {
        // Convert environment variables from Docker format (KEY=VALUE) to Kubernetes format
        let env_vars: Vec<EnvVar> = config
            .env
            .iter()
            .filter_map(|e| {
                let parts: Vec<&str> = e.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some(EnvVar {
                        name: parts[0].to_string(),
                        value: Some(parts[1].to_string()),
                        value_from: None,
                    })
                } else {
                    None
                }
            })
            .collect();

        let mut pod_labels = BTreeMap::new();
        pod_labels.insert("socket-shim".to_string(), "true".to_string());
        pod_labels.insert("container-id".to_string(), config.container_id.clone());

        // Merge user-provided labels
        for (k, v) in config.labels {
            pod_labels.insert(k, v);
        }

        let mut limits = BTreeMap::new();
        limits.insert("cpu".to_string(), Quantity(self.cpu_limit.clone()));
        limits.insert("memory".to_string(), Quantity(self.memory_limit.clone()));

        let mut requests = BTreeMap::new();
        requests.insert("cpu".to_string(), Quantity(self.cpu_request.clone()));
        requests.insert("memory".to_string(), Quantity(self.memory_request.clone()));

        let mut container = Container {
            name: "task".to_string(),
            image: Some(config.image),
            env: Some(env_vars),
            resources: Some(ResourceRequirements {
                limits: Some(limits),
                requests: Some(requests),
                claims: None,
            }),
            ..Default::default()
        };

        if let Some(cmd) = config.cmd {
            container.args = Some(cmd);
        }

        if let Some(entrypoint) = config.entrypoint {
            container.command = Some(entrypoint);
        }

        if let Some(working_dir) = config.working_dir {
            container.working_dir = Some(working_dir);
        }

        Job {
            metadata: ObjectMeta {
                name: Some(format!("socket-swap-{}", config.container_id)),
                labels: Some(pod_labels.clone()),
                ..Default::default()
            },
            spec: Some(JobSpec {
                ttl_seconds_after_finished: Some(self.ttl_seconds),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(pod_labels),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![container],
                        restart_policy: Some("Never".to_string()),
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub async fn create_job(&self, config: JobConfig) -> Result<Job> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let job = self.build_job(config);
        let created_job = jobs.create(&PostParams::default(), &job).await?;
        Ok(created_job)
    }

    pub async fn get_job(&self, container_id: &str) -> Result<Job> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let job_name = format!("socket-swap-{}", container_id);

        jobs.get(&job_name)
            .await
            .map_err(|_| Error::NotFound(format!("Container {} not found", container_id)))
    }

    pub async fn delete_job(&self, container_id: &str) -> Result<()> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let job_name = format!("socket-swap-{}", container_id);

        jobs.delete(&job_name, &DeleteParams::default())
            .await
            .map_err(|_| Error::NotFound(format!("Container {} not found", container_id)))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn list_jobs(&self) -> Result<Vec<Job>> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams::default().labels("socket-shim=true");

        let job_list = jobs.list(&lp).await?;
        Ok(job_list.items)
    }

    #[allow(dead_code)]
    pub fn extract_container_id(&self, job: &Job) -> Option<String> {
        job.metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get("container-id"))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::Client;

    async fn mock_job_manager() -> JobManager {
        // We use a dummy client for unit tests that don't hit the API
        // We can create a Client from a dummy config using Client::try_from
        use kube::config::Config;
        let config = Config::new(hyper::Uri::from_static("http://localhost"));
        let client = Client::try_from(config).unwrap();

        JobManager {
            client,
            namespace: "default".to_string(),
            ttl_seconds: 300,
            cpu_limit: "500m".to_string(),
            memory_limit: "512Mi".to_string(),
            cpu_request: "100m".to_string(),
            memory_request: "128Mi".to_string(),
        }
    }

    #[tokio::test]
    async fn test_build_job_basic() {
        let manager = mock_job_manager().await;
        let container_id = "test-container";
        let image = "nginx:latest";
        let env = vec!["FOO=BAR".to_string(), "INVALID".to_string()];

        let config = JobConfigBuilder::default()
            .container_id(container_id)
            .image(image)
            .env(env)
            .build()
            .unwrap();

        let job = manager.build_job(config);

        assert_eq!(
            job.metadata.name,
            Some("socket-swap-test-container".to_string())
        );

        let pod_spec = job.spec.unwrap().template.spec.unwrap();
        let container = &pod_spec.containers[0];

        assert_eq!(container.image, Some(image.to_string()));

        // Check env vars
        let env_vars = container.env.as_ref().unwrap();
        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0].name, "FOO");
        assert_eq!(env_vars[0].value, Some("BAR".to_string()));

        // Check labels
        let job_labels = job.metadata.labels.as_ref().unwrap();
        assert_eq!(job_labels.get("socket-shim"), Some(&"true".to_string()));
        assert_eq!(
            job_labels.get("container-id"),
            Some(&container_id.to_string())
        );
    }

    #[tokio::test]
    async fn test_build_job_with_commands() {
        let manager = mock_job_manager().await;
        let cmd = vec!["arg1".to_string(), "arg2".to_string()];
        let entrypoint = vec!["/bin/sh".to_string(), "-c".to_string()];
        let working_dir = "/app".to_string();

        let config = JobConfigBuilder::default()
            .container_id("test")
            .image("image")
            .cmd(cmd.clone())
            .entrypoint(entrypoint.clone())
            .working_dir(working_dir.clone())
            .build()
            .unwrap();

        let job = manager.build_job(config);

        let container = &job.spec.unwrap().template.spec.unwrap().containers[0];
        assert_eq!(container.args, Some(cmd));
        assert_eq!(container.command, Some(entrypoint));
        assert_eq!(container.working_dir, Some(working_dir));
    }

    #[tokio::test]
    async fn test_extract_container_id() {
        let manager = mock_job_manager().await;
        let mut labels = BTreeMap::new();
        labels.insert("container-id".to_string(), "my-id".to_string());

        let job = Job {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            manager.extract_container_id(&job),
            Some("my-id".to_string())
        );
    }
}
