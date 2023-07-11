#![feature(c_unwind)]
#![feature(thread_id_value)]
#![feature(file_create_new)]
#![allow(non_snake_case)]

use anyhow::Result;
use rglua::prelude::*;

mod detour;
mod error;

#[macro_export]
macro_rules! log {
    ($state:expr, $fmt:expr, $( $arg:expr ),*) => {
        printgm!($state, concat!("accelerator: ", $fmt), $( $arg ),*)
    };
    ($state:expr, $fmt:expr) => {
        printgm!($state, concat!("accelerator: ", $fmt))
    };
}

#[gmod_open]
unsafe fn open(state: LuaState) -> Result<i32> {
    log!(state, "loading...");

    unsafe { detour::apply(state) };

    Ok(0)
}

#[gmod_close]
unsafe fn close(state: LuaState) -> Result<i32> {
    log!(state, "unloading...");

    unsafe { detour::revert(state) };

    Ok(0)
}
