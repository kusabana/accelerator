#![feature(c_unwind)]
#![feature(thread_id_value)]
#![feature(file_create_new)]
#![allow(non_snake_case)]

use gmod::gmcl::override_stdout;
use gmod::lua::State;

#[macro_use] extern crate gmod;

mod detour;
mod error;

#[macro_export]
macro_rules! log {
    ($fmt:expr, $( $arg:expr ),*) => {
        println!(concat!("accelerator: ", $fmt), $( $arg ),*)
    };
    ($fmt:expr) => {
        println!(concat!("accelerator: ", $fmt))
    };
}arcadian

#[gmod13_open]
unsafe fn open(_state: State) -> i32 {
    override_stdout();
    
    log!("loading...");

    unsafe { detour::apply() };

    0
}

#[gmod13_close]
unsafe fn close(_state: State) -> i32{
    log!("unloading...");

    unsafe { detour::revert() };
    
    0
}
