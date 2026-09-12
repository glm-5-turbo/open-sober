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
    fn crafted_etc1_solid_blocks_decode_to_expected_colors() {
        // A hand-crafted ETC1 block: individual mode (diff bit clear), table codeword
        // 0, all 16 selectors 0. data = [R,G,B, 0(cw+diff+flip), 0,0,0,0]. Decode:
        // each byte is split into two 4-bit sub-block base colors (high/low nibble,
        // replicated as c*0x11) and +2 modifier (ETC1_MODIFIER_TABLE[0][0]) applied.
        // So decoded channel = (c*0x11)+2, clamped. This is the exact blob the
        // --renderframe-etc harness uploads (decoded live to 255,2,2 red / 2,255,2
        // green / 2,2,255 blue / 255,255,255 white).
        let enc = |t: i32| -> u8 { let c = ((t - 2).clamp(0, 240) >> 4) as u8; (c << 4) | c };
        let blk = |r: u8, g: u8, b: u8| -> [u8; 8] { [r, g, b, 0, 0, 0, 0, 0] };
        let red = blk(enc(255), enc(2), enc(2));
        let mut out = [0u32; 16];
        texture2ddecoder::decode_etc1(&red, 4, 4, &mut out).unwrap();
        // every texel identical (solid block); BGRA-in-memory => from_le_bytes([b,g,r,a])
        assert!(out.iter().all(|&p| p == out[0]));
        assert_eq!(out[0], u32::from_le_bytes([2, 2, 255, 255]), "red ETC1 block -> (255,2,2)");

        // Whole 8x8 (4 blocks, row-major top-first) via the crate's own decompress.
        let etc_data: Vec<u8> = {
            let mut d = Vec::with_capacity(32);
            d.extend_from_slice(&red);
            d.extend_from_slice(&blk(enc(2), enc(255), enc(2))); // green
            d.extend_from_slice(&blk(enc(2), enc(2), enc(255))); // blue
            d.extend_from_slice(&blk(enc(255), enc(255), enc(255))); // white
            d
        };
        let px = decompress(GL_ETC1_RGB8_OES, 8, 8, &etc_data).expect("8x8 ETC1 decodes");
        assert_eq!(px.len(), 64);
        let col = |i: usize| px[i].to_le_bytes(); // [b,g,r,a] in memory
        assert_eq!(col(0), [2, 2, 255, 255], "block0 top-left -> (255,2,2) red");
        // a texel in block1 (top-right, e.g. row0 col4) -> green
        assert_eq!(col(4), [2, 255, 2, 255], "block1 top-right -> (2,255,2) green");
        // block2 (bottom-left, row4 col0) -> blue
        assert_eq!(col(4 * 8), [255, 2, 2, 255], "block2 bottom-left -> (2,2,255) blue");
        // block3 (bottom-right, row4 col4) -> white
        assert_eq!(col(4 * 8 + 4), [255, 255, 255, 255], "block3 -> white");

        // The SAME blocks labeled GL_COMPRESSED_RGB8_ETC2 (0x9274) must decode identically
        // via decode_etc2_rgb (ETC2 RGB modes 1/2 are bit-identical to ETC1
        // individual/differential). Proves the real Android-Roblox ETC2 path.
        let px2 = decompress(GL_COMPRESSED_RGB8_ETC2, 8, 8, &etc_data).expect("8x8 ETC2 decodes");
        let col2 = |i: usize| px2[i].to_le_bytes();
        assert_eq!(col2(0), [2, 2, 255, 255], "ETC2 block0 -> (255,2,2) red");
        assert_eq!(col2(4), [2, 255, 2, 255], "ETC2 block1 -> (2,255,2) green");
        assert_eq!(col2(4 * 8), [255, 2, 2, 255], "ETC2 block2 -> (2,2,255) blue");
        assert_eq!(col2(4 * 8 + 4), [255, 255, 255, 255], "ETC2 block3 -> white");
        assert_eq!(px2, px, "ETC2 and ETC1 decode the same blocks identically");
    }

    #[test]
    fn non_android_and_unknown_formats_return_none() {
        assert!(decompress(0x83F1, 4, 4, &[0u8; 8]).is_none()); // DXT1 -> Mesa
        assert!(decompress(0xFFFFFFFF, 4, 4, &[0u8; 8]).is_none());
    }

    #[test]
    fn etc2_rgba8_eac_solid_blocks_decode_expected_alpha() {
        // ETC2-RGBA8 (0x9278, the real Android RGBA-EAC format) block = 16 bytes:
        //   [0..8]  EAC alpha sub-block (solid: data[0]=A, data[1]=0 => multiplier==0
        //           path in decode_etc2_a8_block => every texel alpha = data[0]),
        //   [8..16] ETC2-RGB sub-block (bit-identical to the SH29-proven ETC1 blocks).
        // This pins the EAC-alpha half of decode_etc2_rgba8, which the RGB-only
        // 0x9274 path cannot exercise. Same encoding the --renderframe-etc2a harness
        // uploads live (alphas 255/190/128/64 on the red/green/blue/white blocks).
        let enc = |t: i32| -> u8 { let c = ((t - 2).clamp(0, 240) >> 4) as u8; (c << 4) | c };
        let blk = |r: u8, g: u8, b: u8| -> [u8; 8] { [r, g, b, 0, 0, 0, 0, 0] };
        let rgba8 = |a: u8, rgb: [u8; 8]| -> [u8; 16] {
            let mut d = [0u8; 16];
            d[0] = a; // EAC alpha base (solid, multiplier==0)
            d[1] = 0; // multiplier==0 (data[1] & 0xf0 == 0)
            d[8..16].copy_from_slice(&rgb);
            d
        };
        let red = blk(enc(255), enc(2), enc(2));   // (255,2,2)
        let grn = blk(enc(2), enc(255), enc(2));   // (2,255,2)
        let blu = blk(enc(2), enc(2), enc(255));   // (2,2,255)
        let wht = blk(enc(255), enc(255), enc(255)); // white
        let mut data = [0u8; 64]; // 8x8 = 4 blocks, row-major top-first
        data[0..16].copy_from_slice(&rgba8(255, red));
        data[16..32].copy_from_slice(&rgba8(190, grn));
        data[32..48].copy_from_slice(&rgba8(128, blu));
        data[48..64].copy_from_slice(&rgba8(64, wht));

        let px = decompress(GL_COMPRESSED_RGBA8_ETC2_EAC, 8, 8, &data).expect("8x8 ETC2-RGBA8 decodes");
        assert_eq!(px.len(), 64);
        let col = |i: usize| px[i].to_le_bytes(); // memory [b,g,r,a]
        // block0 (top-left, row0 col0) = red with alpha 255
        assert_eq!(col(0), [2, 2, 255, 255]);
        // block1 (top-right, row0 col4) = green, alpha 190
        assert_eq!(col(4), [2, 255, 2, 190]);
        // block2 (bottom-left, row4 col0) = blue, alpha 128
        assert_eq!(col(4 * 8), [255, 2, 2, 128]);
        // block3 (bottom-right, row4 col4) = white, alpha 64
        assert_eq!(col(4 * 8 + 4), [255, 255, 255, 64]);
        // Every texel within a block carries that block's alpha (solid EAC).
        assert_eq!(col(2), [2, 2, 255, 255]);
        assert_eq!(col(4 * 8 + 2), [255, 2, 2, 128]);
    }

    #[test]
    fn astc_ldr_void_extent_blocks_decode_expected_color_and_alpha() {
        // ASTC (0x93B0 = 4x4) LDR void-extent solid-color block (Khronos astc.txt
        // "Void-Extent Blocks"): buf[0]=0xFC, bit8 set (buf[1]&1), bit9 = Dynamic-Range
        // flag = 0 for LDR. Color components are UNORM16 at bytes 8(R)/10(G)/12(B)/14(A);
        // texture2ddecoder's LDR path reads the UNORM16 HIGH bytes [9,11,13,15] = value>>8
        // (exact truncation for a value set as hi<<8), so a solid color+alpha is fully
        // determined by those 4 bytes. Same encoding the --renderframe-astc harness
        // uploads live (gray alphas 255/190/128/64 on the 4 blocks).
        let void_extent = |g: u8| -> [u8; 16] {
            let mut d = [0u8; 16];
            d[0] = 0xFC; // low 8 bits of the 9-bit block-mode "111111100"
            d[1] = 0x01; // bit 8 = 1; bit 9 (Dynamic Range) = 0 => LDR
            // UNORM16 high bytes = 8-bit color (UNORM16 value = g<<8).
            d[9] = g; // R low->high
            d[11] = g; // G
            d[13] = g; // B
            d[15] = g; // A
            d
        };
        let mut data = [0u8; 64]; // 8x8 = 4 blocks (row-major): alphas 255/190/128/64
        data[0..16].copy_from_slice(&void_extent(255));
        data[16..32].copy_from_slice(&void_extent(190));
        data[32..48].copy_from_slice(&void_extent(128));
        data[48..64].copy_from_slice(&void_extent(64));

        let px = decompress(0x93B0, 8, 8, &data).expect("8x8 ASTC 4x4 decodes");
        assert_eq!(px.len(), 64);
        let mem = |i: usize| px[i].to_le_bytes(); // [b,g,r,a]
        assert_eq!(mem(0), [255, 255, 255, 255]); // block0 top-left, alpha 255
        assert_eq!(mem(4), [190, 190, 190, 190]); // block1 top-right, alpha 190
        assert_eq!(mem(4 * 8), [128, 128, 128, 128]); // block2 bottom-left, alpha 128
        assert_eq!(mem(4 * 8 + 4), [64, 64, 64, 64]); // block3 bottom-right, alpha 64
        assert_eq!(mem(2), [255, 255, 255, 255]); // solid within block0
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