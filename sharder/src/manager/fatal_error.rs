use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

#[derive(Debug)]
pub struct FatalError {
    pub bot_token: String,
    pub error_code: CloseCode,
    pub error: String,
}

impl FatalError {
    pub fn new(bot_token: String, error_code: CloseCode, error: String) -> FatalError {
        FatalError {
            bot_token,
            error_code,
            error
        }
    }
}