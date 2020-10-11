use std::io;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error occurred during I/O operation: {0}")]
    IOError(#[from] io::Error),

    #[error("error occurred during database operation: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),
}