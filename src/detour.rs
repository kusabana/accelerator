use std::{ffi::CStr, os::raw::c_char};
use std::sync::Mutex;
use std::ops::Deref;

use rglua::prelude::*;
use anyhow::Result;
use gmod::{type_alias, open_library, find_gmod_signature};
use viable::vtable;

use crate::error::AcceleratorError;
use crate::log;

static mut CHECK_UPDATING_DETOUR: Option<gmod::detour::GenericDetour<CheckUpdatingSteamResources>> = None;
static mut STATE: Option<Mutex<LuaState>> = None;


pub struct CClientState;

#[vtable]
pub struct INetworkStringTable {
    #[offset(4)]
    get_size: extern "C" fn() -> i32,
    #[offset(10)]
    get_entry: extern "C" fn(index: i32) -> *const c_char,
}

impl INetworkStringTable {
    /// helper to get table entries as a Vec<String>
    pub unsafe fn entries(&mut self) -> Vec<String> {
        (0..self.get_size())
            .into_iter()
            .map(|i| self.get_entry(i))
            .map(|c| CStr::from_ptr(c).to_string_lossy().to_string())
            .collect()
    }
}

#[cfg_attr(all(target_os = "windows", target_pointer_width = "64"), abi("fastcall"))]
#[cfg_attr(all(target_os = "windows", target_pointer_width = "32"), abi("stdcall"))]
#[type_alias(CheckUpdatingSteamResources)]
unsafe extern "cdecl" fn CheckUpdatingSteamResources_detour(this: *mut CClientState) {
    let binding = STATE.as_ref().unwrap();
    let state = *binding.lock().unwrap().deref();

    let list_ptr = this
        .cast::<u8>()
        .offset(135248)
        .cast::<*mut INetworkStringTable>()
        .read();

    if let Some(downloadables) = list_ptr.as_mut() {
        for downloadable in downloadables.entries() {
            log!(state, "queue", "dispatching downloadable: {}", downloadable);

            // handle fastdl
            // handle .gma's
        }

        // ^ above should block until done
        // call finishsignonstate_new
    }
}

pub unsafe fn apply(state: LuaState) -> Result<()> {
    STATE = Some(Mutex::new(state));
	let (_lib, _path) = open_library!("engine_client")?;
    
	let CheckUpdatingSteamResources = find_gmod_signature!((_lib, _path) -> {
		win64_x86_64: [@SIG = "40 55 53 56 57 41 54 41 56 41 57 48 8D AC 24 ? ? ? ? 48 81 EC ? ? ? ? 48 8B 05 ? ? ? ? 48 33 C4 48 89 85 ? ? ? ? 49 8B F1 4D 8B F8 4C 8B F2 48 8B F9 4D 85 C9 0F 84"],
		win32_x86_64: [@SIG = "55 8B EC 81 EC ? ? ? ? 56 57 8B 7D 10 8B F1 85 FF 0F 84 ? ? ? ? 57 E8 ? ? ? ? 83 C4 04 83 F8 01 0F 8C ? ? ? ? 80 3F 1B 75 35 8B 06 6A 1B 68 ? ? ? ? 56 FF 90 ? ? ? ?"],

        linux64_x86_64: [@SIG = "55 48 89 e5 41 57 41 56 41 55 41 54 49 89 fc 53 48 83 ec 38 64 48 8b"],
		linux32_x86_64: [@SIG = "55 89 E5 57 56 53 81 EC ? ? ? ? 8B 45 18 8B 55 14 8B 5D 08 8B 7D 0C 89 85 ? ? ? ? 8B 45 1C 8B 75 10 89 85 ? ? ? ? 8B 45 20 89 85 ? ? ? ? 8B 45 24 89 85 ? ? ? ? 65 A1 ? ? ? ? 89 45 E4 31 C0 85 D2 0F 84 ? ? ? ? 89 14 24 89 95"],

		win32: [@SIG = "55 8B EC 8B 55 10 81 EC ? ? ? ? 56 8B F1 57 85 D2 0F 84 ? ? ? ? 8B CA 8D 79 01 8D 49 00 8A 01 41 84 C0 75 F9 2B CF 83 F9 01 0F 8C ? ? ? ? 80 3A 1B 75 35"],
		linux32: [@SIG = "55 89 E5 57 56 53 81 EC ? ? ? ? 8B 45 18 8B 55 14 8B 5D 08 8B 7D 0C 89 85 ? ? ? ? 8B 45 1C 8B 75 10 89 85 ? ? ? ? 8B 45 20 89 85 ? ? ? ? 8B 45 24 89 85 ? ? ? ? 65 A1 ? ? ? ? 89 45 E4 31 C0 85 D2 0F 84 ? ? ? ? 89 14 24 89 95 ? ? ? ?"],
	}).ok_or(AcceleratorError::SigScanError("CheckUpdatingSteamResources"))?;
    let check_updating_detour = gmod::detour::GenericDetour::new::<CheckUpdatingSteamResources>(CheckUpdatingSteamResources, CheckUpdatingSteamResources_detour)?;
	check_updating_detour.enable()?;

    CHECK_UPDATING_DETOUR = Some(check_updating_detour);

    Ok(())
}

pub unsafe fn revert(_state: LuaState) -> Result<()> {
    CHECK_UPDATING_DETOUR.take();
    STATE.take();

    Ok(())
}