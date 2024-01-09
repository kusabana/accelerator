use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("signature not found ({0})")]
    SigNotFound(String),
}
