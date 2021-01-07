use hashring::HashRing;
use model::Snowflake;
use crate::k8s::{KubernetesClient, PodUpdate};
use std::collections::HashMap;
use crate::RouterError;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use std::sync::Arc;

pub struct Pool {
    ring: Mutex<HashRing<String>>,
    client: Arc<KubernetesClient>,
}

impl Pool {
    pub fn new(client: KubernetesClient) -> Pool {
        Pool {
            ring: Mutex::new(HashRing::new()),
            client: Arc::new(client),
        }
    }

    pub async fn get_node(&mut self, guild_id: Snowflake) -> Option<String> {
        let mut ring = self.ring.lock().await;
        let node = ring.get(&guild_id);
        node.cloned()
    }

    pub async fn maintain_nodes(&mut self) {
        let (tx, mut rx) = mpsc::channel(1);

        let labels = HashMap::new(); // TODO: Populate app=worker
        let labels_cloned = labels.clone();

        let client = Arc::clone(&self.client);
        tokio::spawn(async move {
            loop {
                let mut tx = tx.clone();
                let client = Arc::clone(&client);
                client.watch_pods(tx, labels_cloned.clone()).await;
            }
        });

        self.repopulate(labels.clone()).await;

        while let Some(update) = rx.recv().await {
            match update {
                PodUpdate::Add(data) => {
                    let mut ring = self.ring.lock().await;
                    ring.add(data.ip);
                }

                PodUpdate::Remove(name) => {

                }

                PodUpdate::LostConnection(new_pods) => {
                    let mut ring = self.ring.lock().await;
                    *ring = HashRing::new();

                    new_pods.into_iter().for_each(|pod| ring.add(pod.ip));
                }
            }
        }
    }

    pub async fn repopulate(&mut self, labels: HashMap<String, String>) -> Result<(), RouterError> {
        let mut ring = self.ring.lock().await; // we want to lock for the entire function

        let pod_ips = Arc::clone(&self.client).get_pods(labels).await
            .and_then(|pods| KubernetesClient::get_pod_ips(pods))?;

        pod_ips.into_iter().for_each(|ip| ring.add(ip));

        Ok(())
    }
}