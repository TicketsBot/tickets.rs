use super::KubernetesClient;
use crate::RouterError;
use std::collections::HashMap;
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use kube::api::ListParams;
use kube_runtime::watcher;
use futures_util::TryStreamExt;
use kube_runtime::watcher::Event;
use tokio::sync::mpsc;
use super::{PodUpdate, PodData};
use std::sync::Arc;

impl KubernetesClient {
    pub async fn get_pods(self: Arc<Self>, labels: HashMap<String, String>) -> Result<Vec<Pod>, RouterError> {
        let pod_api: Api<Pod> = Api::namespaced(self.kube_client.clone(), &self.namespace);

        let params = ListParams::default()
            .labels(&KubernetesClient::build_labels_string(labels)[..]);

        let pods = pod_api.list(&params).await.map_err(RouterError::KubeError)?;
        Ok(pods.items)
    }

    pub async fn watch_pods(self: Arc<Self>, tx: mpsc::Sender<PodUpdate>, labels: HashMap<String, String>) -> Result<(), RouterError> {
        let pod_api: Api<Pod> = Api::namespaced(self.kube_client.clone(), &self.namespace);

        let params = ListParams::default()
            .labels(&KubernetesClient::build_labels_string(labels)[..]);

        let watcher = watcher(pod_api, params);
        watcher.try_for_each(|ev| async {
            let mut tx = tx.clone();

            // TODO: Error handling
            match ev {
                Event::Applied(pod) => {
                    if let Some(data) = KubernetesClient::pod_to_pod_data(pod) {
                        let _ = tx.send(PodUpdate::Add(data)).await;
                    }
                }

                Event::Deleted(pod) => {
                    if let Some(name) = pod.metadata.name {
                        let _ = tx.send(PodUpdate::Remove(name)).await;
                    }
                }

                Event::Restarted(pods) => {
                    let data: Vec<PodData> = pods.into_iter()
                        .filter_map(|pod| KubernetesClient::pod_to_pod_data(pod))
                        .collect();

                    let _ = tx.send(PodUpdate::LostConnection(data)).await;
                }
            }

            Ok(())
        }).await.map_err(RouterError::WatcherError)
    }

    pub fn get_pod_ips(pods: Vec<Pod>) -> Result<Vec<String>, RouterError> {
        let ips = pods.into_iter()
            .filter_map(|pod| pod.status.unwrap().pod_ip)
            .collect();

        Ok(ips)
    }

    fn build_labels_string<'a>(labels: HashMap<String, String>) -> String {
        let labels: Vec<String> = labels.into_iter()
            .map(|(mut key, value)| {
                key.push_str(&value);
                key
            })
            .collect();

        labels.join(",")
    }

    fn pod_to_pod_data(pod: Pod) -> Option<PodData> {
        if let Some(name) = pod.metadata.name {
            if let Some(ip) = pod.status.map(|status| status.pod_ip).flatten() {
                return Some(PodData::new(name, ip));
            }
        }

        None
    }
}