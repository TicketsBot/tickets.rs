use crate::RouterError;

pub struct KubernetesClient {
    pub(crate) kube_client: kube::Client,
    pub(crate) namespace: Box<str>,
}

impl KubernetesClient {
    pub async fn new(namespace: Box<str>) -> Result<KubernetesClient, RouterError> {
        Ok(KubernetesClient {
            kube_client: kube::Client::try_default().await.map_err(RouterError::KubeError)?,
            namespace,
        })
    }
}
