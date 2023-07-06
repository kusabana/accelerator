use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcceleratorError<'a> {
    #[error("failed to find signature for {0}")]
    SigScanError(&'a str),
}