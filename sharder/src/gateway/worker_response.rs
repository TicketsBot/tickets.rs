use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerResponse<'a> {
    pub success: bool,
    pub error: Option<&'a str>,
}