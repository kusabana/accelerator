use anyhow::Result;
use rglua::prelude::*;

mod error;
mod config;

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
    log!(state, "core", "queue sig: `{}`", config.get_value("queue")?);

    Ok(0)
}

#[gmod_close]
fn close(state: LuaState) -> Result<i32> {
    log!(state, "core", "unloading...");
    
    Ok(0)
}