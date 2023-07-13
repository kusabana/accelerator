use anyhow::Result;
use bzip2_rs::decoder::DecoderReader;
use gmod::{find_gmod_signature, open_library, type_alias};
use reqwest::StatusCode;
use rglua::prelude::*;
use std::ffi::OsString;
use std::fs::File;
use std::io::copy;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;
use std::{
    ffi::{c_void, CStr},
    os::raw::c_char,
};

use crate::error::AcceleratorError;
use crate::log;

static mut GET_DOWNLOAD_QUEUE_SIZE_DETOUR: Option<
    gmod::detour::GenericDetour<GetDownloadQueueSize>,
> = None;
static mut QUEUE_DOWNLOAD_DETOUR: Option<gmod::detour::GenericDetour<QueueDownload>> = None;
static mut DOWNLOAD_UPDATE_DETOUR: Option<gmod::detour::GenericDetour<DownloadUpdate>> = None;

struct DownloadState {
    lua: LuaState,
    handles: Vec<JoinHandle<Result<String>>>,
    timestamp: Option<Instant>,
}

impl DownloadState {
    pub const fn new(state: LuaState) -> Self {
        return Self {
            lua: state,
            handles: Vec::new(),
            timestamp: None,
        }
    }
}

static mut STATE: Option<Mutex<DownloadState>> = None;

#[type_alias(GetDownloadQueueSize)]
unsafe extern "cdecl" fn GetDownloadQueueSize_detour() -> i64 {
    let binding = STATE.as_ref().unwrap();
    let state = &mut binding.lock().unwrap();
    let res: i64 = GET_DOWNLOAD_QUEUE_SIZE_DETOUR.as_ref().unwrap().call();

    return res + <usize as TryInto<i64>>::try_into(state.handles.len()).unwrap()
}

#[type_alias(QueueDownload)]
unsafe extern "cdecl" fn QueueDownload_detour(
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

    let url = CStr::from_ptr(c_url)
        .to_str()
        .unwrap_or_default()
        .to_string();

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
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(format!("{}/{}", url, path.to_str().unwrap_or_default(),))
            .send()?;

        if response.status() == StatusCode::OK {
            let mut content = Cursor::new(response.bytes()?);

            let file_path = Path::new("garrysmod/download").join(path.with_extension(""));
            std::fs::create_dir_all(file_path.parent().unwrap_or_else(|| return Path::new("")))?;
            let mut dest = File::create_new(file_path)?;

            let _ = if compressed {
                copy(&mut DecoderReader::new(&mut content), &mut dest)
            } else {
                copy(&mut content, &mut dest)
            };

            return Ok(path.to_str().unwrap().to_string());
        }

        return Err(AcceleratorError::RemoteFileNotFound(path.display().to_string(), url).into())
    });
    state.handles.push(handle);
}

#[type_alias(DownloadUpdate)]
unsafe extern "cdecl" fn DownloadUpdate_detour() -> bool {
    let binding = STATE.as_ref().unwrap();
    let mut state = binding.lock().unwrap();

    if !state.handles.is_empty() {
        while let Some(handle) = state.handles.pop() {
            let file = handle.join().unwrap();

            match file {
                Ok(file) => log!(state.lua, "finished `{}`", file),
                Err(e) => log!(state.lua, "caught error: {}", e),
            }
        }

        log!(state.lua, "finished!");
        if let Some(timestamp) = state.timestamp {
            log!(state.lua, "elapsed: `{:?}`", timestamp.elapsed());
            state.timestamp = None;
            drop(state);
        }
    }

    return DOWNLOAD_UPDATE_DETOUR.as_ref().unwrap().call()
}

