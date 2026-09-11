//! Compressed-texture interception (GRAPHICS_RECOMMENDATION section 4).
//!
//! Roblox uploads Android compressed-texture formats (ETC1/ETC2/EAC/ASTC) that
//! desktop OpenGL/zink does not necessarily support natively (ASTC especially is
//! unsupported on NVIDIA). We trap `glCompressedTexImage2D` /
//! `glCompressedTexSubImage2D` in the GLES path, decompress Android-specific
//! formats to BGRA with the pure-Rust `texture2ddecoder`, and upload as
//! uncompressed `GL_RGBA8`. Audio/GUI formats that are not Android-specific
//! (BCn/S3TC, RGTC) are passed straight through to Mesa.
//!
//! This crate is shared by:
//!   * `glesv2-wrapper` — the `libGLESv2.so` cdylib (guest sees it when the
//!     Android binary links/dlsyms GLES directly), and
//!   * `arm64jit` — the in-process JIT resolver, whose float/mixed `HostGlesCall`
//!     bridge routes compressed-texture calls here too, so both translation paths
//!     decode Android uploads identically (a JIT path that fell through to raw
//!     Mesa would upload undecodable ETC2/ASTC on a host whose desktop GL can't
//!     natively decode them — e.g. ASTC on NVIDIA).

use std::ffi::c_void;

/// Texture format enum values (from GLES2/GLES3 headers, verified above).
pub const GL_ETC1_RGB8_OES: u32 = 0x8D64;
pub const GL_COMPRESSED_R11_EAC: u32 = 0x9270;
pub const GL_COMPRESSED_SIGNED_R11_EAC: u32 = 0x9271;
pub const GL_COMPRESSED_RG11_EAC: u32 = 0x9272;
pub const GL_COMPRESSED_SIGNED_RG11_EAC: u32 = 0x9273;
pub const GL_COMPRESSED_RGB8_ETC2: u32 = 0x9274;
pub const GL_COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2: u32 = 0x9276;
pub const GL_COMPRESSED_RGBA8_ETC2_EAC: u32 = 0x9278;
pub const GL_ATC_RGB_AMD: u32 = 0x8C92;
pub const GL_ATC_RGBA_AMD: u32 = 0x8C93;

pub const GL_RGBA: u32 = 0x1908;
pub const GL_UNSIGNED_BYTE: u32 = 0x1401;
pub const GL_TEXTURE_2D: u32 = 0x0DE1;
pub const GL_RGBA8: u32 = 0x8058;

/// ASTC block footprint (block_width, block_height) for format value 0x93B0..0x93BD.
/// Index = format - 0x93B0.
pub const ASTC_BLOCK_SIZES: [(u32, u32); 14] = [
    (4, 4), (5, 4), (5, 5), (6, 5), (6, 6), (8, 5), (8, 6),
    (8, 8), (10, 5), (10, 6), (10, 8), (10, 10), (12, 10), (12, 12),
];

/// True if `internalformat` is an Android-specific compressed format we can decode.
pub fn is_android_format(internalformat: u32) -> bool {
    matches!(
        internalformat,
        GL_ETC1_RGB8_OES
            | GL_COMPRESSED_R11_EAC
            | GL_COMPRESSED_SIGNED_R11_EAC
            | GL_COMPRESSED_RG11_EAC
            | GL_COMPRESSED_SIGNED_RG11_EAC
            | GL_COMPRESSED_RGB8_ETC2
            | GL_COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2
            | GL_COMPRESSED_RGBA8_ETC2_EAC
            | GL_ATC_RGB_AMD
            | GL_ATC_RGBA_AMD
    ) || (0x93B0..=0x93BD).contains(&internalformat)
}

