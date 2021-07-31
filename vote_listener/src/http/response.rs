use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Response {
    Error { error: String },
    Success { success: bool },
}

impl Response {
    pub fn error(error: &str) -> Response {
        Response::Error {
            error: error.to_owned(),
        }
    }

    pub fn success() -> Response {
        Response::Success { success: true }
    }
}
