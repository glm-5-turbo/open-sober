// SPDX-License-Identifier: MIT
//
// x86-64 instruction decoder — parses legacy prefixes, REX, VEX, ModR/M,
// SIB, and displacement bytes into a structured instruction description.

use core::ptr;

/// A decoded x86-64 instruction (up to 15 bytes).
#[derive(Debug, Clone, Default)]
pub struct DecodedInstruction {
    /// Total instruction length in bytes
    pub len: u8,
    /// Legacy prefix bytes (0x66, 0xF2, 0xF3, seg prefixes)
    pub prefixes: [u8; 4],
    pub prefix_count: u8,
    /// Opcode bytes (1-3)
    pub opcode: [u8; 3],
    pub opcode_len: u8,
    /// ModR/M byte
    pub modrm: u8,
    /// SIB byte
    pub sib: u8,
    /// Displacement value
    pub displacement: i32,
    /// Register index from ModR/M.reg (with REX.R extension)
    pub reg: u8,
    /// R/M field from ModR/M (with REX.B extension)
    pub rm: u8,
    /// Operand size in bits (16, 32, or 64)
    pub operand_size: u8,
    /// Whether ModR/M is present
    pub has_modrm: bool,
    /// Whether SIB is present
    pub has_sib: bool,
    /// Whether displacement is present
    pub has_displacement: bool,
    /// Whether this is a SIMD instruction
    pub is_simd: bool,
    /// Whether VEX prefix was used
    pub is_vex: bool,
    /// 0x66 prefix present
    pub has_66: bool,
    /// 0xF2 prefix present
    pub has_f2: bool,
    /// 0xF3 prefix present
    pub has_f3: bool,
    /// REX byte value
    pub rex: u8,
    /// Whether REX is present
    pub has_rex: bool,
    /// VEX vvvv field (destructive source)
    pub vex_vvvv: u8,
    /// VEX.W bit
    pub vex_w: bool,
}

