//! glesv2-wrapper: thin `libGLESv2.so` cdylib that forwards every GLES entry point
//! to Mesa's real `libGLESv2.so.2` (GRAPHICS_RECOMMENDATION Phase 1 / §3.3), and
//! intercepts `glCompressedTexImage2D` / `glCompressedTexSubImage2D` for Android
//! compressed texture formats (ETC1/ETC2/EAC/ASTC) by decompressing to RGBA with
//! the shared `texture_codec` crate (§4). Non-Android formats are passed
//! through to Mesa untouched. (texture_codec is also used by the arm64jit GLES
//! bridge so both translation paths decode Android textures identically.)

pub mod dl;
pub mod generated_gles;
pub mod intercept;

// Re-export the shared Android-texture codec as the `texture` module so existing
// `use crate::texture` call sites and the historical public surface stay stable.
pub mod texture {
    pub use texture_codec::*;
}

pub use generated_gles::*;