// SPDX-License-Identifier: MIT
//
// Per-instruction ARM64 -> x86-64 translation.
//
// Model: guest registers live in a spilled `CpuState`. The translated prologue
// loads the CpuState address into RBX (callee-saved). Each guest register i is
// addressed as `[RBX + 8*i]`. Operands are re-loaded from state on every
// instruction (no live-range tracking / register allocation). Scratch regs:
// RAX, RCX, R10, R11, RDI.
//
// Guest address == host address (shared address space), so a guest load/store
// dereferences the host pointer held in the guest register directly.
//
// NZCV is not consumed yet; set-flag ops are still wired through arithmetic but
// the flags result is only written back as a placeholder.

use crate::decode::{Inst, ShiftKind};
use crate::x86::{CodeBuf, RAX, RBX, RCX, RDX, RDI, R10};

/// Byte offset of guest register g inside CpuState (x[g] at 8*g).
#[inline]
fn slot(g: u32) -> i32 {
    (g as i32) * 8
}

/// Load guest reg `g` into x86 reg `x`.
#[inline]
fn ldg(buf: &mut CodeBuf, x: u8, g: u32) {
    buf.mov_load64(x, RBX, slot(g));
}
/// Store x86 reg `x` into guest reg `g`.
#[inline]
fn stg(buf: &mut CodeBuf, g: u32, x: u8) {
    buf.mov_store64(RBX, slot(g), x);
}

/// Write 0 to guest register `g`.
#[inline]
fn stg0(buf: &mut CodeBuf, g: u32) {
    buf.mov_ri64(RAX, 0);
    buf.mov_store64(RBX, slot(g), RAX);
}

/// Byte offset of `CpuState.nzcv` (after pc@256: nzcv u32 at 264).
const NZCV_OFF: i32 = 8 * 32 + 8; // 264
/// Byte offset of `CpuState.pad`.
#[allow(dead_code)]
const PAD_OFF: i32 = 8 * 32 + 12; // 268

/// Convert the *live* x86 status flags (CF/ZF/SF/OF set by the last arithmetic
/// instruction) into a packed AArch64 NZCV u32 (N=31,Z=30,C=29,V=28) and store
/// it at `CpuState.nzcv`. Uses RAX/RCX/RDX as scratch. Must be called right
/// after the flag-setting op, before any flag-clobbering instruction.
fn store_nzcv(buf: &mut CodeBuf) {
    // push rax, rcx, rdx then snapshot eflags via pushfq.
    buf.push(RAX);
    buf.push(RCX);
    buf.push(RDX);
    buf.pushfq();
    buf.pop(RAX); // eax = rflags: CF0, PF2, AF4, ZF6, SF7, OF11
    // Build nzcv into RDX.
    buf.xor_rr64(RDX, RDX);
    // C = CF(eax bit0) -> nzcv bit29
    buf.mov_rr64(RCX, RAX);
    buf.and_ri64(RCX, 1);
    buf.shl_ri8(RCX, 29);
    buf.or_rr64(RDX, RCX);
    // V = OF(eax bit 11) -> nzcv bit 28
    buf.mov_rr64(RCX, RAX);
    buf.shr_ri8(RCX, 11);
    buf.and_ri64(RCX, 1);
    buf.shl_ri8(RCX, 28);
    buf.or_rr64(RDX, RCX);
    // Z = ZF(eax bit 6) -> nzcv bit 30
    buf.mov_rr64(RCX, RAX);
    buf.shr_ri8(RCX, 6);
    buf.and_ri64(RCX, 1);
    buf.shl_ri8(RCX, 30);
    buf.or_rr64(RDX, RCX);
    // N = SF(eax bit 7) -> nzcv bit 31
    buf.mov_rr64(RCX, RAX);
    buf.shr_ri8(RCX, 7);
    buf.and_ri64(RCX, 1);
    buf.shl_ri8(RCX, 31);
    buf.or_rr64(RDX, RCX);
    buf.mov_store32(RBX, NZCV_OFF, RDX);
    buf.pop(RDX);
    buf.pop(RCX);
    buf.pop(RAX);
}

