use std::ffi::{c_char, c_void, CStr, OsString};
use std::fs::File;
use std::io::{copy, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use anyhow::Result;
use gmod::detour::GenericDetour;
use rglua::prelude::*;

use crate::error::AcceleratorError;
use crate::log;

static VALVE_USER_AGENT: &str = "Half-Life 2";
static VALVE_REFERER: &str = "hl2://accelerator";

static mut GET_DOWNLOAD_QUEUE_SIZE_DETOUR: Option<GenericDetour<GetDownloadQueueSize>> = None;
static mut QUEUE_DOWNLOAD_DETOUR: Option<GenericDetour<QueueDownload>> = None;
static mut DOWNLOAD_UPDATE_DETOUR: Option<GenericDetour<DownloadUpdate>> = None;

struct DownloadState {
    lua: LuaState,
    handles: Vec<JoinHandle<Result<String>>>,
    timestamp: Option<Instant>,
}

impl DownloadState {
    pub const fn new(state: LuaState) -> Self {
        Self {
            lua: state,
            handles: Vec::new(),
            timestamp: None,
        }
    }
}

static mut STATE: Option<Mutex<DownloadState>> = None;

#[gmod::type_alias(GetDownloadQueueSize)]
unsafe extern "cdecl" fn get_download_queue_size() -> i64 {
    let binding = STATE.as_ref().unwrap();
    let state = &mut binding.lock().unwrap();
    let res: i64 = GET_DOWNLOAD_QUEUE_SIZE_DETOUR.as_ref().unwrap().call();

    res + <usize as TryInto<i64>>::try_into(state.handles.len()).unwrap()
}

#[gmod::type_alias(QueueDownload)]
unsafe extern "cdecl" fn queue_download(
    this: *mut c_void,
    c_url: *const c_char,
    unk0: i32,
    c_path: *const c_char,
    as_http: bool,
    compressed: bool,
    unk3: i32,
) {
    let binding = STATE.as_ref().unwrap();
    let state = &mut binding.lock().unwrap();

    if state.timestamp.is_none() {
        state.timestamp = Some(Instant::now());
    }

    let mut url = CStr::from_ptr(c_url)
        .to_str()
        .unwrap_or_default()
        .to_string();

    if url.ends_with('/') {
        url.pop();
    }

    // dispatch to netchan if no url or as_http is false
    if url.is_empty() || !as_http {
        log!(
            state.lua,
            "calling original... URL.IS_EMPTY={:?} !AS_HTTP={:?}",
            url.is_empty(),
            !as_http
        );

        return QUEUE_DOWNLOAD_DETOUR
            .as_ref()
            .unwrap()
            .call(this, c_url, unk0, c_path, as_http, compressed, unk3);
    }

    let game_path = CStr::from_ptr(c_path)
        .to_str()
        .unwrap_or_default()
        .replace('\\', "/");

    let mut path = PathBuf::from_str(&game_path).unwrap();
    if compressed {
        let mut os: OsString = path.into();
        os.push(".bz2");
        path = os.into();
    }

    log!(state.lua, "dispatching `{}`", path.display());
    let handle: JoinHandle<Result<String>> = thread::spawn(move || {
        let url = format!("{}/{}", url, path.to_str().unwrap_or_default());
        let mut content = ureq::get(&url)
            .set("User-Agent", VALVE_USER_AGENT)
            .set("Referer", VALVE_REFERER)
            .call()?
            .into_reader();

        let file_path = Path::new("garrysmod/download").join(path.with_extension(""));
        std::fs::create_dir_all(file_path.parent().unwrap_or_else(|| return Path::new("")))?;
        let mut dest = File::create_new(file_path)?;

        let mut reader: Box<dyn Read> = if compressed {
            use bzip2::read::BzDecoder;
            Box::new(BzDecoder::new(&mut content))
        } else {
            Box::new(content)
        };

        copy(&mut reader, &mut dest)?;
        Ok(path.to_str().unwrap().to_string())
    });

    state.handles.push(handle);
}

#[gmod::type_alias(DownloadUpdate)]
unsafe extern "cdecl" fn download_update() -> bool {
    let binding = STATE.as_ref().unwrap();
    let mut state = binding.lock().unwrap();

    if !state.handles.is_empty() {
        while let Some(handle) = state.handles.pop() {
            let file = handle.join().unwrap();

            match file {
                Ok(file) => log!(state.lua, "download finished: `{}`", file),
                Err(e) => log!(state.lua, "download failed: {}", e),
            }
        }

        log!(state.lua, "finished!");
        if let Some(timestamp) = state.timestamp {
            log!(state.lua, "elapsed: `{:?}`", timestamp.elapsed());
            state.timestamp = None;
            drop(state);
        }
    }

    return DOWNLOAD_UPDATE_DETOUR.as_ref().unwrap().call();
}

pub unsafe fn apply(lua: LuaState) -> Result<()> {
    log!(lua, "applying detours");

    let state = DownloadState::new(lua);

    let (_lib, path) = if cfg!(all(target_os = "linux", target_pointer_width = "64")) {
        gmod::open_library!("engine_client")?
    } else {
        gmod::open_library!("engine")?
    };

    macro_rules! detour_fn {
        ($func:ident, $type_alias:ty, ($library:ident, $path:ident) -> $pattern:tt) => {
            let $func = {
                let addr = gmod::find_gmod_signature!(($library, $path) -> $pattern)
                    .ok_or(AcceleratorError::SigNotFound(stringify!($type_alias).to_string()))?;

                let detour = GenericDetour::new::<$type_alias>(addr, $func)?;
                detour.enable()?;
                detour
            };
        }
    }

    detour_fn!(get_download_queue_size, GetDownloadQueueSize, (_lib, path) -> {
        win64_x86_64: [@SIG = "48 83 ec 28 48 8b 0d ?? ?? ?? ?? 48 8b 01 ff 50 58 48 8b c8 48 8b 10 ff 52 10"],
        win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        linux64_x86_64: [@SIG = "55 48 89 e5 53 48 83 ec 08 48 8b 05 ?? ?? ?? ?? 8b 1d"],
        linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        win32: [@SIG = "8b 0d ?? ?? ?? ?? 56 8b 01 ff 50 2c 8b 35 ?? ?? ?? ?? 8b c8 8b 10 ff 52 08 03 c6 5e c3"], // untested
        linux32: [@SIG = "55 89 e5 53 83 ec 14 8b 15 60 ?? ?? ?? 8b 1d"],
    });

    detour_fn!(download_update, DownloadUpdate, (_lib, path) -> {
        win64_x86_64: [@SIG = "48 83 ec 28 48 8b 0d ?? ?? ?? ?? 48 8b 01 ff 50 58 48 8b c8 48 8b 10 ff 52 08"],
        win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        linux64_x86_64: [@SIG = "55 48 8d 3d ?? ?? ?? ?? 48 89 e5 5d e9 9f ff ff ff 90 90 90"],
        linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        win32: [@SIG = "55 8b ec 5d e9 87 05 00 00"], // untested
        linux32: [@SIG = "55 89 e5 83 ec 18 c7 04 24 ?? ?? ?? ?? e8 9e ff ff ff c9 c3"],
    });

    detour_fn!(queue_download, QueueDownload, (_lib, path) -> {
        win64_x86_64: [@SIG = "40 53 55 56 57 41 54 41 55 41 56 41 57 48 81 ec 78 02 00 00"],
        win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        linux64_x86_64: [@SIG = "55 48 89 e5 41 57 49 89 cf 41 56 41 55 45 89 cd"],
        linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        win32: [@SIG = "55 8b ec 51 83 3d ?? ?? ?? ?? 01 0f 8e 8f 01 00 00 8b 0d ?? ?? ?? ?? 53 8b 01 ff 50 2c 8b 5d"], // untested
        linux32: [@SIG = "55 89 e5 57 56 53 81 ec 5c 02 00 00 8b 45 0c 8b 5d 08 8b 7d 1c"],
    });

    GET_DOWNLOAD_QUEUE_SIZE_DETOUR = Some(get_download_queue_size);
    QUEUE_DOWNLOAD_DETOUR = Some(queue_download);
    DOWNLOAD_UPDATE_DETOUR = Some(download_update);

    STATE = Some(Mutex::new(state));

    Ok(())
}

pub unsafe fn revert(lua: LuaState) {
    log!(lua, "reverting detours");

    GET_DOWNLOAD_QUEUE_SIZE_DETOUR.take();
    QUEUE_DOWNLOAD_DETOUR.take();
    DOWNLOAD_UPDATE_DETOUR.take();

    STATE.take();
}
