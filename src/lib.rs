#![feature(c_unwind)]
#![feature(let_chains)]
#![allow(non_snake_case)]

use anyhow::Result;
use rglua::prelude::*;

mod error;
mod detour;

#[macro_export]
macro_rules! log {
    ($state:expr, $module:expr, $fmt:expr, $( $arg:expr ),*) => {
        printgm!($state, concat!("accelerator::{:<10}", $fmt), $module, $( $arg ),*);
    };
    ($state:expr, $module:expr, $fmt:expr) => {
        printgm!($state, concat!("accelerator::{:<10}", $fmt), $module);
    };
}

#[gmod_open]
unsafe fn open(state: LuaState) -> Result<i32> {
    log!(state, "core", "loading...");

    unsafe { detour::apply(state)? };

    Ok(0)
}

#[gmod_close]
unsafe fn close(state: LuaState) -> Result<i32> {
    log!(state, "core", "unloading...");

    unsafe { detour::revert(state)? };
    
    Ok(0)
}