/// Load the *stored* `CpuState.nzcv` into the real x86 rflags (CF/ZF/SF/OF) so
/// a following native `jcc`/`cmovcc` (via `x86_cc_for_cond`) evaluates the
/// AArch64 condition correctly, even when the immediately preceding op did not
/// set the flags (the dispatcher reloads operands, clobbering them).
///
/// Restores RAX/RCX, then sets flags via popfq as the LAST flag-clobbering op;
/// the consuming `jcc`/`cmovcc` MUST run immediately after. RDX is clobbered.
fn load_nzcv_to_eflags(buf: &mut CodeBuf) {
    buf.push(RAX);
    buf.push(RCX);
    buf.mov_load32(RCX, RBX, NZCV_OFF); // RCX = packed nzcv
    // RDX accumulates the eflags image; RAX is scratch. Build
    // eflags = { CF:nzcv.29, ZF:nzcv.30, SF:nzcv.31, OF:nzcv.28 }.
    buf.mov_ri64(RDX, 0x202); // reserved rflags bit1
    // C -> CF(bit0)
    buf.mov_rr64(RAX, RCX);
    buf.shr_ri8(RAX, 29);
    buf.and_ri64(RAX, 1);
    buf.or_rr64(RDX, RAX);
    // V -> OF(bit11)
    buf.mov_rr64(RAX, RCX);
    buf.shr_ri8(RAX, 28);
    buf.and_ri64(RAX, 1);
    buf.shl_ri8(RAX, 11);
    buf.or_rr64(RDX, RAX);
    // Z -> ZF(bit6), N -> SF(bit7) via one shift by 24 (nzcv bits 30,31 -> 6,7)
    buf.mov_rr64(RAX, RCX);
    buf.shr_ri8(RAX, 24);
    buf.and_ri64(RAX, 0xc0); // bits 6,7
    buf.or_rr64(RDX, RAX);
    // RDX holds the final eflags value. Restore saved RAX/RCX (pops do not use
    // RDX), then push the eflags and pop them into rflags. The popfq is the last
    // flag-clobbering op; the calling jcc/cmovcc must follow immediately.
    buf.pop(RCX);
    buf.pop(RAX);
    buf.push(RDX);
    buf.popfq();
}

/// Constant `val` into guest reg `rd`.
fn mov_guest_imm(buf: &mut CodeBuf, rd: u32, val: u64) {
    // Use imm32 (sign-extended) when the upper 32 bits equal the sign-extension
    // of the low 32 bits; otherwise materialize the full 64-bit constant.
    let lo = val as u32 as i32;
    if (lo as i64 as u64) == val {
        buf.mov_ri32(RAX, lo as u32);
    } else {
        buf.mov_ri64(RAX, val);
    }
    stg(buf, rd, RAX);
}

/// Map an ARM condition code (0..15) to the x86-64 `0F 8x` jcc opcode, on the
/// assumption that the immediately preceding instruction set the x86 flags in
/// the ARM flow (SUB yields borrow semantics, so ARM carry == x86 C-free).
fn x86_cc_for_cond(cond: u8) -> Option<u8> {
    Some(match cond {
        0x0 => 0x84, // EQ  (ZF)
        0x1 => 0x85, // NE  (!ZF)
        0x2 => 0x83, // HS  (C set; unsigned >= -> !CF, JAE)
        0x3 => 0x82, // LO  (C clear; unsigned <  -> sub CF, JB)
        0x4 => 0x88, // MI  (N)
        0x5 => 0x89, // PL  (!N)
        0x6 => 0x8a, // VS  (V)
        0x7 => 0x8b, // VC  (!V)
        0x8 => 0x87, // HI  (C && !Z -> JAE && !ZF, i.e. JA)
        0x9 => 0x86, // LS  (!HI  -> JBE)
        0xA => 0x8d, // GE  (signed >=, JGE)
        0xB => 0x8c, // LT  (signed <, JL)
        0xC => 0x8f, // GT  (signed >, JG)
        0xD => 0x8e, // LE  (signed <=, JLE)
        _ => return None,
    })
}

/// Apply AArch64 shift `kind` by `amt` to the value currently in x86 reg `x`
/// (uses RCX for the count). Only constant shifts are handled (guest encodes
/// the amount as an immediate in ADD/SUB shifted-register).
fn apply_shift_const(buf: &mut CodeBuf, x: u8, kind: ShiftKind, amt: u8) {
    if amt == 0 {
        return;
    }
    // mov cl, amt ; then shl/shr/sar r64, cl
    buf.mov_ri32(RCX, amt as u32); // mov ecx, amt  (zero-extends), low byte CL
    match kind {
        ShiftKind::Lsl => buf.shl_cl64(x),
        ShiftKind::Lsr => buf.shr_cl64(x),
        ShiftKind::Asr => buf.sar_cl64(x),
        ShiftKind::Ror => buf.ror_cl64(x),
    }
}

/// A branch/jump fixup: the guest target PC and the byte offset within the
/// emitted buffer where the rel32 displacement field lives. Resolved once the
/// buffer is laid out (jit.rs patches it to the host offset of the target).
#[derive(Debug, Clone, Copy)]
pub struct Fixup {
    pub target_pc: u64,
    pub disp_off: usize,
    /// encoded x86 jcc condition (0x84=JZ) or 0 for an unconditional jmp,
    /// `0xff` means unconditional-jump fixup (E9).
    pub cc: u8,
}

