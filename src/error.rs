use thiserror::Error;

#[derive(Error, Debug)]
pub enum BitcoindError {
    #[error("Docker error: {0}")]
    DockerError(#[from] bollard::errors::Error),
    
    #[error("Image hash mismatch: expected {expected}, found {found}")]
    ImageHashMismatch { expected: String, found: String },
    
    #[error("Other error: {0}")]
    Other(String),
}