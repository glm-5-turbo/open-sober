//! Hand-written GLES entry points that intercept Android compressed textures.
//! These are excluded from `generated_gles.rs` (see gen_forward.py) so each call
//! can try the texture2ddecoder path first and fall through to Mesa otherwise.

use std::ffi::{c_void, CString};

use crate::texture;

/// `CString` cache of symbol names is unnecessary; we resolve lazily per call via dl.
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
    if unsafe { texture::handle_compressed_tex_image_2d(
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

/// glCompressedTexSubImage2D: for an Android format, decompress the whole sub-rect
/// into an RGBA buffer sized to (width×height) and upload with glTexSubImage2D.
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
    if let Some(pixels) = unsafe { subimage_decompress(format, width, height, data, image_size) } {
        let real = real_tex_sub_image2d!();
        unsafe {
            real(
                target,
                level,
                xoffset,
                yoffset,
                width,
                height,
                texture::GL_RGBA,
                texture::GL_UNSIGNED_BYTE,
                pixels.as_ptr() as *const c_void,
            );
        }
        return;
    }
    unsafe { real_compressed_tex_sub_image2d!()(target, level, xoffset, yoffset, width, height, format, image_size, data) }
}

/// Decompress a compressed sub-image into an RGBA buffer sized width×height.
unsafe fn subimage_decompress(
    format: u32,
    width: i32,
    height: i32,
    data: *const c_void,
    image_size: i32,
) -> Option<Vec<u32>> {
    if !texture::is_android_format(format) || width <= 0 || height <= 0 || data.is_null() || image_size <= 0 {
        return None;
    }
    let w = width as usize;
    let h = height as usize;
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, image_size as usize) };
    let mut px = texture::decompress(format, w, h, slice)?;
    texture::bgra_to_rgba(&mut px);
    Some(px)
}

#[allow(dead_code)]
fn _keep_cstring(_c: CString) {}