/// Translate a single instruction (writes to `buf`). `pc` is the guest PC of
/// this instruction (needed for PC-relative branch targets). Branch
/// instructions append a `Fixup` to `out` so the JIT can patch their target.
pub fn translate(
    buf: &mut CodeBuf,
    pc: u64,
    inst: Inst,
    fixups: &mut Vec<Fixup>,
) -> Result<(), String> {
    match inst {
        Inst::MoveWide {
            rd, imm16, hw, opc, ..
        } => {
            let val = (imm16 as u64) << ((hw as u64) * 16);
            // opc: 0=movz,1=movk,2=movn. movz/movk write imm (movk merges later,
            // treated as movz for the first pass).
            if opc == 2 {
                mov_guest_imm(buf, rd as u32, !val);
            } else {
                mov_guest_imm(buf, rd as u32, val);
            }
            Ok(())
        }
        Inst::AddSubImm {
            rd,
            rn,
            imm12,
            shift12,
            sub,
            s,
            ..
        } => {
            let imm: u64 = (imm12 as u64) << if shift12 { 12 } else { 0 };
            ldg(buf, RAX, rn as u32);
            if sub {
                buf.sub_ri64(RAX, imm as u32);
            } else {
                buf.add_ri64(RAX, imm as u32);
            }
            if s {
                store_nzcv(buf); // N/Z/C/V -> CpuState.nzcv
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::AddSubReg {
            rd,
            rn,
            rm,
            sub,
            s,
            shift,
            sh_amt,
            ..
        } => {
            ldg(buf, RAX, rn as u32);
            ldg(buf, RCX, rm as u32);
            apply_shift_const(buf, RCX, shift, sh_amt);
            if sub {
                buf.sub_rr64(RAX, RCX);
            } else {
                buf.add_rr64(RAX, RCX);
            }
            if s {
                store_nzcv(buf);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::LogicReg {
            rd,
            rn,
            rm,
            op,
            s,
            shift,
            sh_amt,
            sf,
        } => {
            let _ = sf; // operand size handled by existing emitters; sf informative
            // rn == 31 (XZR) reads as zero (common for the `mov xd, xm` alias
            // `orr xd, xzr, xm`); otherwise load rn.
            if rn == 31 {
                buf.mov_ri64(RAX, 0);
            } else {
                ldg(buf, RAX, rn as u32);
            }
            ldg(buf, RCX, rm as u32);
            apply_shift_const(buf, RCX, shift, sh_amt);
            match op {
                0 => buf.and_rr64(RAX, RCX), // AND
                1 => buf.or_rr64(RAX, RCX),  // ORR
                2 => buf.xor_rr64(RAX, RCX), // EOR
                // Inverted (N=1) variants: AND/NOT, OR/NOT, XOR/NOT (BIC/ORN/EON).
                4 => {
                    buf.not_r64(RCX);
                    buf.and_rr64(RAX, RCX); // BIC
                }
                5 => {
                    buf.not_r64(RCX);
                    buf.or_rr64(RAX, RCX); // ORN
                }
                6 => {
                    buf.not_r64(RCX);
                    buf.xor_rr64(RAX, RCX); // EON
                }
                _ => return Err(format!("LogicReg op {} not implemented", op)),
            }
            if s {
                            // ANDS/ORRS/EORS/TST set NZCV: x86 `and/or/xor` set CF=0,OF=0 and
                            // ZF/SF from the result, which is exactly AArch64's N/Z/C/V here.
                            store_nzcv(buf);
                        }
                        if rd != 31 {
                            stg(buf, rd as u32, RAX);
                        }
                        Ok(())
                    }
                    Inst::LogicImm {
                        rd,
                        rn,
                        mask,
                        op,
                        sf,
                    } => {
                        // AND/ORR/EOR/ANDS with a bitmask immediate (the `mov xD,#imm` alias
                        // is ORR xD, xzr, #imm). Load Rn into RAX, materialize `mask` in
                        // RCX, combine, then (for op==3 / tst) store NZCV.
                        if rn == 31 {
                            buf.mov_ri64(RAX, 0); // xzr reads as zero
                        } else {
                            ldg(buf, RAX, rn as u32);
                        }
                        buf.mov_ri64(RCX, mask);
                        match op {
                            0 => buf.and_rr64(RAX, RCX), // AND
                            1 => buf.or_rr64(RAX, RCX),  // ORR
                            2 => buf.xor_rr64(RAX, RCX), // EOR
                            3 => {
                                buf.and_rr64(RAX, RCX); // ANDS
                                store_nzcv(buf);
                            }
                            _ => return Err(format!("LogicImm op {} not implemented", op)),
                        }
                        if !sf {
                            buf.and_ri64(RAX, 0xffff_ffff);
                        }
                        if rd != 31 {
                            stg(buf, rd as u32, RAX);
                        }
                        Ok(())
                    }
                    Inst::MulDiv { div, signed, rd, rn, rm, ra, sf } => {
                        if div {
                            // UDIV/SDIV: RAX = Rn / Rm (quotient). Dividend in
                            // RDX:RAX, divisor in RCX.
                            ldg(buf, RAX, rn as u32); // dividend low half
                            if signed {
                                // sign-extend the 32-bit W operand to 64 (for X
                                // operands already loaded sign-correct; movsxd of a
                                // 64-bit value's low 32 would corrupt it, so only
                                // re-extend for W).
                                if !sf {
                                    buf.movsxd_r64_r32(RAX, RAX);
                                }
                                buf.cqo(); // RAX -> RDX:RAX (signed)
                            } else {
                                buf.xor_rr64(RDX, RDX); // unsigned: zero-high half
                            }
                            // divisor: sign-extend Rm for SDIV-W too.
                            ldg(buf, RCX, rm as u32);
                            if signed && !sf {
                                buf.movsxd_r64_r32(RCX, RCX);
                            }
                            if signed {
                                buf.idiv_r64(RCX);
                            } else {
                                buf.div_r64(RCX);
                            }
                            // quotient in RAX. Store (W: low 32 preserved by div if no overflow).
                        } else {
                            // MADD/MSUB: RAX = Rn*rm [+/-] ra.
                            ldg(buf, RAX, rn as u32);
                            ldg(buf, RCX, rm as u32);
                            buf.imul_rr64(RAX, RCX); // RAX = Rn*rm (low 64)
                            ldg(buf, RDI, ra as u32);
                            if signed {
                                buf.sub_rr64(RAX, RDI); // Rn*rm - ra
                            } else {
                                buf.add_rr64(RAX, RDI); // Rn*rm + ra
                            }
                        }
                        if !sf {
                            buf.and_ri64(RAX, 0xffff_ffff);
                        }
                        if rd != 31 {
                            stg(buf, rd as u32, RAX);
                        }
                        Ok(())
                    }
        Inst::CSel {
            rd,
            rn,
            rm,
            cond,
            op,
            sf,
        } => {
            // `csel rd, rn, rm, c`: rd = c ? rn : f(rm), where f applies the
            // csinc/csinv/csneg transform to rm (op 0=identity,1=+1,2=~,3=-).
            // then-branch value = rn (RDI), else-branch = f(rm) (R10). Compute
            // both before restoring the flags from NZCV (load_nzcv_to_eflags
            // sets them last, and cmovcc reads them immediately after).
            let _ = sf;
            ldg(buf, RDI, rn as u32); // then: rn
            ldg(buf, R10, rm as u32); // else: f(rm)
            match op {
                0 => {}
                1 => buf.add_ri64(R10, 1),   // csinc / cset / cinc
                2 => buf.not_r64(R10),       // csinv
                3 => buf.neg_r64(R10),       // csneg
                _ => return Err(format!("CSel op {} not implemented", op)),
            }
            if cond == 0xE {
                // AL: unconditional — just rn
                if rd != 31 {
                    stg(buf, rd as u32, RDI);
                }
                return Ok(());
            }
            if cond == 0xF {
                // NV: never — just f(rm)
                if rd != 31 {
                    stg(buf, rd as u32, R10);
                }
                return Ok(());
            }
            let cc = (x86_cc_for_cond(cond)
                .ok_or_else(|| format!("CSel: bad cond {cond:#x}"))?
                - 0x40); // jcc 0x8X -> cmovcc 0x4X (subtract the 0x80 top byte)
            load_nzcv_to_eflags(buf);
            buf.cmov_rr64(cc, R10, RDI); // R10 = cond ? RDI(rn) : R10(f(rm))
            if rd != 31 {
                stg(buf, rd as u32, R10);
            }
            Ok(())
        }
        Inst::LdStrImm {
            rt,
            rn,
            imm,
            size,
            ld,
        } => {
            // address = rn + imm*size (scaled byte offset)
            ldg(buf, RDX, rn as u32); // pointer operand into RDX
            let off = (imm as i32).checked_mul(size as i32).unwrap_or(0);
            if off != 0 {
                buf.lea64(RDX, RDX, off);
            }
            match (size, ld) {
                (8, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (8, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (4, true) => {
                    buf.mov_load32(RAX, RDX, 0); // w zero-extends
                    stg(buf, rt as u32, RAX);
                }
                (4, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0); // ldrh zero-extends
                    stg(buf, rt as u32, RAX);
                }
                (2, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (1, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0); // ldrb zero-extends
                    stg(buf, rt as u32, RAX);
                }
                (1, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store8(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("LdStrImm size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::AcqRel { size, ld, rt, rn } => {
            // LDAR/STLR ordering is a no-op in a single-threaded JIT; act as a
            // plain load/store of `size` bytes at [Rn].
            ldg(buf, RDX, rn as u32);
            match (size, ld) {
                (3, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (3, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (2, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (1, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (1, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (0, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (0, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store8(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("AcqRel size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::LdExr { size, ld, rs, rt, rn } => {
            // Exclusive block in a single-threaded JIT always succeeds: `ldxr` is
            // a plain load, `stxr` is a plain store with the status register Rs
            // written 0 (success).
            ldg(buf, RDX, rn as u32);
            match (size, ld) {
                (3, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (3, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (2, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (2, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (1, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (1, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (0, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (0, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store8(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (s, _) => return Err(format!("LdExr size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::BitField { rd, rn, immr, imms, sf, arith } => {
            let bits = if sf { 64u32 } else { 32u32 };
            ldg(buf, RAX, rn as u32); // load Rn
            if imms == bits - 1 {
                // LSR (logical) or ASR (arithmetic/sign) by immr
                let sh = (immr & (bits - 1)) as u8;
                if arith {
                    buf.sar_ri8(RAX, sh);
                } else {
                    buf.shr_ri8(RAX, sh);
                }
            } else if immr == (imms + 1) % bits {
                // LSL (alias) : shift left by (bits-1-imms)
                let sh = ((bits - 1 - imms) & (bits - 1)) as u8;
                buf.shl_ri8(RAX, sh);
            } else if immr > imms {
                // BFI/BFC insert: lsb = (bits-immr)&(bits-1); width = imms+1.
                // rd = (rd & ~mask) | ((Rn << lsb) & mask); mask = ((1<<width)-1)<<lsb
                let lsb = (bits - immr) & (bits - 1);
                let width = imms + 1;
                let mask = ((1u64 << width) - 1) << lsb;
                ldg(buf, R10, rd as u32); // old Rd
                buf.shl_ri8(RAX, lsb as u8); // Rn << lsb
                buf.mov_ri64(RCX, mask);
                buf.and_rr64(RAX, RCX); // inserted field
                buf.not_r64(RCX); // ~mask
                buf.and_rr64(R10, RCX); // rd & ~mask
                buf.or_rr64(R10, RAX); // (rd & ~mask) | inserted
                buf.mov_rr64(RAX, R10);
            } else {
                // general UBFM/SBFM extract: (Rn >> immr) & low(width) bits,
                // then optionally sign-extend from `width`.
                let width = imms - immr + 1;
                let sh = (immr & (bits - 1)) as u8;
                buf.shr_ri8(RAX, sh); // drop low immr bits
                // keep only `width` low bits
                if width < bits {
                    let mask: u64 = (1u64 << width) - 1;
                    if mask & 0xffff_ffff == mask {
                        buf.and_ri64(RAX, mask as u32);
                    } else {
                        buf.mov_ri64(RCX, mask);
                        buf.and_rr64(RAX, RCX);
                    }
                }
                if arith {
                    // sign-extend the `width`-bit field to `bits`:
                    // shift left to push the sign bit to the top, then arithmetic
                    // shift right back (replicates the sign).
                    let se = (bits - width) as u8;
                    buf.shl_ri8(RAX, se);
                    buf.sar_ri8(RAX, se);
                }
            }
            if !sf {
                buf.and_ri64(RAX, 0xffff_ffff);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::SysReg { sysreg, rt, read } => {
            // Only tpidr_el0 is modelled (sysreg==0). MRS read: Rt = CpuState.tpidr.
            // MSR write: CpuState.tpidr = Rt. Other sysregs should not decode here.
            debug_assert_eq!(sysreg, 0, "unhandled SysReg in translate");
            if read {
                // Rt = [RBX + TPIDR_OFF]
                if rt != 31 {
                    buf.mov_load64(rt, RBX, crate::jit::TPIDR_OFF);
                }
            } else {
                // tpidr_el0 = Rt
                if rt != 31 {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RBX, crate::jit::TPIDR_OFF, RAX);
                }
            }
            Ok(())
        }
        Inst::FpScalar { rd, rn, rm, op, sz } => {
            // scalar FP on d/s regs. d-reg = low 8 bytes of CpuState.v[reg].slot
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16; // low 8B of a 16B slot
            if !sz {
                return Err(format!("FpScalar single-precision (sz=0) not implemented (op {op})"));
            }
            match op {
                0 => {
                    // fmov d, d: copy 64-bit int
                    buf.mov_load64(RAX, RBX, vslot(rn));
                    buf.mov_store64(RBX, vslot(rd), RAX);
                }
                1 => {
                    // fabs: clear sign bit (bit63)
                    buf.mov_load64(RAX, RBX, vslot(rn));
                    buf.mov_ri64(RCX, 0x7fff_ffff_ffff_ffff);
                    buf.and_rr64(RAX, RCX);
                    buf.mov_store64(RBX, vslot(rd), RAX);
                }
                2 => {
                    // fneg: flip sign bit
                    buf.mov_load64(RAX, RBX, vslot(rn));
                    buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                    buf.xor_rr64(RAX, RCX);
                    buf.mov_store64(RBX, vslot(rd), RAX);
                }
                4 | 5 | 6 | 7 => {
                    buf.movq_load(0, RBX, vslot(rn));
                    buf.movq_load(1, RBX, vslot(rm));
                    match op {
                        4 => buf.mulsd(0, 1),
                        5 => buf.addsd(0, 1),
                        6 => buf.subsd(0, 1),
                        7 => buf.divsd(0, 1),
                        _ => unreachable!(),
                    }
                    buf.movq_store(RBX, vslot(rd), 0);
                }
                _ => return Err(format!("FpScalar op {op} not implemented")),
            }
            Ok(())
        }
        Inst::FcvtToInt { rd, rn, mode, sf } => {
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.movq_load(0, RBX, vslot(rn)); // d-source (low 8B) -> xmm0
            if mode == 2 {
                buf.cvtsd2si(RAX, 0); // fcvtas: round to nearest (MXCSR, default even)
            } else {
                buf.cvttsd2si(RAX, 0); // fcvtzs: truncate toward zero
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::FmovGp { f, sz, rd, rn } => {
            // FMOV core <-> scalar FP. Double(d/x) uses the low 64 of the slot;
            // single(s/w) uses the low 32.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if f {
                            // FP -> GP
                            if sz {
                                buf.mov_load64(RAX, RBX, vslot(rn));
                            } else {
                                buf.mov_load32(RAX, RBX, vslot(rn));
                            }
                if rd != 31 {
                    stg(buf, rd as u32, RAX);
                }
            } else {
                // GP -> FP
                ldg(buf, RAX, rn as u32);
                if sz {
                    buf.mov_store64(RBX, vslot(rd), RAX);
                } else {
                    buf.mov_store32(RBX, vslot(rd), RAX);
                }
            }
            Ok(())
        }
        Inst::LdStPair {
            rt,
            rt2,
            rn,
            imm,
            ld,
            writeback,
            preidx,
            size_64,
            q128,
            fp_d,
        } => {
            let esize = if q128 {
                16i32
            } else if fp_d {
                8i32
            } else if size_64 {
                8i32
            } else {
                4i32
            };
            let imm32 = imm as i32;
            // eff base: pre-index adjusts the address by imm before the access;
            // post/offset use rn (post then adds imm for writeback).
            ldg(buf, RDX, rn as u32); // RDX = rn
            let (access_off, wb_off) = if preidx {
                (imm32, imm32) // access at rn+imm, then rn=rn+imm
            } else {
                (0i32, if writeback { imm32 } else { 0 }) // access at rn, wb adds imm
            };
            if fp_d {
                // 64-bit FP/vector d-pair: each reg is one u64 in CpuState.v
                // at VECTOR_BASE + rt*8; transfer via RAX.
                let v0 = crate::jit::VECTOR_BASE + (rt as i32) * 8;
                let v1 = crate::jit::VECTOR_BASE + (rt2 as i32) * 8;
                for (reg_off, mem_off) in [(v0, access_off), (v1, access_off + esize)] {
                    if ld {
                        buf.mov_load64(RAX, RDX, mem_off); // rax <- [addr]
                        buf.mov_store64(RBX, reg_off, RAX); // v <- rax
                    } else {
                        buf.mov_load64(RAX, RBX, reg_off); // rax <- v
                        buf.mov_store64(RDX, mem_off, RAX); // [addr] <- rax
                    }
                }
                if writeback {
                    ldg(buf, RAX, rn as u32);
                    buf.add_ri64(RAX, wb_off as u32);
                    stg(buf, rn as u32, RAX);
                }
                return Ok(());
            }
            if q128 {
                // 128-bit SIMD pair: transfer 16 bytes per register between the
                // guest v-slots (CpuState.v, VECTOR_BASE+16*reg) and memory via XMM0.
                let v0 = crate::jit::VECTOR_BASE + (rt as i32) * 16;
                let v1 = crate::jit::VECTOR_BASE + (rt2 as i32) * 16;
                for (reg_vslot, mem_off) in [(v0, access_off), (v1, access_off + esize)] {
                    if ld {
                        buf.movdqu_load(0, RDX, mem_off); // xmm0 <- [addr]
                        buf.movdqu_store(RBX, reg_vslot, 0); // guest v <- xmm0
                    } else {
                        buf.movdqu_load(0, RBX, reg_vslot); // xmm0 <- [vslot]
                        buf.movdqu_store(RDX, mem_off, 0); // [addr] <- xmm0
                    }
                }
            } else if ld {
                // load rt = [RDX + access_off], rt2 = [.. + esize]
                if size_64 {
                    buf.mov_load64(RAX, RDX, access_off);
                    stg(buf, rt as u32, RAX);
                    buf.mov_load64(RAX, RDX, access_off + esize);
                    stg(buf, rt2 as u32, RAX);
                } else {
                    buf.mov_load32(RAX, RDX, access_off);
                    stg(buf, rt as u32, RAX);
                    buf.mov_load32(RAX, RDX, access_off + esize);
                    stg(buf, rt2 as u32, RAX);
                }
            } else {
                // store rt at [eff], rt2 at [eff+esize]
                if size_64 {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RDX, access_off, RAX);
                    ldg(buf, RAX, rt2 as u32);
                    buf.mov_store64(RDX, access_off + esize, RAX);
                } else {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store32(RDX, access_off, RAX);
                    ldg(buf, RAX, rt2 as u32);
                    buf.mov_store32(RDX, access_off + esize, RAX);
                }
            }
            if writeback {
                // rn = rn + wb_off
                ldg(buf, RCX, rn as u32);
                if wb_off != 0 {
                    buf.lea64(RCX, RCX, wb_off);
                }
                stg(buf, rn as u32, RCX);
            }
            Ok(())
        }
        Inst::LdStrReg {
            rt,
            rn,
            rm,
            size,
            ld,
            shift,
        } => {
            // addr = rn + (rm << shift_amt), shift_amt = log2(size) when S=1.
            let shift_amt = if shift {
                match size {
                    1 => 0,
                    2 => 1,
                    4 => 2,
                    _ => 3,
                }
            } else {
                0
            };
            ldg(buf, RAX, rn as u32); // address base in RAX
            ldg(buf, RCX, rm as u32); // index in RCX
            if shift_amt != 0 {
                // Currently only constant <=3 via the (unused) sar_cl; emit shift left.
                // x86 has no shl-by-imm op in this emitter; use add-based *2 for 1..3.
                for _ in 0..shift_amt {
                    buf.add_rr64(RCX, RCX); // RCX += RCX (shift left by 1)
                }
            }
            buf.add_rr64(RAX, RCX); // RAX = effective address
            match (size, ld) {
                (8, true) => {
                    buf.mov_load64(RCX, RAX, 0);
                    stg(buf, rt as u32, RCX);
                }
                (8, false) => {
                    ldg(buf, RCX, rt as u32);
                    buf.mov_store64(RAX, 0, RCX);
                }
                (4, true) => {
                    buf.mov_load32(RCX, RAX, 0);
                    stg(buf, rt as u32, RCX);
                }
                (4, false) => {
                    ldg(buf, RCX, rt as u32);
                    buf.mov_store32(RAX, 0, RCX);
                }
                (2, true) => {
                    buf.movzx_word_mem(RCX, RAX, 0);
                    stg(buf, rt as u32, RCX);
                }
                (2, false) => {
                    ldg(buf, RCX, rt as u32);
                    buf.mov_store16(RAX, 0, RCX);
                }
                (1, true) => {
                    buf.movzx_byte_mem(RCX, RAX, 0);
                    stg(buf, rt as u32, RCX);
                }
                (1, false) => {
                    ldg(buf, RCX, rt as u32);
                    buf.mov_store8(RAX, 0, RCX);
                }
                (s, _) => return Err(format!("LdStrReg size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::VecLdStImm { vt, rn, imm, ld } => {
            // addr = rn + imm*16 ; transfer a full 128-bit vector between the
            // guest vector slot (CpuState.v, 16 bytes at VECTOR_BASE+16*vt) and
            // the guest pointer, via x86 XMM0.
            ldg(buf, RAX, rn as u32); // base address
            let off = (imm as i32).wrapping_mul(16);
            if off != 0 {
                buf.lea64(RAX, RAX, off);
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            if ld {
                buf.movdqu_load(0, RAX, 0); // xmm0 <- [addr]
                buf.movdqu_store(RBX, vslot, 0); // cmp-state v <- xmm0
            } else {
                buf.movdqu_load(0, RBX, vslot); // xmm0 <- [vslot]
                buf.movdqu_store(RAX, 0, 0); // [addr] <- xmm0
            }
            Ok(())
        }
        Inst::VecMovi { vd, lo, hi } => {
            // Write a full 128-bit vector immediate into the guest v-slot
            // (CpuState.v, 16 bytes at VECTOR_BASE + 16*vd). The two u64 halves
            // are hoisted as immediates.
            let vslot = crate::jit::VECTOR_BASE + (vd as i32) * 16;
            buf.mov_ri64(RAX, lo);
            buf.mov_store64(RBX, vslot, RAX);
            buf.mov_ri64(RAX, hi);
            buf.mov_store64(RBX, vslot + 8, RAX);
            Ok(())
        }
        Inst::Hint => {
            // Hint / PAC NOP — execute as a no-op (PAC is ignored in the guest).
            Ok(())
        }
        Inst::Br { rn } => {
            // pc = x[rn]; return to the host dispatcher (which re-enters at pc).
            ldg(buf, RAX, rn as u32);
            buf.mov_store64(RBX, crate::jit::PC_OFF, RAX);
            buf.ret();
            Ok(())
        }
        Inst::Blr { rn } => {
            // x30 = pc + 4 (link); pc = x[rn]; return to the host dispatcher.
            buf.mov_ri64(RAX, pc.wrapping_add(4));
            stg(buf, 30, RAX);
            ldg(buf, RAX, rn as u32);
            buf.mov_store64(RBX, crate::jit::PC_OFF, RAX);
            buf.ret();
            Ok(())
        }
        Inst::Ret => {
            // return x0 in RAX, and set guest pc = x30 (link address) so a host
            // dispatcher can resume at the caller. $[x0] at RBX+0, x30 at RBX+240.
            ldg(buf, RAX, 30);
            buf.mov_store64(RBX, crate::jit::PC_OFF, RAX);
            buf.mov_load64(RAX, RBX, 0);
            buf.ret();
            Ok(())
        }
        Inst::B { imm, link } => {
            if link {
                // BL: save guest LR = pc+4, then host `call` to the target block.
                // LR is stored so any guest read of x30 stays correct.
                let ra = pc.wrapping_add(4);
                buf.mov_ri64(RAX, ra);
                stg(buf, 30, RAX);
                let disp = buf.call_rel32();
                fixups.push(Fixup {
                    target_pc: pc.wrapping_add(imm as u64),
                    disp_off: disp,
                    cc: 0xfe, // call fixup
                });
            } else {
                let target = pc.wrapping_add(imm as u64);
                let disp = buf.jmp_rel32();
                fixups.push(Fixup {
                    target_pc: target,
                    disp_off: disp,
                    cc: 0xff,
                });
            }
            Ok(())
        }
        Inst::Adr { rd, imm } => {
            // rd = pc + imm (load the effective address of a nearby symbol)
            let target = (pc as i64).wrapping_add(imm);
            buf.mov_ri64(RAX, target as u64);
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Adrp { rd, imm } => {
            // rd = page(PC) + (imm<<12)  -- page-aligned effective address.
            let page = (pc as i64) & !0xfff_i64;
            let target = page.wrapping_add(imm);
            buf.mov_ri64(RAX, target as u64);
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Cbz {
            rt, imm, nonzero, ..
        } => {
            let target = pc.wrapping_add(imm as u64);
            ldg(buf, RAX, rt as u32); // test rt
            buf.test_rr64(RAX, RAX);
            let cc = if nonzero { 0x85 } else { 0x84 }; // jnz / jz
            let disp = buf.jcc_rel32(cc);
            fixups.push(Fixup {
                target_pc: target,
                disp_off: disp,
                cc,
            });
            Ok(())
        }
        Inst::Tbz {
            rt,
            bit,
            imm,
            nonzero,
            ..
        } => {
            let target = pc.wrapping_add(imm as u64);
            ldg(buf, RAX, rt as u32); // load rt
            // test the single bit: test rax, 1<<bit
            buf.mov_ri64(RCX, (1u64 << bit.min(63)) & (if bit >= 64 { 0 } else { 0xffff_ffff_ffff_ffff }));
            // simpler: AND with constant handled per-bit via a cached reg
            buf.test_rr64(RAX, RCX);
            // tbz: branch if bit==0 => JE when ZF set; tbnz: branch if bit==1 => JNE
            let cc = if nonzero { 0x85 } else { 0x84 }; // jnz / jz
            let disp = buf.jcc_rel32(cc);
            fixups.push(Fixup {
                target_pc: target,
                disp_off: disp,
                cc,
            });
            Ok(())
        }
        Inst::BCond { cond, imm } => {
            let target = pc.wrapping_add(imm as u64);
            match cond {
                0xE => {
                    // AL: unconditional branch via jmp
                    let disp = buf.jmp_rel32();
                    fixups.push(Fixup {
                        target_pc: target,
                        disp_off: disp,
                        cc: 0xff,
                    });
                }
                0xF => {
                    // NV: never executed -> nothing to emit
                }
                c => {
                    let cc = x86_cc_for_cond(c)
                        .ok_or_else(|| format!("B.cond unsupported cond {:x}", c))?;
                    // Evaluate the condition from the stored NZCV (the dispatcher
                    // may clobber live x86 flags with operand reloads; NZCV is
                    // the authoritative copy). load_nzcv_to_eflags sets the flags
                    // via popfq as the last clobbering op, so the jcc that
                    // follows reads exactly the guest condition.
                    load_nzcv_to_eflags(buf);
                    let disp = buf.jcc_rel32(cc);
                    fixups.push(Fixup {
                        target_pc: target,
                        disp_off: disp,
                        cc,
                    });
                }
            }
            Ok(())
        }
        _ => Err(format!(
            "translate: unhandled {inst:?} at guest pc 0x{pc:x}"
        )),
    }
}
