use anyhow::Result;
use rglua::prelude::*;
use skidscan::Signature;

mod error;
mod config;

use error::AcceleratorError;
use config::{Config, CONFIG_LOCATION};

#[macro_export]
macro_rules! log {
    ($state:expr, $module:expr, $fmt:expr, $( $arg:expr ),*) => {
        printgm!($state, concat!("accelerator::{:<10}", $fmt), $module, $( $arg ),*);
    };
    ($state:expr, $module:expr, $fmt:expr) => {
        printgm!($state, concat!("accelerator::{:<10}", $fmt), $module);
    };
}

const TARGET: &str = env!("TARGET");

#[gmod_open]
fn open(state: LuaState) -> Result<i32> {
    log!(state, "core", "loading config for target `{}`...", TARGET);

    let config = Config::from_file(CONFIG_LOCATION, TARGET)?;

    let get_download_queue_size: Signature = config.get_value("CL_GetDownloadQueueSize")?
            .parse().map_err(AcceleratorError::SigParseError)?;
    let queue_download: Signature = config.get_value("CL_QueueDownload")?
            .parse().map_err(AcceleratorError::SigParseError)?;
    let download_update: Signature = config.get_value("CL_DownloadUpdate")?
            .parse().map_err(AcceleratorError::SigParseError)?;

    unsafe {
        log!(state, "core", "CL_GetDownloadQueueSize => {:?}", get_download_queue_size.scan_module("engine_client.so")
                .map_err(AcceleratorError::ModuleScanError)?);
        log!(state, "core", "CL_QueueDownload => {:?}", queue_download.scan_module("engine_client.so")
                .map_err(AcceleratorError::ModuleScanError)?);
        log!(state, "core", "CL_DownloadUpdate => {:?}", download_update.scan_module("engine_client.so")
                .map_err(AcceleratorError::ModuleScanError)?);
    }
    Ok(0)
}

#[gmod_close]
fn close(state: LuaState) -> Result<i32> {
    log!(state, "core", "unloading...");
    
    Ok(0)
}