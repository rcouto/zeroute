//! Rust-owned settings, readiness policy, and the Android JNI boundary.
//!
//! The Android adapter supplies protected transport sockets and a TUN descriptor.

mod android_bridge;
pub mod engine;
pub mod readiness;
pub mod settings;

pub mod network;
pub mod packet;

pub mod session;
