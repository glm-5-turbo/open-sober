//! egl-wrapper: thin `libEGL.so` cdylib that forwards every EGL entry point to
//! Mesa's real `libEGL.so.1` (GRAPHICS_RECOMMENDATION Phase 1 / §3.3).
//!
//! Why a shim at all: under the binary-translation layer the Android `libroblox.so`
//! resolves the sonames `libEGL.so` / `libGLESv2.so`. On desktop those are
//! `libEGL.so.1` / `libGLESv2.so.2`. This crate provides the exact soname the guest
//! expects and forwards each call into Mesa, so the EGL/GLES call path is unchanged
//! (Mesa+zink/llvmpipe does the real rendering headlessly or on a GPU host).

pub mod dl;
pub mod generated_egl;

pub use generated_egl::*;