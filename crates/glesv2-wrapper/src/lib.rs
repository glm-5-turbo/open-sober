//! glesv2-wrapper: thin `libGLESv2.so` cdylib that forwards every GLES entry point
//! to Mesa's real `libGLESv2.so.2` (GRAPHICS_RECOMMENDATION Phase 1 / §3.3), and
//! intercepts `glCompressedTexImage2D` / `glCompressedTexSubImage2D` for Android
//! compressed texture formats (ETC1/ETC2/EAC/ASTC) by decompressing to RGBA with
//! the pure-Rust `texture2ddecoder` crate (§4). Non-Android formats are passed
//! through to Mesa untouched.

pub mod dl;
pub mod generated_gles;
pub mod intercept;
pub mod texture;

pub use generated_gles::*;