/// Decode an x86-64 instruction at the given IP.
/// Returns a fully parsed DecodedInstruction.
///
/// # Safety
/// `ip` must point to valid, readable memory (at least 15 bytes recommended).
pub unsafe fn decode_instruction(ip: *const u8) -> DecodedInstruction {
    let mut inst = DecodedInstruction::default();
    let mut pos: usize = 0;

    // --- Parse legacy prefixes ---
    while pos < 15 {
        let b = *ip.add(pos);
        if matches!(b, 0x66 | 0x67 | 0xF2 | 0xF3 | 0x26 | 0x2E | 0x36 | 0x3E | 0x64 | 0x65) {
            if inst.prefix_count < 4 {
                inst.prefixes[inst.prefix_count as usize] = b;
                inst.prefix_count += 1;
            }
            pos += 1;
        } else {
            break;
        }
    }

    // --- Parse REX prefix ---
    if pos < 15 && (*ip.add(pos) & 0xF0) == 0x40 {
        inst.rex = *ip.add(pos);
        inst.has_rex = true;
        pos += 1;
    }

    // --- Categorize prefixes ---
    for i in 0..inst.prefix_count as usize {
        match inst.prefixes[i] {
            0x66 => inst.has_66 = true,
            0xF2 => inst.has_f2 = true,
            0xF3 => inst.has_f3 = true,
            _ => {}
        }
    }

    let first_byte = *ip.add(pos);
    let mut opcode_map: u8 = 0;

    // --- Parse VEX prefix ---
    if first_byte == 0xC4 || first_byte == 0xC5 {
        inst.is_vex = true;
        pos += 1;
        let mut vex_w = 0u8;
        let mut vex_pp: u8 = 0;
        let vex_m: u8;

        if first_byte == 0xC4 {
            // 3-byte VEX
            let b1 = *ip.add(pos); pos += 1;
            let vex_r = (b1 >> 7) & 1;
            let _vex_x = (b1 >> 6) & 1;
            let _vex_b = (b1 >> 5) & 1;
            vex_m = b1 & 0x1F;
            let b2 = *ip.add(pos); pos += 1;
            vex_w = (b2 >> 7) & 1;
            vex_pp = b2 & 0x03;
            // Reconstruct REX: W|~R|~X|~B
            inst.rex = (vex_w << 3) | ((!vex_r & 1) << 2) | ((!((b1 >> 6) & 1) & 1) << 1) | ((!((b1 >> 5) & 1) & 1));
            inst.has_rex = true;
        } else {
            // 2-byte VEX
            let b1 = *ip.add(pos); pos += 1;
            let vex_r = (b1 >> 7) & 1;
            vex_pp = b1 & 0x03;
            vex_m = 1;
            inst.rex = ((!vex_r & 1) << 2);
            inst.has_rex = true;
        }

        if vex_pp == 1 { inst.has_66 = true; }
        if vex_pp == 2 { inst.has_f2 = true; }
        if vex_pp == 3 { inst.has_f3 = true; }

        opcode_map = match vex_m {
            1 => 1,
            2 => 2,
            3 => 3,
            _ => 0,
        };

        inst.vex_w = vex_w != 0;
        first_byte as usize; // suppress unused warning
    }

    // --- Parse opcode bytes ---
    if !inst.is_vex && first_byte == 0x0F {
        // Two-byte opcode (0x0F xx)
        opcode_map = 1;
        inst.opcode[0] = 0x0F;
        inst.opcode[1] = *ip.add(pos + 1);
        inst.opcode_len = 2;
        pos += 2;

        let op2 = inst.opcode[1];
        if op2 == 0x38 || op2 == 0x3A {
            // Three-byte opcode (0x0F 0x38 xx or 0x0F 0x3A xx)
            opcode_map = if op2 == 0x38 { 2 } else { 3 };
            inst.opcode[2] = *ip.add(pos);
            inst.opcode_len = 3;
            pos += 1;
        }
    } else if inst.is_vex {
        match opcode_map {
            1 => {
                inst.opcode[0] = 0x0F;
                inst.opcode[1] = first_byte;
                inst.opcode_len = 2;
            }
            2 => {
                inst.opcode[0] = 0x0F;
                inst.opcode[1] = 0x38;
                inst.opcode[2] = first_byte;
                inst.opcode_len = 3;
            }
            3 => {
                inst.opcode[0] = 0x0F;
                inst.opcode[1] = 0x3A;
                inst.opcode[2] = first_byte;
                inst.opcode_len = 3;
            }
            _ => {
                inst.opcode[0] = first_byte;
                inst.opcode_len = 1;
            }
        }
    } else {
        inst.opcode[0] = first_byte;
        inst.opcode_len = 1;
    }

    // --- Determine if ModR/M is needed ---
    let needs_modrm = needs_modrm_for_opcode(opcode_map, &inst);
    let is_simd = is_simd_for_opcode(opcode_map, &inst);
    inst.is_simd = is_simd;

    // --- Parse ModR/M, SIB, displacement ---
    if needs_modrm && pos < 15 {
        let modrm_byte = *ip.add(pos);
        let mod_field = (modrm_byte >> 6) & 0x03;
        let reg_field = (modrm_byte >> 3) & 0x07;
        let rm_field = modrm_byte & 0x07;

        inst.modrm = modrm_byte;
        inst.reg = reg_field | (((inst.rex >> 2) & 1) << 3);
        inst.rm = rm_field | ((inst.rex & 1) << 3);
        inst.has_modrm = true;
        pos += 1;

        if mod_field != 3 {
            // Memory operand
            if rm_field == 4 {
                // SIB byte present
                let sib_byte = *ip.add(pos);
                let base_field = sib_byte & 0x07;
                inst.sib = sib_byte;
                inst.has_sib = true;
                pos += 1;

                let disp_sz = displacement_size_with_sib(mod_field, base_field);
                if disp_sz > 0 && pos + disp_sz as usize <= 15 {
                    inst.has_displacement = true;
                    inst.displacement = if disp_sz == 4 {
                        i32::from_le_bytes(ptr::read(ip.add(pos) as *const [u8; 4]))
                    } else {
                        *ip.add(pos) as i8 as i32
                    };
                    pos += disp_sz as usize;
                }
            } else {
                let disp_sz = displacement_size(mod_field, rm_field, false);
                if disp_sz > 0 && pos + disp_sz as usize <= 15 {
                    inst.has_displacement = true;
                    inst.displacement = if disp_sz == 4 {
                        i32::from_le_bytes(ptr::read(ip.add(pos) as *const [u8; 4]))
                    } else {
                        *ip.add(pos) as i8 as i32
                    };
                    pos += disp_sz as usize;
                }
            }
        }
    }

    // --- Determine operand size ---
    inst.operand_size = if inst.has_66 { 16 } else { 32 };
    if (inst.rex & 0x08) != 0 && !inst.has_66 {
        inst.operand_size = 64;
    }

    inst.len = pos as u8;
    inst
}