pub unsafe fn apply(l: LuaState) -> Result<()> {
    log!(l, "applying detours...");

    let state = DownloadState::new(l);

    let (_lib, path) = if cfg!(all(target_os = "linux", target_pointer_width = "64")) {
        open_library!("engine_client")?
    } else {
        open_library!("engine")?
    };

    // most of these sigs aren't very future-proof but i spent hours on learning
    // the binary ninja api to create my script in scripts/ so i'm not going to put it to waste.
    let GetDownloadQueueSize = find_gmod_signature!((_lib, path) -> {
		win64_x86_64: [@SIG = "48 83 ec 28 48 8b 0d 85 5c 32 00 48 8b 01 ff 50 58 48 8b c8 48 8b 10 ff 52 10 03 05 88 ff 33 00 48 83 c4 28 c3"],
		win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

		linux64_x86_64: [@SIG = "55 48 89 e5 53 48 83 ec 08 48 8b 05 ?? ?? ?? ?? 8b 1d"],
		linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        win32: [@SIG = "8b 0d ?? ?? ?? ?? 56 8b 01 ff 50 2c 8b 35 ?? ?? ?? ?? 8b c8 8b 10 ff 52 08 03 c6 5e c3"],
		linux32: [@SIG = "55 89 e5 53 83 ec 14 8b 15 ?? ?? ?? ?? 8b 1d ?? ?? ?? ?? 8b 02 89 14 24"],
	}).ok_or(AcceleratorError::SigNotFound)?;
    let get_download_queue_size_detour = gmod::detour::GenericDetour::new::<GetDownloadQueueSize>(
        GetDownloadQueueSize,
        GetDownloadQueueSize_detour,
    )?;
    get_download_queue_size_detour.enable()?;

    let DownloadUpdate = find_gmod_signature!((_lib, path) -> {
		win64_x86_64: [@SIG = "48 83 ec 28 48 8b 0d ?? ?? ?? ?? 48 8b 01 ff 50 58 48 8b c8 48 8b 10 ff 52 08"],
		win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

		linux64_x86_64: [@SIG = "55 48 8d 3d ?? ?? ?? ?? 48 89 e5 5d e9 9f ff ff ff 90 90 90"],
		linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

		win32: [@SIG = "55 8b ec 5d e9 87 05 00 00"],
		linux32: [@SIG = "55 89 e5 83 ec 18 c7 04 24 ?? ?? ?? ?? e8 9e ff ff ff c9 c3"],
	}).ok_or(AcceleratorError::SigNotFound)?;
    let download_update_detour =
        gmod::detour::GenericDetour::new::<DownloadUpdate>(DownloadUpdate, DownloadUpdate_detour)?;
    download_update_detour.enable()?;

    let QueueDownload = find_gmod_signature!((_lib, path) -> {
		win64_x86_64: [@SIG = "40 53 55 56 57 41 54 41 55 41 56 41 57 48 81 ec 78 02 00 00"],
		win32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

        linux64_x86_64: [@SIG = "55 48 89 e5 41 57 49 89 cf 41 56 41 55 45 89 cd"],
		linux32_x86_64: [@SIG = "00 00"], // open an issue if you need this sig, or find it yourself

		win32: [@SIG = "55 8b ec 51 83 3d ?? ?? ?? ?? 01 0f 8e 8f 01 00 00 8b 0d ?? ?? ?? ?? 53 8b 01 ff 50 2c 8b 5d"],
		linux32: [@SIG = "55 89 e5 57 56 53 81 ec 5c 02 00 00 8b 45 0c 8b 5d 08 8b 7d 1c"],
	}).ok_or(AcceleratorError::SigNotFound)?;
    let queue_download_detour =
        gmod::detour::GenericDetour::new::<QueueDownload>(QueueDownload, QueueDownload_detour)?;
    queue_download_detour.enable()?;

    GET_DOWNLOAD_QUEUE_SIZE_DETOUR = Some(get_download_queue_size_detour);
    QUEUE_DOWNLOAD_DETOUR = Some(queue_download_detour);
    DOWNLOAD_UPDATE_DETOUR = Some(download_update_detour);

    STATE = Some(Mutex::new(state));

    return Ok(())
}

pub unsafe fn revert(l: LuaState) {
    log!(l, "reverting detours...");

    GET_DOWNLOAD_QUEUE_SIZE_DETOUR.take();
    QUEUE_DOWNLOAD_DETOUR.take();
    DOWNLOAD_UPDATE_DETOUR.take();

    STATE.take();
}
