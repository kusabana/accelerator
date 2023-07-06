use thiserror::Error;
use skidscan::ModuleSigScanError;
use skidscan::SignatureParseError;

#[derive(Error, Debug)]
pub enum AcceleratorError {
    #[error("config entry for `{0}` is not present")]
    EntryMissing(String),
    #[error("config entry for `{0}` is invalid")]
    EntryInvalid(String),
    
    #[error("an error occoured while scanning for signature")]
    ModuleScanError(ModuleSigScanError),
    #[error("an error occoured while parsing signature")]
    SigParseError(SignatureParseError),
}