fn needs_modrm_for_opcode(map: u8, inst: &DecodedInstruction) -> bool {
    match map {
        1 => {
            let op2 = inst.opcode[1];
            op2 < 0x06
                || (0x10..=0x73).contains(&op2)
                || (0x7C..=0x7F).contains(&op2)
                || (0x90..=0x9F).contains(&op2)
                || (0xA3..=0xAF).contains(&op2)
                || (0xB0..=0xBF).contains(&op2)
                || (0xC0..=0xCF).contains(&op2)
                || (0xD0..=0xDF).contains(&op2)
                || (0xE0..=0xEF).contains(&op2)
                || op2 >= 0xF0
        }
        2 | 3 => true,
        _ => {
            // One-byte opcode ModR/M table (SIMD variants only)
            matches!(
                inst.opcode[0],
                0x00..=0x03
                    | 0x08..=0x0B
                    | 0x10..=0x13
                    | 0x18..=0x1B
                    | 0x20..=0x23
                    | 0x28..=0x2B
                    | 0x30..=0x33
                    | 0x38..=0x3B
                    | 0x60..=0x63
                    | 0x68..=0x6B
                    | 0x80..=0x83
                    | 0x88..=0x8B
                    | 0xC0..=0xC1
                    | 0xC4..=0xC6
                    | 0xD0..=0xD3
                    | 0xF6..=0xF7
                    | 0xFE..=0xFF
            )
        }
    }
}

fn is_simd_for_opcode(map: u8, inst: &DecodedInstruction) -> bool {
    match map {
        1 => {
            let op2 = inst.opcode[1];
            (0x10..=0x7F).contains(&op2)
                || ((0x90..=0x9F).contains(&op2)
                    && op2 != 0xA0
                    && op2 != 0xA1
                    && op2 != 0xA2
                    && op2 != 0xA8
                    && op2 != 0xA9
                    && op2 != 0xB0
                    && op2 != 0xB1)
                || (0xC4..=0xCF).contains(&op2)
                || (0xD0..=0xDF).contains(&op2)
                || (0xE0..=0xEF).contains(&op2)
        }
        2 | 3 => true,
        _ => false,
    }
}

fn displacement_size(mod_field: u8, rm_field: u8, has_sib: bool) -> u8 {
    match mod_field {
        0 => {
            if rm_field == 5 && !has_sib { 4 }
            else { 0 }
        }
        1 => 1,
        2 => 4,
        _ => 0,
    }
}

fn displacement_size_with_sib(mod_field: u8, base_field: u8) -> u8 {
    match mod_field {
        0 if base_field == 5 => 4,
        1 => 1,
        2 => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_popcnt() {
        // POPCNT r32, r32: F3 0F B8 C0 (popcnt eax, eax)
        let code = [0xF3u8, 0x0F, 0xB8, 0xC0];
        let inst = unsafe { decode_instruction(code.as_ptr()) };
        assert!(inst.has_f3, "Expected F3 prefix");
        assert_eq!(inst.opcode[0], 0x0F);
        assert_eq!(inst.opcode[1], 0xB8);
        assert_eq!(inst.len, 4);
        inst_test_modrm(inst);
    }

    #[test]
    fn test_decode_movbe() {
        // MOVBE r32, m32: 0F 38 F0 10 (movbe edx, [rax])
        let code = [0x0Fu8, 0x38, 0xF0, 0x10];
        let inst = unsafe { decode_instruction(code.as_ptr()) };
        assert_eq!(inst.opcode_len, 3);
        assert_eq!(inst.opcode[0], 0x0F);
        assert_eq!(inst.opcode[1], 0x38);
        assert_eq!(inst.opcode[2], 0xF0);
    }

    #[test]
    fn test_decode_lzcnt() {
        // LZCNT r32, r32: F3 0F BD C0
        let code = [0xF3u8, 0x0F, 0xBD, 0xC0];
        let inst = unsafe { decode_instruction(code.as_ptr()) };
        assert!(inst.has_f3);
        assert_eq!(inst.opcode[0], 0x0F);
        assert_eq!(inst.opcode[1], 0xBD);
    }

    fn inst_test_modrm(_inst: DecodedInstruction) {
        // stub for consistency
    }
}