use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerResponse {
    pub success: bool,
    pub error: Option<String>,
}