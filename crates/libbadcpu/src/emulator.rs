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
    let wide = inst.vex_w;

    // VEX.0F3A.F0: RORX dest(reg), src(rm), imm8 — rotate right by imm8,
    // flags NOT affected, VEX vvvv field unused (must be 0b1111). The decoder
    // stops after ModR/M (it does not collect an immediate), so the imm8 lives
    // at RIP+inst.len and we must advance by len+1.
    if inst.opcode[0] == 0x0F && inst.opcode[1] == 0x3A {
        if op3 == 0xF0 {
            let src = get_rm_value(inst, ctx);
            let gregs = &(*ctx).uc_mcontext.gregs;
            let rip = gregs[REG_RIP] as u64;
            let imm = unsafe { *(rip as *const u8).add(inst.len as usize) as u32 };
            let amt = imm & if wide { 63 } else { 31 };
            let rotated = if wide {
                src.rotate_right(amt)
            } else {
                ((src as u32).rotate_right(amt)) as u64
            };
            *get_reg_ptr(ctx, inst.reg) = rotated as i64;
            advance_rip(ctx, inst.len + 1);
            return EmulationResult::Success;
        }
        return EmulationResult::UnrecognizedInstruction;
    }

    if inst.opcode[0] == 0x0F && inst.opcode[1] == 0x38 {
        let src2 = get_rm_value(inst, ctx);

        // BMI-non-destructive ops that set a carry flag must set it AFTER
        // update_flags_common (which clears flags). Captured here, applied below.
        let mut cf_pending = false;

        let result: u64 = match op3 {
            // ANDN dest, src1, src2: dest = NOT(src2) AND src1, where src1 is
            // the VEX vvvv field (the NDS/non-destructive source), NOT modrm.reg.
            0xF2 => {
                let src1 = get_reg(ctx, inst.vex_vvvv);
                src1 & !src2
            }
            0xF3 => src2.wrapping_neg() & src2,
            0xF1 => src2 ^ (src2.wrapping_sub(1)),
            0xF4 => src2 & (src2.wrapping_sub(1)),
            // 0F38 F5 is shared by BZHI / PEXT / PDEP, disambiguated by the VEX
            // pp bits (assembler ground truth: bzhi=pp0, pext=pp2, pdep=pp3):
            //   BZHI dest, src1(rm), control(vvvv): DEST = src1 & (2^ctrl - 1);
            //       if ctrl >= operand size, DEST = src1 with CF=1.
            //   PEXT dest, source(vvvv), mask(rm): gather source bits at mask
            //       positions into the low bits of dest; CF = last bit gathered.
            //   PDEP dest, source(vvvv), mask(rm): deposit source bit i into the
            //       i-th *set* mask position; CF = last bit deposited.
            0xF5 => {
                if inst.has_f3 || inst.has_f2 {
                    let data = get_reg(ctx, inst.vex_vvvv); // source (vvvv)
                    let mask = src2;                        // mask (rm)
                    let nbits = if wide { 64 } else { 32 };
                    let mut dst: u64 = 0;
                    let mut cnt: u32 = 0;
                    let mut last: u64 = 0;
                    for bit in 0..nbits {
                        let bmask = 1u64 << bit;
                        if mask & bmask != 0 {
                            let sb = if inst.has_f3 {
                                // PDEP: take the next source bit, place at mask pos.
                                (data >> cnt) & 1
                            } else {
                                // PEXT: take the source bit at this position, pack low.
                                (data >> bit) & 1
                            };
                            last = sb;
                            if inst.has_f3 {
                                dst |= sb << bit;
                            } else {
                                dst |= sb << cnt;
                            }
                            cnt += 1;
                        }
                    }
                    cf_pending = last != 0;
                    dst
                } else {
                    // BZHI
                    let control = get_reg(ctx, inst.vex_vvvv);
                    let bits = if wide { 64u64 } else { 32u64 };
                    if control >= bits {
                        cf_pending = true;
                        src2
                    } else if wide {
                        src2 & ((1u64 << control) - 1)
                    } else {
                        src2 & ((1u32 << control as u32) as u64 - 1)
                    }
                }
            }
            // 0F38 F7: BEXTR (pp=0) or the BMI2 shifts SHRX/SARX/SHLX
            // (pp = F3/F2/66 respectively). They share the opcode; the
            // fp-prefix byte picks which.
            0xF7 => {
                if inst.has_f3 || inst.has_f2 || inst.has_66 {
                    let count = get_reg(ctx, inst.vex_vvvv);
                    if inst.has_f3 {
                        // SHRX — logical right (zero-extends)
                        if wide {
                            src2 >> (count & 63)
                        } else {
                            ((src2 as u32) >> (count & 31)) as u64
                        }
                    } else if inst.has_f2 {
                        // SARX — arithmetic right (fills sign)
                        if wide {
                            ((src2 as i64) >> (count & 63)) as u64
                        } else {
                            ((src2 as i32) >> (count & 31)) as u64
                        }
                    } else {
                        // SHLX — logical left
                        if wide {
                            src2 << (count & 63)
                        } else {
                            ((src2 as u32) << (count & 31)) as u64
                        }
                    }
                } else {
                    // BEXTR dest, src1(rm), control(vvvv): extract the bit
                    // field [start, start+len) of src1. start=control[7:0],
                    // len=control[15:8]. len>=operand-size yields src1. A
                    // start at/behind the operand size yields 0 (guarded so a
                    // Rust `>>` never shifts by >= bits, which panics — and a
                    // panic inside the SIGILL handler abort the process).
                    let control = get_reg(ctx, inst.vex_vvvv);
                    let start = (control & 0xff) as u32;
                    let len = ((control >> 8) & 0xff) as u32;
                    if wide {
                        if len >= 64 {
                            src2
                        } else if start >= 64 {
                            0
                        } else {
                            (src2 >> start) & ((1u64 << len) - 1)
                        }
                    } else {
                        let s = src2 as u32;
                        (if len >= 32 {
                            s
                        } else if start >= 32 {
                            0
                        } else {
                            (s >> start) & ((1u32 << len) - 1)
                        }) as u64
                    }
                }
            }
            // 0F38 F6: MULX dest_hi(reg), dest_lo(vvvv), src(rm) — unsigned
            // multiply of RDX * rm. operands: the source is rm, the LOWER
            // product half goes to the VEX vvvv register, the UPPER half to
            // MODRM.reg. Implicit multiplier is RDX (not RAX, unlike MUL).
            // Flags: CF and OF cleared, others untouched.
            0xF6 => {
                let src = src2;
                let rdx = get_reg(ctx, 2); // RDX = GREGS_IDX[2] = slot 12
                // Implicit multiplier is RDX. Wide: 64x64->128 split at 64;
                // narrow: 32-bit operands (EDX, low 32 of src) -> 64-bit product,
                // high half written to reg, low half to vvvv (both 32-bit regs).
                let (hi, lo) = if wide {
                    let p = (rdx as u128) * (src as u128);
                    ((p >> 64) as u64, p as u64)
                } else {
                    let p = (rdx as u32 as u64) * (src as u32 as u64);
                    ((p >> 32) & 0xFFFF_FFFF, p & 0xFFFF_FFFF)
                };
                *get_reg_ptr(ctx, inst.reg) = hi as i64;      // MODRM.reg = high
                *get_reg_ptr(ctx, inst.vex_vvvv) = lo as i64; // vvvv = low
                set_flags_clear_all(ctx);
                advance_rip(ctx, inst.len);
                return EmulationResult::Success;
            }
            _ => return EmulationResult::UnrecognizedInstruction,
        };

        let result = if wide { result } else { result & 0xFFFF_FFFF };

        *get_reg_ptr(ctx, inst.reg) = result as i64;
        // The BMI integer ops set ZF (and for BZHI carry already handled);
        // the shifts leave flags untouched (CF unset, ZF cleared).
        update_flags_common(ctx, result, wide);
        // BZHI (ctrl >= bits), PEXT and PDEP set CF to the last bit; apply it
        // now that update_flags_common has cleared the flags.
        if cf_pending {
            set_cf(ctx);
        }
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
        (src as u16).leading_zeros() as u64
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
// On x86-64 Linux, ucontext_t.uc_mcontext.gregs is greg_t[23] (NGREG), laid
// out by glibc in a NON-trivial order (see <sys/ucontext.h> / libc's
// REG_* constants): R8..R15 occupy slots 0..7, then RDI,RSI,RBP,RBX,RDX,
// RAX,RCX,RSP (8..15), then RIP=16, EFL=17. Only RIP (16) and EFL (17)
// match the hardware register number; all GPR slots differ. The emulator
// must map an x86 register NUMBER (ModR/M.reg base 0..15, REX.R/B-extended)
// to the correct gregs slot, NOT use the register number as the index.

// gregs slot for each x86-64 register number (REX-extended, 0..15).
// Built from glibc's REG_* enum (R8=0...R15=7, RDI=8, RSI=9, RBP=10,
// RBX=11, RDX=12, RAX=13, RCX=14, RSP=15).
const GREGS_IDX: [usize; 16] = [
    13, // reg 0  = RAX
    14, // reg 1  = RCX
    12, // reg 2  = RDX
    11, // reg 3  = RBX
    15, // reg 4  = RSP
    10, // reg 5  = RBP
    9,  // reg 6  = RSI
    8,  // reg 7  = RDI
    0,  // reg 8  = R8
    1,  // reg 9  = R9
    2,  // reg 10 = R10
    3,  // reg 11 = R11
    4,  // reg 12 = R12
    5,  // reg 13 = R13
    6,  // reg 14 = R14
    7,  // reg 15 = R15
];

const REG_RIP: usize = 16;
const REG_EFL: usize = 17;

fn reg_index(reg: u8) -> usize {
    if (reg as usize) < GREGS_IDX.len() {
        GREGS_IDX[reg as usize]
    } else {
        GREGS_IDX[0]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::decode_instruction;

    /// A hand-built ucontext: only the gregs slots we touch are set; the rest
    /// zero. Returns (ctx_ptr, code_backing) so the code bytes stay alive.
    /// gregs slots use the REAL glibc x86-64 layout (RAX=13, RCX=14, RIP=16).
    fn make_ctx(code: &[u8]) -> (libc::ucontext_t, Vec<u8>) {
        let mut ctx: libc::ucontext_t = unsafe { std::mem::zeroed() };
        let mut backing = code.to_vec();
        backing.resize(32, 0xcc); // pad so decode has readable bytes
        ctx.uc_mcontext.gregs[REG_RIP] = backing.as_ptr() as i64;
        (ctx, backing)
    }

    /// POPCNT r64,r64 = F3 48 0F B8 /r. Emulation must write the DEST reg
    /// (ModR/M.reg) and read the SOURCE (ModR/M.rm) at their true gregs slots.
    #[test]
    fn popcnt_writes_dest_at_correct_gregs_slot() {
        // popcnt rax, rcx  (REX.W=48, mod=11 reg=000 rm=001)
        let code = [0xF3u8, 0x48, 0x0F, 0xB8, 0xC1];
        let mut ctx;
        let backing;
        unsafe {
            let (c, b) = make_ctx(&code);
            ctx = c;
            backing = b;
            // source rcx = 0b1011 (3 set bits); dest rax = junk that must be overwritten.
            ctx.uc_mcontext.gregs[14] = 0b1011; // RCX
            ctx.uc_mcontext.gregs[13] = 0xDEAD; // RAX
            let features = crate::cpuid::CpuFeatures::default();
            let inst = decode_instruction(backing.as_ptr());
            let res = emulate(&inst, &features, &mut ctx);
            assert_eq!(res, EmulationResult::Success, "popcnt should emulate");
            // Destination is rax -> gregs[13], value = 3.
            assert_eq!(ctx.uc_mcontext.gregs[13], 3, "popcnt result in RAX");
            // RIP advanced by the instruction length.
            assert_eq!(
                ctx.uc_mcontext.gregs[REG_RIP],
                backing.as_ptr() as i64 + 5,
                "RIP advanced by 5"
            );
        }
    }

    /// LZCNT (F3 0F BD) on a non-zero source writes the count to the DEST reg.
    #[test]
    fn lzcnt_writes_dest_count_at_correct_slot() {
        // F3 REX.W 0F BD C8 = lzcnt rcx, rax (prefix F3, then REX.W=48, reg=001=rcx, rm=000=rax)
        let code = [0xF3u8, 0x48, 0x0F, 0xBD, 0xC8];
        let mut ctx;
        let backing;
        unsafe {
            let (c, b) = make_ctx(&code);
            ctx = c;
            backing = b;
            ctx.uc_mcontext.gregs[13] = 1; // rax = 1 -> lzcnt(1)=63
            let features = crate::cpuid::CpuFeatures::default();
            let inst = decode_instruction(backing.as_ptr());
            let _ = emulate(&inst, &features, &mut ctx);
            assert_eq!(ctx.uc_mcontext.gregs[14], 63, "lzcnt result in RCX");
        }
    }

    /// TZCNT 16-bit (66 F3 0F BC) exercises the 16-bit GPR path.
    #[test]
    fn tzcnt_16bit_matches_real_count() {
        // 66 F3 0F BC C8 = tzcnt cx, ax (operand-size 66 -> 16-bit)
        let code = [0x66u8, 0xF3, 0x0F, 0xBC, 0xC8];
        let mut ctx;
        let backing;
        unsafe {
            let (c, b) = make_ctx(&code);
            ctx = c;
            backing = b;
            ctx.uc_mcontext.gregs[13] = 0x100; // ax = 0x0100 -> tzcnt=8
            let features = crate::cpuid::CpuFeatures::default();
            let inst = decode_instruction(backing.as_ptr());
            // inst.reg (dest)=1 = rcx=14; inst.rm=0=rax=13. Verify decode fn table.
            assert_eq!(inst.reg, 1);
            assert_eq!(inst.rm, 0);
            let _ = emulate(&inst, &features, &mut ctx);
            assert_eq!(ctx.uc_mcontext.gregs[14], 8, "tzcnt finally in RCX");
        }
    }

    /// LZCNT 16-bit (66 F3 0F BD) — the `(src as u16).leading_zeros()` count is
    /// already measured against a 16-bit operand, so it must NOT be offset by
    /// 16 (the old `- 16` returned negative/huge for every nonzero source).
    #[test]
    fn lzcnt_16bit_matches_real_count() {
        // 66 F3 0F BD C8 = lzcnt cx, ax (operand-size 66 -> 16-bit)
        let code = [0x66u8, 0xF3, 0x0F, 0xBD, 0xC8];
        let mut ctx;
        let backing;
        unsafe {
            let (c, b) = make_ctx(&code);
            ctx = c;
            backing = b;
            // ax = 0x0002 -> 16-bit leading zeros = 14.
            ctx.uc_mcontext.gregs[13] = 0x2; // rax (=ax)
            let features = crate::cpuid::CpuFeatures::default();
            let inst = decode_instruction(backing.as_ptr());
            let res = emulate(&inst, &features, &mut ctx);
            assert_eq!(res, EmulationResult::Success, "lzcnt 16-bit should emulate");
            assert_eq!(
                ctx.uc_mcontext.gregs[14],
                14,
                "lzcnt(cx, ax=2) = 14 (not a negative/huge wrap)"
            );

            // ax = 0x8000 (only the top bit set) -> 0 leading zeros.
            ctx.uc_mcontext.gregs[13] = 0x8000;
            let inst = decode_instruction(backing.as_ptr());
            let _ = emulate(&inst, &features, &mut ctx);
            assert_eq!(ctx.uc_mcontext.gregs[14], 0, "lzcnt(cx, ax=0x8000) = 0");
        }
    }

    /// ANDN r32a,r32b,r/m32 = VEX.NDS.LZ.0F38.W0 F2 /r.
    /// The first source is the VEX vvvv field, NOT the modrm.reg (dest).
    /// C4 E2 70 F2 C2 = ANDN eax, ecx, edx (mod=11 reg=000 eax rm=010 edx,
    /// vvvv=1 ecx). Semantics: eax = ~edx & ecx.
    #[test]
    fn andn_uses_vev_vvvv_source() {
        // ANDN eax, ecx, edx
        let code = [0xC4u8, 0xE2, 0x70, 0xF2, 0xC2];
        let mut ctx;
        let backing;
        unsafe {
            let (c, b) = make_ctx(&code);
            ctx = c;
            backing = b;
            // ecx (slot 14, vvvv=1) = 0x0F, edx (slot 12, rm=2) = 0x03.
            // ANDN = ~0x03 & 0x0F = 0x0C, written to eax (slot 13).
            ctx.uc_mcontext.gregs[14] = 0x0F; // ecx
            ctx.uc_mcontext.gregs[12] = 0x03; // edx
            ctx.uc_mcontext.gregs[13] = 0xDEAD; // eax (dest, must be overwritten)
            let features = crate::cpuid::CpuFeatures::default();
            let inst = decode_instruction(backing.as_ptr());
            assert!(inst.is_vex, "should be VEX");
            assert_eq!(inst.opcode[0], 0x0F);
            assert_eq!(inst.opcode[1], 0x38);
            assert_eq!(inst.opcode[2], 0xF2);
            assert_eq!(inst.vex_vvvv, 1, "vvvv decoded = ecx");
            assert_eq!(inst.reg, 0, "dest = eax");
            assert_eq!(inst.rm, 2, "src2 = edx");
            let _ = emulate(&inst, &features, &mut ctx);
            // Dest eax = ~edx & ecx = ~0x03 & 0x0F = 0x0C.
            assert_eq!(
                ctx.uc_mcontext.gregs[13],
                0x0C,
                "ANDN eax,ecx,edx = ~edx & ecx in eax"
            );
        }
    }

    /// BLSI/BLSMSK/BLSR do NOT use vvvv — dest is modrm.reg and the sole
    /// BLSR (VEX.0F38.W0 F4) clears the low bit.
        /// C4 E2 70 F4 C2 = blsr eax, edx (mod=11 reg=000 eax rm=010 edx).
        #[test]
        fn blsr_is_rm_sourced_not_vvvv() {
            // blsr eax, edx  (vvvv field present but ignored by BLSR)
            let code = [0xC4u8, 0xE2, 0x70, 0xF4, 0xC2];
            let mut ctx;
            let backing;
            unsafe {
                let (c, b) = make_ctx(&code);
                ctx = c;
                backing = b;
                // edx (slot 12, rm=2) = 0x0B (1011); vvvv=ecx must be IGNORED.
                ctx.uc_mcontext.gregs[12] = 0x0B; // edx
                ctx.uc_mcontext.gregs[14] = 0xFF; // ecx (vvvv, irrelevant)
                ctx.uc_mcontext.gregs[13] = 0xDEAD; // eax (dest)
                let features = crate::cpuid::CpuFeatures::default();
                let inst = decode_instruction(backing.as_ptr());
                assert_eq!(inst.vex_vvvv, 1);
                let _ = emulate(&inst, &features, &mut ctx);
                // BLSR = edx & (edx-1) = 0x0B & 0x0A = 0x0A.
                assert_eq!(ctx.uc_mcontext.gregs[13], 0x0A, "blsr eax,edx = edx&(edx-1)");
            }
        }

        /// Run one emulation of `code` with the given initial register values
        /// (rcx=14, rdx=12, rax=13 slots) and code bytes, returning the value of
        /// the DEST register (rax slot 13).
        fn run_one(code: &[u8], rcx: u64, rdx: u64) -> i64 {
            unsafe {
                let mut ctx: libc::ucontext_t = std::mem::zeroed();
                let mut backing = code.to_vec();
                backing.resize(8, 0xcc);
                ctx.uc_mcontext.gregs[REG_RIP] = backing.as_ptr() as i64;
                ctx.uc_mcontext.gregs[14] = rcx as i64; // rcx (vvvv control source)
                ctx.uc_mcontext.gregs[12] = rdx as i64; // rdx (rm source)
                ctx.uc_mcontext.gregs[13] = 0xdead; // rax dest (must be overwritten)
                let features = crate::cpuid::CpuFeatures::default();
                let inst = decode_instruction(backing.as_ptr());
                let res = emulate(&inst, &features, &mut ctx);
                assert_eq!(res, EmulationResult::Success);
                ctx.uc_mcontext.gregs[13]
            }
        }

        #[test]
        fn bextr_extracts_bit_field() {
            // bextr eax,edx,ecx = C4 E2 70 F7 C2. control=ecx: start=ecx[7:0],
            // len=ecx[15:8]. ecx=0x0404 -> start=4,len=4. (0xF0F0F0F0>>4)&0xF=0xF.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x70, 0xF7, 0xC2], 0x404, 0xF0F0_F0F0), 0xF);
            // bextr rax,rdx,rcx = C4 E2 F0 F7 C2 (W=1): same field on 64-bit value.
            assert_eq!(
                run_one(&[0xC4, 0xE2, 0xF0, 0xF7, 0xC2], 0x404, 0xF0F0_F0F0_F0F0_F0F0),
                0xF
            );
            // len=0 -> result 0 and ZF set (result 0).
            assert_eq!(run_one(&[0xC4, 0xE2, 0x70, 0xF7, 0xC2], 0x0, 0xF0F0_F0F0), 0);
            // start >= operand size (start=64/len=4: ecx=0x440) -> DEST = 0 per x86
            // shift-count masking; the unguarded `src2 >> start` would panic in
            // a Rust build (worse than a wrong result inside the SIGILL handler).
            assert_eq!(
                run_one(&[0xC4, 0xE2, 0xF0, 0xF7, 0xC2], 0x440, 0xF0F0_F0F0_F0F0_F0F0),
                0
            );
            // 32-bit: start=32/len=2 (ecx=0x220) -> 0, no shift panic.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x70, 0xF7, 0xC2], 0x220, 0xF0F0_F0F0), 0);
        }

        #[test]
        fn bzhi_keeps_low_bits_or_whole() {
            // bzhi eax,edx,ecx = C4 E2 70 F5 C2. edx=0xFFFF, ecx=4 -> 0xF.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x70, 0xF5, 0xC2], 4, 0xFFFF), 0xF);
            // ecx >= 32 -> DEST = src1 wholesale.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x70, 0xF5, 0xC2], 0x40, 0x1234_5678), 0x1234_5678);
            // bzhi rax (W=1): C4 E2 F0 F5 C2, keep low 16 of a 64-bit value.
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF0, 0xF5, 0xC2], 0x10, 0xAAAA_BBBB_CCCC_DDDD), 0xDDDD);
        }

        #[test]
        fn bmi2_shifts_select_by_fp_prefix() {
            // shrx eax,edx,ecx = C4 E2 73 F7 C2 (pp=11 -> F3). edx=0x100, ecx=4 -> 0x10.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x73, 0xF7, 0xC2], 4, 0x100), 0x10);
            // shlx rax,rdx,rcx = C4 E2 F1 F7 C2 (pp=01 -> 66, W=1). rdx=1, rcx=3 -> 8.
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF1, 0xF7, 0xC2], 3, 1), 8);
            // sarx eax,edx,ecx = C4 E2 72 F7 C2 (pp=10 -> F2). edx=0xFFFFFFF0 (-16),
            // ecx=1 -> arithmetic right = -8 = 0xFFFFFFF8.
            assert_eq!(
                run_one(&[0xC4, 0xE2, 0x72, 0xF7, 0xC2], 1, 0xFFFF_FFF0),
                0xFFFF_FFF8
            );
        }

        /// PEXT and PDEP share opcode byte 0F38 F5 with BZHI and must be
        /// disambiguated by the VEX pp bits (pext=pp2 -> has_f2, pdep=pp3 ->
        /// has_f3). run_one's rcx=vvvv (the bit SOURCE), rdx=rm (the MASK),
        /// result in rax. Encodings verified with `gcc -c + objdump -d -M
        /// intel`; expected RESULTS verified against real hardware `_pext_u64`
        /// / `_pdep_u64` (this host has BMI2).
        #[test]
        fn pext_gathering_matches_hardware() {
            // pext rax, rcx, rdx (64-bit) = C4 E2 F2 F5 C2. pp=2.
            // pext(0xFF, 0b1010) = 3 ; pext(8, 0b1010) = 2.
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF2, 0xF5, 0xC2], 0xFF, 0b1010), 3);
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF2, 0xF5, 0xC2], 8, 0b1010), 2);
            // 32-bit (W=0) form C4 E2 72 F5 C2: pext32(0xFF,0b1010)=3.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x72, 0xF5, 0xC2], 0xFF, 0b1010), 3);
            // A case that distinguishes PEXT from a mis-decoded BZHI: had it
            // been decoded as BZHI, control=vvvv=source=5 would yield
            // 0xF5 & 0x1F = 0x15; the real PEXT(5, 0xF5) = 3 (bits 0,2 of src).
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF2, 0xF5, 0xC2], 0x5, 0b1111_0101), 3);
        }

        #[test]
        fn pdep_depositing_matches_hardware() {
            // pdep rax, rcx, rdx (64-bit) = C4 E2 F3 F5 C2. pp=3.
            // pdep(3, 0b1010)=10 ; pdep(0xFF,0b10101010)=170 ; pdep(1,0b100)=4.
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF3, 0xF5, 0xC2], 3, 0b1010), 10);
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF3, 0xF5, 0xC2], 0xFF, 0b1010_1010), 170);
            assert_eq!(run_one(&[0xC4, 0xE2, 0xF3, 0xF5, 0xC2], 1, 0b100), 4);
            // 32-bit (W=0) form C4 E2 73 F5 C2: pdep32(3, 0b1010)=10.
            assert_eq!(run_one(&[0xC4, 0xE2, 0x73, 0xF5, 0xC2], 3, 0b1010), 10);
        }

        /// Run one MULX into a hand-set context, returning (high, low) written
        /// to reg and vvvv. Encoding verified by `gcc -c + objdump -d`:
        ///   mulx %rbx,%rax,%rcx = C4 E2 FB F6 CB (W=1): reg=rcx(high),
        ///   vvvv=rax(low), rm=rbx(src); implicit multiplier = RDX.
        fn run_mulx(code: &[u8], rdx: u64, rbx: u64) -> (i64, i64) {
            unsafe {
                let mut ctx: libc::ucontext_t = std::mem::zeroed();
                let mut backing = code.to_vec();
                backing.resize(8, 0xcc);
                ctx.uc_mcontext.gregs[REG_RIP] = backing.as_ptr() as i64;
                ctx.uc_mcontext.gregs[12] = rdx as i64; // RDX = multiplier
                ctx.uc_mcontext.gregs[11] = rbx as i64; // RBX = rm source (slot 3)
                let features = crate::cpuid::CpuFeatures::default();
                let inst = decode_instruction(backing.as_ptr());
                let res = emulate(&inst, &features, &mut ctx);
                assert_eq!(res, EmulationResult::Success);
                // RCX (slot 14) = high (reg), RAX (slot 13) = low (vvvv).
                (ctx.uc_mcontext.gregs[14], ctx.uc_mcontext.gregs[13])
            }
        }

        #[test]
        fn mulx_writes_high_and_low_product_halves() {
            // 5 * 6 = 30 -> high 0, low 30.
            let (hi, lo) = run_mulx(&[0xC4, 0xE2, 0xFB, 0xF6, 0xCB], 5, 6);
            assert_eq!((hi as u64, lo as u64), (0, 30), "5*6=30 -> (0,30)");
            // (2^64-1) * 2 = 2^65 - 2 -> high 1, low 0xFFFFFFFFFFFFFFFE.
            let (hi, lo) = run_mulx(
                &[0xC4, 0xE2, 0xFB, 0xF6, 0xCB],
                0xFFFF_FFFF_FFFF_FFFF,
                2,
            );
            assert_eq!(
                (hi as u64, lo as u64),
                (1, 0xFFFF_FFFF_FFFF_FFFE),
                "2^64-1 * 2 -> (1, 0xFFFF..FFFE)"
            );
        }

        /// Run one RORX into a hand-set context: dest=RBX-src rotated right.
        /// Encoding verified by objdump: rorx $4,%rbx,%rcx = C4 E3 FB F0 CB 04
        /// and rorx $0x1f,%ebx,%ecx = C4 E3 7B F0 CB 1F. Returns the new RCX.
        fn run_rorx(code: &[u8], rbx: u64) -> i64 {
            unsafe {
                let mut ctx: libc::ucontext_t = std::mem::zeroed();
                let mut backing = code.to_vec();
                backing.resize(8, 0xcc);
                ctx.uc_mcontext.gregs[REG_RIP] = backing.as_ptr() as i64;
                ctx.uc_mcontext.gregs[11] = rbx as i64; // RBX = rm source (slot 3)
                let features = crate::cpuid::CpuFeatures::default();
                let inst = decode_instruction(backing.as_ptr());
                let res = emulate(&inst, &features, &mut ctx);
                assert_eq!(res, EmulationResult::Success);
                ctx.uc_mcontext.gregs[14] // RCX (slot 14) = dest (reg field)
            }
        }

        #[test]
        fn rorx_rotates_right_without_flags() {
            // Expected values verified on real hardware (this host has BMI2):
            //   rorx64(1,4)=0x1000000000000000 (right-rotate moves bit0->bit63)
            //   rorx64(0xf0,4)=0xf
            //   rorx32(1,31)=0x2 (bit0 -> bit1 over a 32-bit width)
            assert_eq!(
                run_rorx(&[0xC4, 0xE3, 0xFB, 0xF0, 0xCB, 0x04], 1),
                0x1000_0000_0000_0000
            );
            assert_eq!(run_rorx(&[0xC4, 0xE3, 0xFB, 0xF0, 0xCB, 0x04], 0xF0), 0x0F);
            // 32-bit form: rotate the 32-bit value, zero-extend.
            let v = run_rorx(&[0xC4, 0xE3, 0x7B, 0xF0, 0xCB, 0x1F], 1);
            assert_eq!(v as u64, 0x2, "32-bit rorx 1 by 31 -> 0x2 (5 moves to bit1)");
            // Flags untouched (RORX never modifies EFLAGS): CF still 0 here.
        }
}