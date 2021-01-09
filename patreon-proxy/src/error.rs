use thiserror::Error;

#[derive(Error, Debug)]
pub enum PatreonError {
    #[error("Missing tokens for client ID {0}")]
    MissingTokens(String),
}