/// Decompress `data` (Android compressed `internalformat`) into a BGRA pixel buffer.
/// Returns the raw little-endian RGBA-in-memory byte buffer (u32 per pixel, memory
/// order = BGRA because texture2ddecoder writes `from_le_bytes([b,g,r,a])`).
/// `None` if the format is not handled (caller falls through to Mesa).
pub fn decompress(
    internalformat: u32,
    width: usize,
    height: usize,
    data: &[u8],
) -> Option<Vec<u32>> {
    let mut out = vec![0u32; width * height];
    let res = match internalformat {
        GL_ETC1_RGB8_OES => texture2ddecoder::decode_etc1(data, width, height, &mut out),
        GL_COMPRESSED_RGB8_ETC2 => {
            texture2ddecoder::decode_etc2_rgb(data, width, height, &mut out)
        }
        GL_COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2 => {
            texture2ddecoder::decode_etc2_rgba1(data, width, height, &mut out)
        }
        GL_COMPRESSED_RGBA8_ETC2_EAC => {
            texture2ddecoder::decode_etc2_rgba8(data, width, height, &mut out)
        }
        GL_COMPRESSED_R11_EAC => {
            texture2ddecoder::decode_eacr(data, width, height, &mut out)
        }
        GL_COMPRESSED_SIGNED_R11_EAC => {
            texture2ddecoder::decode_eacr_signed(data, width, height, &mut out)
        }
        GL_COMPRESSED_RG11_EAC => {
            texture2ddecoder::decode_eacrg(data, width, height, &mut out)
        }
        GL_COMPRESSED_SIGNED_RG11_EAC => {
            texture2ddecoder::decode_eacrg_signed(data, width, height, &mut out)
        }
        GL_ATC_RGB_AMD => texture2ddecoder::decode_atc_rgb4(data, width, height, &mut out),
        GL_ATC_RGBA_AMD => {
            texture2ddecoder::decode_atc_rgba8(data, width, height, &mut out)
        }
        _ if (0x93B0..=0x93BD).contains(&internalformat) => {
            let (bw, bh) = ASTC_BLOCK_SIZES[(internalformat - 0x93B0) as usize];
            texture2ddecoder::decode_astc(data, width, height, bw as usize, bh as usize, &mut out)
        }
        _ => return None,
    };
    if res.is_err() {
        return None;
    }
    Some(out)
}

/// Swizzle a `&mut [u32]` buffer from BGRA-in-memory (texture2ddecoder output:
/// memory bytes [b,g,r,a] per pixel) to RGBA-in-memory (memory bytes [r,g,b,a]).
/// Desktop/GLES `glTexImage2D` with `GL_RGBA`/`GL_UNSIGNED_BYTE` needs RGBA order.
pub fn bgra_to_rgba(pixels: &mut [u32]) {
    for px in pixels.iter_mut() {
        let v = *px; // v = b | g<<8 | r<<16 | a<<24
        let b = v & 0xFF;
        let g = (v >> 8) & 0xFF;
        let r = (v >> 16) & 0xFF;
        let a = (v >> 24) & 0xFF;
        *px = r | (g << 8) | (b << 16) | (a << 24);
    }
}

/// Intercept one glCompressedTexImage2D call. Returns `Some` if we handled it
/// (uploaded a decompressed RGBA texture through the real `glTexImage2D`);
/// `None` if the guest should fall through to Mesa's own glCompressedTexImage2D.
///
/// # Safety
/// `data` must point to `image_size` readable bytes if non-null; `real_tex_image2d`
/// must be the genuine Mesa glTexImage2D.
pub unsafe fn handle_compressed_tex_image_2d(
    target: u32,
    level: i32,
    internalformat: u32,
    width: i32,
    height: i32,
    border: i32,
    image_size: i32,
    data: *const c_void,
    real_tex_image2d: unsafe extern "C" fn(
        u32, i32, i32, i32, i32, i32, u32, u32, *const c_void,
    ) -> (),
) -> bool {
    if !is_android_format(internalformat) {
        return false;
    }
    if width <= 0 || height <= 0 || data.is_null() || image_size <= 0 {
        // Be safe: let Mesa reject invalid uploads itself.
        return false;
    }
    let w = width as usize;
    let h = height as usize;
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, image_size as usize) };
    let Some(mut pixels) = decompress(internalformat, w, h, slice) else {
        return false;
    };
    bgra_to_rgba(&mut pixels);
    let rgab = pixels.as_ptr() as *const c_void;
    unsafe {
        real_tex_image2d(
            target, level, GL_RGBA8 as i32, width, height, border, GL_RGBA, GL_UNSIGNED_BYTE, rgab,
        );
    }
    true
}

