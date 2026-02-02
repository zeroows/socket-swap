use crate::error::{Error, Result};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, LogParams},
    Client,
};

pub struct PodManager {
    client: Client,
    namespace: String,
}

impl PodManager {
    pub fn new(client: Client, namespace: String) -> Self {
        PodManager { client, namespace }
    }

    /// Find the pod associated with a job
    pub async fn get_pod_for_job(&self, job: &Job) -> Result<Pod> {
        use backon::ExponentialBuilder;
        use backon::Retryable;

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        // Get the job name to find associated pods
        let job_name = job
            .metadata
            .name
            .as_ref()
            .ok_or_else(|| Error::Internal("Job has no name".to_string()))?;

        let list_fut =
            move || {
                let pods = pods.clone();
                let job_name = job_name.clone();
                async move {
                    // List pods with the job-name label
                    let lp = ListParams::default().labels(&format!("job-name={}", job_name));
                    let pod_list = pods.list(&lp).await.map_err(Error::Kube)?;

                    pod_list.items.into_iter().next().ok_or_else(|| {
                        Error::NotFound(format!("No pod found for job {}", job_name))
                    })
                }
            };

        list_fut
            .retry(ExponentialBuilder::default().with_max_times(5))
            .await
    }

    /// Get logs from a pod
    pub async fn get_logs(
        &self,
        pod_name: &str,
        follow: bool,
        tail_lines: Option<i64>,
    ) -> Result<String> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let mut log_params = LogParams::default();
        if follow {
            log_params.follow = true;
        }
        if let Some(lines) = tail_lines {
            log_params.tail_lines = Some(lines);
        }

        let logs = pods.logs(pod_name, &log_params).await?;
        Ok(logs)
    }

    /// Stream logs from a pod
    pub async fn stream_logs(
        &self,
        pod_name: &str,
        follow: bool,
    ) -> Result<impl futures::Stream<Item = Result<bytes::Bytes>>> {
        use backon::ExponentialBuilder;
        use backon::Retryable;
        use futures::io::AsyncBufReadExt;
        use futures::StreamExt;

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let log_params = LogParams {
            follow,
            ..Default::default()
        };

        // Retry logic for log stream, as pods might take a moment to start
        let reader_fut = move || {
            let pods = pods.clone();
            let pod_name = pod_name.to_string();
            let log_params = log_params.clone();
            async move { pods.log_stream(&pod_name, &log_params).await }
        };

        let reader = reader_fut
            .retry(ExponentialBuilder::default().with_max_times(5))
            .await
            .map_err(Error::Kube)?;

        // Convert AsyncBufRead to a Stream of lines
        let stream = reader.lines().map(|result: std::io::Result<String>| {
            result
                .map(|line| bytes::Bytes::from(format!("{}\n", line)))
                .map_err(Error::Io)
        });

        Ok(stream)
    }

    /// Get pod status
    #[allow(dead_code)]
    pub async fn get_pod(&self, pod_name: &str) -> Result<Pod> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        pods.get(pod_name)
            .await
            .map_err(|_| Error::NotFound(format!("Pod {} not found", pod_name)))
    }
}
