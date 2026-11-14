//! Engine ownership and observations. An integrated core is not an Android tunnel.

use crate::network::Configuration;
use crate::readiness::{Connection, Readiness};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

#[cfg(all(zerotier_core, target_os = "android"))]
mod native;

struct State {
    connection: Connection,
    address: u64,
    error: Option<&'static str>,
    configuration: Option<(u64, Configuration)>,
    revision: u64,
    diagnostics: String,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        connection: Connection::default(),
        address: 0,
        error: None,
        configuration: None,
        revision: 0,
        diagnostics: String::new(),
    })
});

pub struct Snapshot {
    pub readiness: Readiness,
    pub control_plane_online: bool,
    pub address: u64,
    pub error: Option<&'static str>,
}

pub fn snapshot() -> Result<Snapshot, &'static str> {
    let state = STATE.lock().map_err(|_| "Connection state unavailable.")?;
    Ok(Snapshot {
        readiness: state.connection.readiness(),
        control_plane_online: state.connection.control_plane_online(),
        address: state.address,
        error: state.error,
    })
}

pub fn clear_inactive_error() {
    if let Ok(mut state) = STATE.lock()
        && state.connection.readiness() == Readiness::Stopped
    {
        state.error = None;
    }
}

pub fn version() -> Option<String> {
    #[cfg(all(zerotier_core, target_os = "android"))]
    return Some(native::version());
    #[cfg(not(all(zerotier_core, target_os = "android")))]
    None
}

pub fn start(root: &Path) -> Result<(), &'static str> {
    #[cfg(all(zerotier_core, target_os = "android"))]
    return native::start(root, &mut |_| true);
    #[cfg(not(all(zerotier_core, target_os = "android")))]
    {
        let _ = root;
        Err("The ZeroTier core is not included in this build.")
    }
}

pub fn stop() -> Result<(), &'static str> {
    #[cfg(all(zerotier_core, target_os = "android"))]
    return native::stop();
    #[cfg(not(all(zerotier_core, target_os = "android")))]
    Ok(())
}

pub fn configuration() -> Result<String, &'static str> {
    let state = STATE.lock().map_err(|_| "Connection state unavailable.")?;
    Ok(state
        .configuration
        .as_ref()
        .map(|(_, config)| config.android_text(state.revision))
        .unwrap_or_default())
}
pub fn start_protected(
    root: &Path,
    mut protect: impl FnMut(i32) -> bool,
) -> Result<(), &'static str> {
    #[cfg(all(zerotier_core, target_os = "android"))]
    return native::start(root, &mut protect);
    #[cfg(not(all(zerotier_core, target_os = "android")))]
    {
        let _ = (&mut protect, root);
        Err("The ZeroTier core is not included in this build.")
    }
}
pub fn attach_tunnel(fd: i32, revision: u64) -> Result<(), &'static str> {
    #[cfg(all(zerotier_core, target_os = "android"))]
    return native::attach_tunnel(fd, revision);
    #[cfg(not(all(zerotier_core, target_os = "android")))]
    {
        let _ = (fd, revision);
        Err("The Android tunnel backend is unavailable.")
    }
}
pub fn vpn_failed() {
    let _ = stop();
    if let Ok(mut state) = STATE.lock() {
        state.error = Some("Android could not establish the VPN.");
    }
}

pub fn diagnostics() -> Result<String, &'static str> {
    Ok(STATE
        .lock()
        .map_err(|_| "Connection state unavailable.")?
        .diagnostics
        .clone())
}
