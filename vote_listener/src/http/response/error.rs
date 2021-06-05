use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    error: Box<str>,
}

impl ErrorResponse {
    pub fn new(error: Box<str>) -> ErrorResponse {
        ErrorResponse { error }
    }
}