/// Intercept one glCompressedTexSubImage2D call for an Android format: decompress
/// the whole `width x height` sub-rect to RGBA and upload it via the real
/// `glTexSubImage2D`. Returns `true` if handled; `false` means the caller should
/// fall through to Mesa's own glCompressedTexSubImage2D.
///
/// # Safety
/// `data` must point to `image_size` readable bytes if non-null; `real_tex_sub_image2d`
/// must be the genuine Mesa glTexSubImage2D.
pub unsafe fn handle_compressed_tex_sub_image_2d(
    target: u32,
    level: i32,
    xoffset: i32,
    yoffset: i32,
    width: i32,
    height: i32,
    format: u32,
    image_size: i32,
    data: *const c_void,
    real_tex_sub_image2d: unsafe extern "C" fn(
        u32, i32, i32, i32, i32, i32, u32, u32, *const c_void,
    ) -> (),
) -> bool {
    if !is_android_format(format) || width <= 0 || height <= 0 || data.is_null() || image_size <= 0 {
        return false;
    }
    let w = width as usize;
    let h = height as usize;
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, image_size as usize) };
    let Some(mut pixels) = decompress(format, w, h, slice) else {
        return false;
    };
    bgra_to_rgba(&mut pixels);
    let rgab = pixels.as_ptr() as *const c_void;
    unsafe {
        real_tex_sub_image2d(
            target, level, xoffset, yoffset, width, height, GL_RGBA, GL_UNSIGNED_BYTE, rgab,
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_format_detection() {
        assert!(is_android_format(GL_ETC1_RGB8_OES));
        assert!(is_android_format(GL_COMPRESSED_RGBA8_ETC2_EAC));
        assert!(is_android_format(0x93B0)); // ASTC 4x4
        assert!(is_android_format(0x93BD)); // ASTC 12x12
        // Desktop-only BCn / S3TC / RGTC are NOT Android-intercepts; Mesa handles them.
        assert!(!is_android_format(0x83F1)); // GL_COMPRESSED_RGB_S3TC_DXT1_EXT
        assert!(!is_android_format(0x9270 - 1)); // 0x926f, undefined
        assert!(!is_android_format(0x93BE)); // past ASTC range (0x93BE not android)
    }

    #[test]
    fn decodes_etc2_rgb_to_correct_pixel_buffer_size() {
        // A 4x4 ETC2 RGB block = 8 bytes (one 4x4 block).
        let data = [0u8; 8];
        let px = decompress(GL_COMPRESSED_RGB8_ETC2, 4, 4, &data);
        assert!(px.is_some(), "4x4 ETC2 RGB should decode");
        assert_eq!(px.unwrap().len(), 16);
    }

    #[test]
    fn etc2_rgba8_and_astc_decode() {
        // RGBA8_EAC 4x4 = 16 bytes (8 ETC2 + 8 EAC alpha).
        let px = decompress(GL_COMPRESSED_RGBA8_ETC2_EAC, 4, 4, &[0u8; 16]);
        assert_eq!(px.map(|v| v.len()), Some(16));
        // ASTC 4x4 = 16 bytes per block, 1 block for 4x4.
        let px = decompress(0x93B0, 4, 4, &[0u8; 16]);
        assert_eq!(px.map(|v| v.len()), Some(16));
    }

    #[test]
    fn non_android_and_unknown_formats_return_none() {
        assert!(decompress(0x83F1, 4, 4, &[0u8; 8]).is_none()); // DXT1 -> Mesa
        assert!(decompress(0xFFFFFFFF, 4, 4, &[0u8; 8]).is_none());
    }

    #[test]
    fn android_block_sizes_table() {
        // 14 ASTC footprints, indexed by format-0x93B0.
        assert_eq!(ASTC_BLOCK_SIZES.len(), 14);
        assert_eq!(ASTC_BLOCK_SIZES[0], (4, 4));
        assert_eq!(ASTC_BLOCK_SIZES[13], (12, 12));
    }

    #[test]
    fn bgra_in_memory_swizzles_to_rgba() {
        // texture2ddecoder writes from_le_bytes([b,g,r,a]); clamp may replicate.
        // A red pixel: b=0,g=0,r=255,a=255 -> memory bytes [0,0,255,255] -> u32 LE.
        let red_bgra = u32::from_le_bytes([0, 0, 255, 255]);
        let mut buf = vec![red_bgra; 4];
        bgra_to_rgba(&mut buf);
        // After swizzle memory should be [255,0,0,255].
        for px in &buf {
            assert_eq!(px.to_le_bytes(), [255, 0, 0, 255]);
        }
    }
}