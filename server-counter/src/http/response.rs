use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Response {
    Success { success: bool, count: usize },
}

impl Response {
    pub fn success(count: usize) -> Response {
        Response::Success {
            success: true,
            count,
        }
    }
}
