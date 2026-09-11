//! Hand-written GLES entry points that intercept Android compressed textures.
//! These are excluded from `generated_gles.rs` (see gen_forward.py) so each call
//! can try the shared `texture_codec` decode path first and fall through to Mesa
//! otherwise.

use std::ffi::c_void;

use crate::texture as tex; // re-exports texture_codec

macro_rules! real_tex_image2d {
    () => {
        crate::dl::sym::<unsafe extern "C" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)>(
            "libGLESv2.so.2",
            "glTexImage2D",
        )
    };
}
macro_rules! real_tex_sub_image2d {
    () => {
        crate::dl::sym::<unsafe extern "C" fn(
            u32, i32, i32, i32, i32, i32, u32, u32, *const c_void,
        )>(
            "libGLESv2.so.2",
            "glTexSubImage2D",
        )
    };
}
macro_rules! real_compressed_tex_image2d {
    () => {
        crate::dl::sym::<unsafe extern "C" fn(
            u32, i32, u32, i32, i32, i32, i32, *const c_void,
        )>("libGLESv2.so.2", "glCompressedTexImage2D")
    };
}
macro_rules! real_compressed_tex_sub_image2d {
    () => {
        crate::dl::sym::<unsafe extern "C" fn(
            u32, i32, i32, i32, i32, i32, u32, i32, *const c_void,
        )>("libGLESv2.so.2", "glCompressedTexSubImage2D")
    };
}

/// Forwarded glCompressedTexImage2D with Android-format interception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCompressedTexImage2D(
    target: u32,
    level: i32,
    internalformat: u32,
    width: i32,
    height: i32,
    border: i32,
    image_size: i32,
    data: *const c_void,
) {
    if unsafe { tex::handle_compressed_tex_image_2d(
        target,
        level,
        internalformat,
        width,
        height,
        border,
        image_size,
        data,
        real_tex_image2d!(),
    ) } {
        return;
    }
    unsafe { real_compressed_tex_image2d!()(target, level, internalformat, width, height, border, image_size, data) }
}

/// glCompressedTexSubImage2D: for an Android format, decompress the sub-rect
/// into an RGBA buffer sized to (width × height) and upload with glTexSubImage2D;
/// otherwise fall through to Mesa.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn glCompressedTexSubImage2D(
    target: u32,
    level: i32,
    xoffset: i32,
    yoffset: i32,
    width: i32,
    height: i32,
    format: u32,
    image_size: i32,
    data: *const c_void,
) {
    if unsafe { tex::handle_compressed_tex_sub_image_2d(
        target,
        level,
        xoffset,
        yoffset,
        width,
        height,
        format,
        image_size,
        data,
        real_tex_sub_image2d!(),
    ) } {
        return;
    }
    unsafe { real_compressed_tex_sub_image2d!()(target, level, xoffset, yoffset, width, height, format, image_size, data) }
}