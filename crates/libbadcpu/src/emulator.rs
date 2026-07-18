// SPDX-License-Identifier: MIT
//
// Instruction emulation engine. Handles dispatch of decoded x86-64 instructions
// that the host CPU doesn't natively support, emulating them in software.

use crate::decoder::*;
use crate::cpuid::CpuFeatures;

/// Result of an emulation attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmulationResult {
    Success,
    UnrecognizedInstruction,
    #[allow(unused)]
    UnsupportedCpu,
}

/// Emulate a decoded instruction. Modifies the ucontext in-place.
///
/// # Safety
/// `ctx` must be a valid pointer to a ucontext_t struct from a signal handler.
pub unsafe fn emulate(
    inst: &DecodedInstruction,
    _features: &CpuFeatures,
    ctx: *mut libc::ucontext_t,
) -> EmulationResult {
    if inst.is_vex {
        return emulate_vex(inst, ctx);
    }

    if inst.opcode[0] == 0x0F && inst.opcode_len >= 2 {
        let op2 = inst.opcode[1];
        let op3 = if inst.opcode_len >= 3 { inst.opcode[2] } else { 0 };

        // POPCNT: F3 0F B8 /r
        if op2 == 0xB8 && inst.has_f3 && inst.has_modrm {
            return emulate_popcnt(inst, ctx);
        }

        // MOVBE: 0F 38 F0-F1 /r
        if op2 == 0x38 && (op3 == 0xF0 || op3 == 0xF1) {
            return emulate_movbe(inst, ctx);
        }

        // LZCNT: F3 0F BD /r  or   F3 0F 38 F5 /r
        if (op2 == 0xBD || (op2 == 0x38 && op3 == 0xF5)) && inst.has_f3 {
            return emulate_count_leading_zeros(inst, ctx);
        }

        // TZCNT: F3 0F BC /r
        if op2 == 0xBC && inst.has_f3 {
            return emulate_count_trailing_zeros(inst, ctx);
        }
    }

    EmulationResult::UnrecognizedInstruction
}

