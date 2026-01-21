use crate::error::{Error, Result};
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

    pub async fn create_job(
        &self,
        container_id: &str,
        image: &str,
        env: Vec<String>,
        cmd: Option<Vec<String>>,
        entrypoint: Option<Vec<String>>,
        working_dir: Option<String>,
        labels: BTreeMap<String, String>,
    ) -> Result<Job> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);

        // Convert environment variables from Docker format (KEY=VALUE) to Kubernetes format
        let env_vars: Vec<EnvVar> = env
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
        pod_labels.insert("container-id".to_string(), container_id.to_string());
        
        // Merge user-provided labels
        for (k, v) in labels {
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
            image: Some(image.to_string()),
            env: Some(env_vars),
            resources: Some(ResourceRequirements {
                limits: Some(limits),
                requests: Some(requests),
                claims: None,
            }),
            ..Default::default()
        };

        if let Some(cmd) = cmd {
            container.args = Some(cmd);
        }

        if let Some(entrypoint) = entrypoint {
            container.command = Some(entrypoint);
        }

        if let Some(working_dir) = working_dir {
            container.working_dir = Some(working_dir);
        }

        let job = Job {
            metadata: ObjectMeta {
                name: Some(format!("shim-{}", container_id)),
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
        };

        let created_job = jobs.create(&PostParams::default(), &job).await?;
        Ok(created_job)
    }

    pub async fn get_job(&self, container_id: &str) -> Result<Job> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let job_name = format!("shim-{}", container_id);
        
        jobs.get(&job_name)
            .await
            .map_err(|_| Error::NotFound(format!("Container {} not found", container_id)))
    }

    pub async fn delete_job(&self, container_id: &str) -> Result<()> {
        let jobs: Api<Job> = Api::namespaced(self.client.clone(), &self.namespace);
        let job_name = format!("shim-{}", container_id);
        
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

