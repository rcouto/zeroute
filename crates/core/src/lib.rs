//! Rust application boundary for Zeroute.
//!
//! This bootstrap exports only an ABI handshake. ZeroTier, VPN lifecycle,
//! networking and persistence are not implemented yet.

use core::ffi::c_void;

// SAFETY: this uniquely named JNI symbol matches NativeBridge.apiVersion.
// Neither JNI pointer is dereferenced, retained or shared with another thread.
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_rcouto_zeroute_NativeBridge_apiVersion(
    _environment: *mut c_void,
    _class: *mut c_void,
) -> i32 {
    1
}
