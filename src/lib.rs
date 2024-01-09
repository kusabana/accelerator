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
fn open(l: LuaState) -> Result<i32> {
    log!(l, "loading...");

    unsafe {
        detour::apply(l)?;
    }

    Ok(0)
}

#[gmod_close]
fn close(l: LuaState) -> Result<i32> {
    log!(l, "unloading...");

    unsafe {
        detour::revert(l);
    }

    Ok(0)
}
