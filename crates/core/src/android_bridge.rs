//! JNI boundary: settings and lifecycle policy stay in Rust.

use crate::{engine, settings};
use jni::EnvUnowned;
use jni::errors::{Error, ThrowRuntimeExAndDefault};
use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong};
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

static SETTINGS_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct BridgeError(&'static str);

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
impl std::error::Error for BridgeError {}
impl From<Error> for BridgeError {
    fn from(_: Error) -> Self {
        Self("Android native bridge operation failed.")
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_apiVersion<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    unowned
        .with_env(|_| Ok::<_, Error>(4))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_capabilities<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jint {
    // Bit 0 will indicate an integrated engine. Do not advertise it prematurely.
    unowned
        .with_env(|_| Ok::<_, Error>(i32::from(engine::version().is_some())))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_readiness<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jlong {
    unowned
        .with_env(|_| {
            let state = engine::snapshot().map_err(BridgeError)?;
            Ok::<_, BridgeError>(state.readiness.code() as jlong)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_loadNetworkId<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    directory: JString<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let _guard = SETTINGS_LOCK
                .lock()
                .map_err(|_| BridgeError("Settings unavailable."))?;
            let root = directory.to_string();
            let id = settings::load(Path::new(&root))
                .map_err(|_| BridgeError("Saved settings unavailable."))?;
            JString::from_str(env, id.map(|id| id.to_string()).unwrap_or_default())
                .map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_saveNetworkId<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    directory: JString<'caller>,
    network_id: JString<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let _guard = SETTINGS_LOCK
                .lock()
                .map_err(|_| BridgeError("Settings unavailable."))?;
            let root = directory.to_string();
            let input = network_id.to_string();
            let message = settings::save(Path::new(&root), &input)
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            if message.is_empty() {
                engine::clear_inactive_error();
            }
            JString::from_str(env, message).map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_engineVersion<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| JString::from_str(env, engine::version().unwrap_or_default()))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_startEngine<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    directory: JString<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let root = directory.to_string();
            let error = engine::start(Path::new(&root)).err().unwrap_or("");
            JString::from_str(env, error)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_stopEngine<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| JString::from_str(env, engine::stop().err().unwrap_or("")))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_nodeAddress<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let state = engine::snapshot().map_err(BridgeError)?;
            let address = if state.address == 0 {
                String::new()
            } else {
                format!("{:010x}", state.address)
            };
            JString::from_str(env, address).map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_controlPlaneOnline<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> jni::sys::jboolean {
    unowned
        .with_env(|_| {
            Ok::<_, BridgeError>(
                engine::snapshot()
                    .map_err(BridgeError)?
                    .control_plane_online,
            )
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_engineError<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let state = engine::snapshot().map_err(BridgeError)?;
            JString::from_str(env, state.error.unwrap_or("")).map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_startVpnEngine<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    directory: JString<'caller>,
    service: jni::objects::JObject<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let root = directory.to_string();
            let result = engine::start_protected(Path::new(&root), |fd| {
                env.call_method(
                    &service,
                    jni::jni_str!("protectSocket"),
                    jni::jni_sig!("(I)Z"),
                    &[jni::objects::JValue::Int(fd)],
                )
                .and_then(|value| value.z())
                .unwrap_or(false)
            });
            JString::from_str(env, result.err().unwrap_or(""))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_vpnConfiguration<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let config = engine::configuration().map_err(BridgeError)?;
            JString::from_str(env, config).map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_attachTunnel<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    fd: jint,
    revision: jlong,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let result = if revision > 0 {
                engine::attach_tunnel(fd, revision as u64)
            } else {
                Err("Invalid configuration revision.")
            };
            JString::from_str(env, result.err().unwrap_or(""))
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_vpnFailed<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) {
    unowned
        .with_env(|_| {
            engine::vpn_failed();
            Ok::<_, Error>(())
        })
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_trafficDiagnostics<'caller>(
    mut unowned: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) -> JString<'caller> {
    unowned
        .with_env(|env| {
            let text = engine::diagnostics().map_err(BridgeError)?;
            JString::from_str(env, text).map_err(BridgeError::from)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
