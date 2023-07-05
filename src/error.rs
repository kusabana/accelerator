use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("config entry for `{0}` is not present")]
    EntryMissing(String),
    #[error("config entry for `{0}` is invalid")]
    EntryInvalid(String),
}