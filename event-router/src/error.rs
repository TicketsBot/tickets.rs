use k8s_openapi::api::core::v1 as api;

#[derive(thiserror::Error, Debug)]
pub enum RouterError {
    #[error("error occurred while performing kubernetes request: {0}")]
    K8sRequestError(#[from] k8s_openapi::RequestError),

    #[error("error occurred while reading kubernetes response: {0}")]
    K8sResponseError(#[from] k8s_openapi::ResponseError),

    #[error("error occurred while executing http request: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("error occurred while parsing response object, status code {0}, data: {1:?}")]
    UnexpectedResponse(k8s_openapi::http::StatusCode, k8s_openapi::ListResponse<api::Pod>),

    #[error("kube api returned an error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("kube watcher returned an error: {0}")]
    WatcherError(#[from] kube_runtime::watcher::Error),
}

impl<T> Into<Result<T, Self>> for RouterError {
    fn into(self) -> Result<T, RouterError> {
        Err(self)
    }
}