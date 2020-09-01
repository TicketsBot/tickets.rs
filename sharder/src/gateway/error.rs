use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("invalid opcode {0}")]
    InvalidOpcode(u8),
}