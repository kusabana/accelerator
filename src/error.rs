use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("signature not found")]
    SigNotFound,
    #[error("remote file `{0}` not found on `{1}`")]
    RemoteFileNotFound(String, String),
}
