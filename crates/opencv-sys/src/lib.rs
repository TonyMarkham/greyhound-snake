//! The ghcv shim's Rust-side ABI pin.
//!
//! The shim itself is a plain C++ DLL that this crate's build script
//! compiles (see `build.rs` and `cpp/detect.cpp`) — the same arrangement
//! as `occt-sys`. The ABI is flat and version-gated: encoded image bytes
//! in, detected marker ids out; no OpenCV type crosses the boundary and
//! the shim holds no state. The C# side of this bite (tools/ArucoIdentify)
//! binds the DLL directly through P/Invoke; the constant here pins the
//! version the Rust side expects, so a future consumer change is caught
//! at compile time against the same value the C++ publishes.

/// The ABI version `ghcv_abi_version()` reports; must match the value in
/// `cpp/detect.cpp`.
pub const GHCV_ABI_VERSION: i32 = 1;