// --- BMI1 instructions (VEX.0F.38) ---
unsafe fn emulate_vex(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> EmulationResult {
    if inst.opcode_len < 3 {
        return EmulationResult::UnrecognizedInstruction;
    }

    let op3 = inst.opcode[2];
    let is_64bit = (inst.rex & 0x08) != 0 && !inst.has_66;

    if inst.opcode[0] == 0x0F && inst.opcode[1] == 0x38 {
        let src1 = get_reg(ctx, inst.reg);
        let src2 = get_rm_value(inst, ctx);

        let result: u64 = match op3 {
            0xF2 => src1 & !src2,
            0xF3 => src2.wrapping_neg() & src2,
            0xF1 => src2 ^ (src2.wrapping_sub(1)),
            0xF4 => src2 & (src2.wrapping_sub(1)),
            _ => return EmulationResult::UnrecognizedInstruction,
        };

        let result = if is_64bit { result } else { result & 0xFFFF_FFFF };

        *get_reg_ptr(ctx, inst.reg) = result as i64;
        update_flags_common(ctx, result, is_64bit);
        advance_rip(ctx, inst.len);
        return EmulationResult::Success;
    }

    EmulationResult::UnrecognizedInstruction
}

// --- POPCNT: F3 0F B8 /r ---
unsafe fn emulate_popcnt(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> EmulationResult {
    let is_64bit = (inst.rex & 0x08) != 0 && !inst.has_66;
    let src = get_rm_value(inst, ctx);

    let count = if inst.has_66 {
        (src as u16).count_ones() as u64
    } else if is_64bit {
        src.count_ones() as u64
    } else {
        (src as u32).count_ones() as u64
    };

    *get_reg_ptr(ctx, inst.reg) = count as i64;
    set_flags_clear_all(ctx);
    if count == 0 {
        set_zf(ctx);
    }

    advance_rip(ctx, inst.len);
    EmulationResult::Success
}

// --- MOVBE: 0F 38 F0-F1 /r ---
unsafe fn emulate_movbe(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> EmulationResult {
    let is_64bit = (inst.rex & 0x08) != 0 && !inst.has_66;
    let addr_or_val = get_rm_value(inst, ctx);

    if (inst.modrm >> 6) == 3 {
        let swapped = if is_64bit {
            addr_or_val.swap_bytes()
        } else if inst.has_66 {
            (addr_or_val as u16).swap_bytes() as u64
        } else {
            (addr_or_val as u32).swap_bytes() as u64
        };
        *get_reg_ptr(ctx, inst.reg) = swapped as i64;
    } else {
        if is_64bit {
            let val = *(addr_or_val as *const u64);
            *get_reg_ptr(ctx, inst.reg) = val.swap_bytes() as i64;
        } else if inst.has_66 {
            let val = *(addr_or_val as *const u16);
            *get_reg_ptr(ctx, inst.reg) = val.swap_bytes() as i64;
        } else {
            let val = *(addr_or_val as *const u32);
            *get_reg_ptr(ctx, inst.reg) = val.swap_bytes() as i64;
        }
    }

    advance_rip(ctx, inst.len);
    EmulationResult::Success
}

// --- LZCNT: F3 0F BD /r ---
unsafe fn emulate_count_leading_zeros(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> EmulationResult {
    let is_64bit = (inst.rex & 0x08) != 0 && !inst.has_66;
    let src = get_rm_value(inst, ctx);

    let result = if src == 0 {
        if is_64bit { 64 } else if inst.has_66 { 16 } else { 32 }
    } else if is_64bit {
        src.leading_zeros() as u64
    } else if inst.has_66 {
        (src as u16).leading_zeros() as u64 - 16
    } else {
        (src as u32).leading_zeros() as u64
    };

    *get_reg_ptr(ctx, inst.reg) = result as i64;
    set_flags_clear_all(ctx);
    if src == 0 { set_zf(ctx); }
    if result == 0 { set_cf(ctx); }

    advance_rip(ctx, inst.len);
    EmulationResult::Success
}

// --- TZCNT: F3 0F BC /r ---
unsafe fn emulate_count_trailing_zeros(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> EmulationResult {
    let is_64bit = (inst.rex & 0x08) != 0 && !inst.has_66;
    let src = get_rm_value(inst, ctx);

    let result = if src == 0 {
        if is_64bit { 64 } else if inst.has_66 { 16 } else { 32 }
    } else if is_64bit {
        src.trailing_zeros() as u64
    } else if inst.has_66 {
        (src as u16).trailing_zeros() as u64
    } else {
        (src as u32).trailing_zeros() as u64
    };

    *get_reg_ptr(ctx, inst.reg) = result as i64;
    set_flags_clear_all(ctx);
    if src == 0 { set_zf(ctx); }

    advance_rip(ctx, inst.len);
    EmulationResult::Success
}

// === Register access helpers ===
// On x86-64 Linux, gregs is [i64; 23]. We work with u64 values and cast.

// Register index mapping matching Linux kernel's ucontext layout
const REG_RAX: usize = 0;
const REG_RBX: usize = 3;
const REG_RCX: usize = 1;
const REG_RDX: usize = 2;
const REG_RSI: usize = 6;
const REG_RDI: usize = 7;
const REG_RBP: usize = 5;
const REG_RSP: usize = 4;
const REG_R8: usize = 8;
const REG_R9: usize = 9;
const REG_R10: usize = 10;
const REG_R11: usize = 11;
const REG_R12: usize = 12;
const REG_R13: usize = 13;
const REG_R14: usize = 14;
const REG_R15: usize = 15;
const REG_RIP: usize = 16;
const REG_EFL: usize = 17;

fn reg_index(reg: u8) -> usize {
    match reg {
        0 => REG_RAX, 1 => REG_RCX, 2 => REG_RDX, 3 => REG_RBX,
        4 => REG_RSP, 5 => REG_RBP, 6 => REG_RSI, 7 => REG_RDI,
        8 => REG_R8, 9 => REG_R9, 10 => REG_R10, 11 => REG_R11,
        12 => REG_R12, 13 => REG_R13, 14 => REG_R14, 15 => REG_R15,
        _ => 0,
    }
}

/// Get a mutable pointer to a register value (as i64).
unsafe fn get_reg_ptr(ctx: *mut libc::ucontext_t, reg: u8) -> *mut i64 {
    let gregs = &mut (*ctx).uc_mcontext.gregs;
    let idx = reg_index(reg);
    &mut gregs[idx]
}

/// Get register value as u64.
unsafe fn get_reg(ctx: *mut libc::ucontext_t, reg: u8) -> u64 {
    let gregs = &(*ctx).uc_mcontext.gregs;
    let idx = reg_index(reg);
    gregs[idx] as u64
}

/// Get the value of the ModR/M.rm operand (register or memory reference).
unsafe fn get_rm_value(inst: &DecodedInstruction, ctx: *mut libc::ucontext_t) -> u64 {
    if (inst.modrm >> 6) == 3 {
        return get_reg(ctx, inst.rm);
    }

    let mod_field = (inst.modrm >> 6) & 0x03;
    let addr = if inst.rm == 4 && inst.has_sib {
        let scale = (inst.sib >> 6) & 0x03;
        let index = ((inst.sib >> 3) & 0x07) | (((inst.rex >> 1) & 1) << 3);
        let base = (inst.sib & 0x07) | ((inst.rex & 1) << 3);

        let base_val = if base == 5 && mod_field == 0 {
            inst.displacement as i64 as u64
        } else {
            get_reg(ctx, base)
        };
        let index_val = if index == 4 { 0 } else { get_reg(ctx, index) };
        base_val.wrapping_add(index_val << scale)
    } else if inst.rm == 5 && mod_field == 0 {
        inst.displacement as i64 as u64
    } else {
        get_reg(ctx, inst.rm)
    };

    let disp_off = match mod_field {
        1 => inst.displacement as i8 as i64 as u64,
        2 => inst.displacement as i64 as u64,
        _ => 0,
    };

    addr.wrapping_add(disp_off)
}

// === Flag manipulation ===
// EFLAGS bits: CF=0, PF=2, AF=4, ZF=6, SF=7, OF=11

/// Clear arithmetic flags.
unsafe fn set_flags_clear_all(ctx: *mut libc::ucontext_t) {
    let idx = REG_EFL;
    let gregs = &mut (*ctx).uc_mcontext.gregs;
    gregs[idx] &= !0x8D5i64;
}

/// Set Zero Flag (bit 6).
unsafe fn set_zf(ctx: *mut libc::ucontext_t) {
    let idx = REG_EFL;
    let gregs = &mut (*ctx).uc_mcontext.gregs;
    gregs[idx] |= 0x40;
}

/// Set Carry Flag (bit 0).
unsafe fn set_cf(ctx: *mut libc::ucontext_t) {
    let idx = REG_EFL;
    let gregs = &mut (*ctx).uc_mcontext.gregs;
    gregs[idx] |= 0x1;
}

/// Advance RIP by instruction length.
unsafe fn advance_rip(ctx: *mut libc::ucontext_t, len: u8) {
    let idx = REG_RIP;
    let gregs = &mut (*ctx).uc_mcontext.gregs;
    gregs[idx] = gregs[idx].wrapping_add(len as i64);
}

/// Update ZF and SF after an operation.
unsafe fn update_flags_common(ctx: *mut libc::ucontext_t, result: u64, is_64bit: bool) {
    set_flags_clear_all(ctx);
    if result == 0 {
        set_zf(ctx);
    }
    if is_64bit && result >> 63 != 0 {
        let idx = REG_EFL;
        let gregs = &mut (*ctx).uc_mcontext.gregs;
        gregs[idx] |= 0x80;
    } else if !is_64bit && result >> 31 != 0 {
        let idx = REG_EFL;
        let gregs = &mut (*ctx).uc_mcontext.gregs;
        gregs[idx] |= 0x80;
    }
}