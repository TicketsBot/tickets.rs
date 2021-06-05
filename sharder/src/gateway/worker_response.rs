use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerResponse<'a> {
    pub success: bool,
    pub error: Option<&'a str>,
}
