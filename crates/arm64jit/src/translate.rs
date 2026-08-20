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

    /// Pack a FP-comparison result (x86 flags from a prior `comisd`/`ucomisd`) into
    /// the guest NZCV (bit 31=N, 30=Z, 29=C, 28=V). AArch64 `fcmp` semantics from
    /// the x86 flags set by comisd:
    ///   A<B (ord): CF=1,PF=0,ZF=0 -> N0 Z0 C0 V0
    ///   A>B (ord): CF=0,ZF=0,PF=0 -> N0 Z0 C1 V0
    ///   A==B:       CF=0,PF=0,ZF=1 -> N0 Z1 C1 V0   (C=1 for ge)
    ///   unordered:  CF=1,PF=1,ZF=1 -> N0 Z1 C1 V1
    /// so  Z=ZF, V=PF, C=(!CF)|PF, N=0. Clobbers RAX/RCX/RDX.
    fn store_nzcv_fp(buf: &mut CodeBuf) {
        buf.push(RAX);
        buf.push(RCX);
        buf.push(RDX);
        buf.pushfq();
        buf.pop(RAX); // eax = rflags (CF0, PF2, ZF6)
        buf.xor_rr64(RDX, RDX);
        // V = PF(bit2) -> bit28
        buf.mov_rr64(RCX, RAX);
        buf.shr_ri8(RCX, 2);
        buf.and_ri64(RCX, 1);
        buf.shl_ri8(RCX, 28);
        buf.or_rr64(RDX, RCX);
        // C = (!CF) | PF -> bit29
        buf.mov_rr64(RCX, RAX);
        buf.and_ri64(RCX, 1); // CF
        buf.xor_ri64(RCX, 1); // !CF
        buf.mov_rr64(RDI, RAX);
        buf.shr_ri8(RDI, 2);
        buf.and_ri64(RDI, 1); // PF
        buf.or_rr64(RCX, RDI);
        buf.shl_ri8(RCX, 29);
        buf.or_rr64(RDX, RCX);
        // Z = ZF(bit6) -> bit30
        buf.mov_rr64(RCX, RAX);
        buf.shr_ri8(RCX, 6);
        buf.and_ri64(RCX, 1);
        buf.shl_ri8(RCX, 30);
        buf.or_rr64(RDX, RCX);
        // N = 0 (never set for a valid FP compare in these cases)
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
        Inst::ClzCls { rd, rn, sf, cls } => {
            // clz/cls Wd|Xd, Rn. CLZ via LZCNT (F3 0F BD /r), which returns the
            // count of leading zeros directly and matches AArch64's clz(x=0)=|bits|.
            // Host x86-64 (Haswell+) universally supports LZCNT.
            if cls {
                return Err("CLS (count leading sign) not implemented".into());
            }
            if rd == 31 {
                return Ok(());
            }
            ldg(buf, RAX, rn as u32);
            if !sf {
                buf.and_ri64(RAX, 0xffff_ffff);
            }
            if sf {
                buf.bytes.extend_from_slice(&[0x48, 0xf3, 0x0f, 0xbd, 0xc0]); // lzcnt rax, rax
            } else {
                buf.bytes.extend_from_slice(&[0xf3, 0x0f, 0xbd, 0xc0]); // lzcnt eax, eax
                                                                        // lzcnt eax zeroes the upper 32 (correct W zero-extend)
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Rev { rd, rn, op, sf } => {
            if rd == 31 {
                return Ok(());
            }
            ldg(buf, RAX, rn as u32);
            match op {
                2 => {
                    // rev: full byte reverse. W => 32-bit bswap, X => 64-bit bswap.
                    if sf {
                        buf.bswap_r64(RAX);
                    } else {
                        buf.bswap_r32(RAX); // 32-bit bswap zeroes upper half
                    }
                }
                1 => {
                    // rev16: reverse each adjacent byte pair (16-bit element).
                    // X: ((x & 0x00FF00FF00FF00FF) << 8) | ((x & 0xFF00FF00FF00FF00) >> 8)
                    if sf {
                        buf.mov_ri64(RCX, 0x00ff_00ff_00ff_00ff);
                        buf.and_rr64(RCX, RAX);          // even-lane bytes
                        buf.mov_rr64(RDX, RAX);
                        buf.mov_ri64(R10, 0xff00_ff00_ff00_ff00);
                        buf.and_rr64(RDX, R10);          // odd-lane bytes
                        buf.shl_ri8(RCX, 8);
                        buf.shr_ri8(RDX, 8);
                        buf.or_rr64(RAX, RCX);
                        buf.or_rr64(RAX, RDX);
                    } else {
                        // W (32-bit): two swaps.
                        buf.mov_rr64(RCX, RAX);
                        buf.and_ri64(RCX, 0x00ff_00ff);
                        buf.shl_ri8(RCX, 8);
                        buf.and_ri64(RAX, 0xff00_ff00);
                        buf.shr_ri8(RAX, 8);
                        buf.or_rr64(RAX, RCX);
                    }
                }
                3 => {
                    // rev32 (X only): swap the two 32-bit halves.
                    buf.ror_ri8(RAX, 32);
                }
                _ => {
                    // op==0 rbit: reverse all bits (SWAR byte/pair steps up to 64).
                    let steps: [(u8, u64); 6] = [
                        (1, 0x5555_5555_5555_5555),
                        (2, 0x3333_3333_3333_3333),
                        (4, 0x0f0f_0f0f_0f0f_0f0f),
                        (8, 0x00ff_00ff_00ff_00ff),
                        (16, 0x0000_ffff_0000_ffff),
                        (32, 0x0000_0000_ffff_ffff),
                    ];
                    let width = if sf { 6usize } else { 4usize };
                    for (k, (sh, m)) in steps.iter().take(width).enumerate() {
                        // t = ((x >> sh) & m) | ((x & m) << sh)
                        buf.mov_rr64(RCX, RAX);
                        buf.shr_ri8(RCX, *sh);
                        buf.and_ri64(RCX, *m as u32); // m low32 (archs: pre-32 steps fit u32)
                        buf.and_ri64(RAX, *m as u32);
                        buf.shl_ri8(RAX, (1u32 << k as u32) as u8);
                        buf.or_rr64(RAX, RCX);
                    }
                }
            }
            if !sf {
                buf.and_ri64(RAX, 0xffff_ffff);
            }
            stg(buf, rd as u32, RAX);
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
        Inst::FcsSel { rd, rn, rm, cond, sz } => {
            // fcsel d{rd}, d{rn}, d{rm}, <cond>: rd = cond ? rn : rm on the FP
            // slots. FP values are selected by their bit pattern (cmov on the
            // integer ref of the double/single), so the same register-select
            // machinery as the integer CSel applies.
            let true_slot = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let else_slot = crate::jit::VECTOR_BASE + (rm as i32) * 16;
            let dst_slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            if sz {
                buf.mov_load64(RDI, RBX, true_slot);
                buf.mov_load64(R10, RBX, else_slot);
            } else {
                buf.mov_load32(RDI, RBX, true_slot);
                buf.mov_load32(R10, RBX, else_slot);
            }
            if cond == 0xE {
                buf.mov_store64(RBX, dst_slot, RDI); // AL -> rn
                return Ok(());
            }
            if cond == 0xF {
                buf.mov_store64(RBX, dst_slot, R10); // NV -> rm
                return Ok(());
            }
            let cc = (x86_cc_for_cond(cond)
                .ok_or_else(|| format!("FcSel: bad cond {cond:#x}"))?
                - 0x40); // jcc 0x8X -> cmovcc 0x4X
            load_nzcv_to_eflags(buf);
            buf.cmov_rr64(cc, R10, RDI); // R10 = cond ? rn : rm
            buf.mov_store64(RBX, dst_slot, R10);
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
        Inst::Ror { rd, rn, rot, sf } => {
                    // Extend to 64-bit, rotate right by rot, then mask for W.
                    ldg(buf, RAX, rn as u32);
            let r = (rot & (if sf { 63u32 } else { 31u32 })) as u8;
            buf.ror_ri8(RAX, r);
            if !sf {
                buf.and_ri64(RAX, 0xffff_ffff);
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Svc { imm: _ } => {
            // Route the supervisor call to the host `guest_svc` dispatcher (this
            // is the hookpoint for real AArch64->host syscall routing).
            let addr = crate::jit::guest_svc as usize as u64;
            buf.mov_ri64(RAX, addr);
            buf.call_r64(RAX); // guest_svc(st); returns the syscall result in RAX
            stg(buf, 0, RAX); // system value -> guest x0 (AArch64 return reg)
            Ok(())
        }
        Inst::BitField { rd, rn, immr, imms, sf, arith, insert } => {
            let bits = if sf { 64u32 } else { 32u32 };
            if insert && immr <= imms {
                // BFXIL (opc=00, non-wrap): rd[imms:immr] = rn[imms:immr],
                // i.e. the field is copied in-place and Rd's other bits are kept.
                // mask = ((1<<width)-1) << immr  on a `bits`-wide integer.
                let width = (imms - immr + 1) as u32;
                let mask: u64 = (((1u64 << width) - 1) << immr) & (if sf { u64::MAX } else { 0xffff_ffff });
                ldg(buf, RAX, rn as u32); // Rn
                ldg(buf, R10, rd as u32); // old Rd
                buf.mov_ri64(RCX, mask);
                buf.and_rr64(RAX, RCX); // field of Rn
                // rd & ~mask
                buf.not_r64(RCX);
                buf.and_rr64(R10, RCX);
                buf.or_rr64(R10, RAX);
                buf.mov_rr64(RAX, R10);
            } else {
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
            } else if imms + immr + 1 == bits {
                // ROR (rotate right): the field spans the whole register, so the
                // result is a pure rotate right by `imms`. (Verified against two
                // real `ror rd, rn, #imm` words from libroblox: 0x139652d7 /
                // 0x138f51eb, both ror #20 carry rotation in imms.)
                buf.ror_ri8(RAX, (imms & (bits - 1)) as u8);
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
            } // end `else` (non-BFXIL) arm of BitField
            Ok(())
        }
        Inst::SysReg { sysreg, rt, read } => {
            // sysreg==0: tpidr_el0 (CpuState.tpidr). sysreg==1: cntfrq_el0
            // (counter tick rate in Hz) read as a fixed constant. Decode only
            // produces these two; write to cntfrq is not generated.
            if sysreg == 1 {
                // mrs xN, cntfrq_el0  ->  xN = 100_000_000 (100 MHz counter).
                // Constant Hz: the guest divides/downscales counter deltas with
                // this, so a fixed, self-consistent rate is honest for boot.
                if read && rt != 31 {
                    buf.mov_ri64(rt, 100_000_000);
                }
                return Ok(());
            }
            if sysreg == 3 {
                // mrs xN, cntvct_el0  ->  xN = CpuState.cntvct (live monotonic
                // counter, stamped by the run loop between guest blocks).
                if read && rt != 31 {
                    buf.mov_load64(rt, RBX, crate::jit::CNTVCT_OFF);
                }
                return Ok(());
            }
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
            // ops 0-3 (fmov/fabs/fneg) are double-only; single handled for 4-7 below.
            if !sz && op <= 3 {
                return Err(format!("FpScalar single-precision (sz=0) op {op} not implemented"));
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
                    if sz {
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
                    } else {
                        // single-precision (32-bit scalar FP): operands live in
                        // the low 4 bytes of each slot.
                        let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                        buf.mov_load32(RAX, RBX, f(rn));
                        buf.movd_xmm_r32(0, RAX);
                        buf.mov_load32(RAX, RBX, f(rm));
                        buf.movd_xmm_r32(1, RAX);
                        // emit scalar single (F3 0F 5x /r): SrcDst=xmm0, rm=xmm1
                        let opcode: u8 = match op {
                            4 => 0x59, // mulss
                            5 => 0x58, // addss
                            6 => 0x5c, // subss
                            7 => 0x5e, // divss
                            _ => unreachable!(),
                        };
                        buf.bytes.extend_from_slice(&[0xf3, 0x0f, opcode, 0xc1]);
                        buf.movd_r32_xmm(RAX, 0);
                        buf.mov_store32(RBX, f(rd), RAX);
                    }
                }
                _ => return Err(format!("FpScalar op {op} not implemented")),
            }
            Ok(())
        }
        Inst::FpUnary { rd, rn, op, sz } => {
            // scalar 1-source FP: fsqrt / frint{mpz}. d-reg = low 8B of CpuState.v[reg].
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if !sz {
                return Err(format!("FpUnary single-precision (sz=0) not implemented (op {op})"));
            }
            buf.movq_load(0, RBX, vslot(rn));
            match op {
                0 => buf.sqrtsd(0, 0),  // fsqrt  d{rd}, d{rn}
                1 => buf.roundsd(0, 0, 0x01), // frintm: round toward -inf (floor)
                2 => buf.roundsd(0, 0, 0x02), // frintp: round toward +inf (ceil)
                4 => buf.roundsd(0, 0, 0x03), // (frintz: toward zero) — reserved mapping
                5 => {
                    // fabs d{rd}, d{rn}: clear the sign bit on the FP bit-pattern.
                    buf.movq_r64_xmm(RAX, 0);
                    buf.mov_ri64(RCX, 0x7fff_ffff_ffff_ffff);
                    buf.and_rr64(RAX, RCX);
                    buf.movq_xmm_r64(0, RAX);
                }
                6 => {
                    // fneg d{rd}, d{rn}: flip the sign bit on the FP bit-pattern.
                    buf.movq_r64_xmm(RAX, 0);
                    buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                    buf.xor_rr64(RAX, RCX);
                    buf.movq_xmm_r64(0, RAX);
                }
                _ => return Err(format!("FpUnary op {op} not implemented")),
            }
            buf.movq_store(RBX, vslot(rd), 0);
            Ok(())
        }
        Inst::Fabd { rd, rn, rm } => {
            // fabd Dd, Dn, Dm = |dn - dm| (scalar double). Compute a-b in xmm,
            // round-trip the bit pattern to a GPR, clear the sign bit, and store.
            // Honest for finite doubles; NaN stays NaN (sign-bit clear keeps it a NaN).
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.movq_load(0, RBX, vslot(rn));
            buf.movq_load(1, RBX, vslot(rm));
            buf.subsd(0, 1); // xmm0 = rn - rm
            buf.movq_r64_xmm(RDX, 0); // RDX = bits(rn - rm)
            buf.mov_ri64(RDI, 0x7fff_ffff_ffff_ffff); // ~signbit
            buf.and_rr64(RDX, RDI); // clear bit 63 (|x|)
            buf.movq_xmm_r64(0, RDX); // back to xmm
            buf.movq_store(RBX, vslot(rd as u8), 0);
            Ok(())
        }
        Inst::FcvtToInt {
            rd,
            rn,
            mode,
            sf,
            unsigned,
            src_sng,
        } => {
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if src_sng {
                // single (S) source: load low 32 bits, promote to double in xmm0.
                buf.mov_load32(RAX, RBX, vslot(rn));
                buf.movd_xmm_r32(0, RAX);
                buf.cvtss2sd(0, 0);
            } else {
                buf.movq_load(0, RBX, vslot(rn)); // d-source (low 8B) -> xmm0
            }
            if mode == 2 {
                buf.cvtsd2si(RAX, 0); // fcvtas: round to nearest (MXCSR, default even)
            } else if mode == 3 || mode == 4 {
                // fcvtpu/ps (+inf) and fcvtmu/ms (-inf): round the double to an
                // integer-valued double first (roundsd 0x01=floor, 0x02=ceil), then
                // trunc-convert. Matches qemu ground truth (fcvtpu: 1.5->2, 0.5->1,
                // negative/NaN->0, +inf->0xFFFF... via x86 cvttsd2si saturation).
                buf.roundsd(0, 0, if mode == 3 { 0x02 } else { 0x01 });
                buf.cvttsd2si(RAX, 0);
                if unsigned {
                    buf.xor_rr64(RCX, RCX);
                    buf.test_rr64(RAX, RAX);
                    buf.cmov_rr64(0x48, RAX, RCX); // cmovs RAX, RCX (neg -> 0)
                }
            } else if unsigned {
                // fcvtzu: truncate toward zero (fcvtzs) but to an UNSIGNED value.
                // `cvttsd2si` is exact for d in [0,2^63); negatives are clamped to 0
                // below. (d >= 2^63 is architecturally out-of-range; x86 clamps —
                // an explicitly-documented limitation, NOT silent corruption.)
                buf.cvttsd2si(RAX, 0);
                // if RAX < 0 (d was negative) => result 0
                buf.xor_rr64(RCX, RCX);
                buf.test_rr64(RAX, RAX);
                buf.cmov_rr64(0x48, RAX, RCX); // cmovs RAX, RCX (RAX<0 -> 0)
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
        Inst::Scvtf {
            rd,
            rn,
            to_double,
            sf,
            unsigned,
        } => {
            // scvtf/ucvtf -> cvtsi2{sd|ss}: load the integer Rn, widen per `sf`,
            // and store the float into v{rd} (low 8B for double, low 4B for single).
            // For unsigned (ucvtf) with a 64-bit source, cvtsi2sd is exact only up
            // to 2^63-1, so we apply the standard +2^64 correction when the sign
            // bit of the 64-bit value is set (see Ucvtf2d). 32-bit-unsigned fits
            // exactly in f64 so no correction is needed there.
            let vslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            if unsigned && sf {
                // RDX = u64 source
                ldg(buf, RDX, rn as u32);
                if to_double {
                    buf.cvtsi2sd(0, true, RDX); // xmm0 = (double)(int64)u
                    buf.test_rr64(RDX, RDX);
                    let jns = buf.jcc_rel32(0x89); // JNS: skip correction if u < 2^63
                    buf.mov_ri64(RCX, 0x43f0_0000_0000_0000); // 2^64 (double bits)
                    buf.movq_xmm_r64(1, RCX);
                    buf.addsd(0, 1);
                    let end = buf.len();
                    let disp = (end as i64 - (jns as i64 + 4)) as i32;
                    buf.bytes[jns..jns + 4].copy_from_slice(&disp.to_le_bytes());
                    buf.movq_store(RBX, vslot, 0);
                } else {
                    // single result from 64-bit unsigned: (float)u = (double)u then cvtss
                    buf.cvtsi2sd(0, true, RDX);
                    buf.test_rr64(RDX, RDX);
                    let jns = buf.jcc_rel32(0x89);
                    buf.mov_ri64(RCX, 0x43f0_0000_0000_0000);
                    buf.movq_xmm_r64(1, RCX);
                    buf.addsd(0, 1);
                    let end = buf.len();
                    let disp = (end as i64 - (jns as i64 + 4)) as i32;
                    buf.bytes[jns..jns + 4].copy_from_slice(&disp.to_le_bytes());
                    buf.cvtsd2ss(0, 0); // float from the double
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, vslot, RAX);
                }
            } else {
                ldg(buf, RAX, rn as u32);
                if to_double {
                    buf.cvtsi2sd(0, sf, RAX);
                    buf.movq_store(RBX, vslot, 0);
                } else {
                    buf.cvtsi2ss(0, sf, RAX);
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, vslot, RAX);
                }
            }
            Ok(())
        }
        Inst::FmovImm { rd, f64, value_bits } => {
            // fmov Dd,#imm / fmov Sd,#imm: write the decoded IEEE-754 value into
            // the destination FP slot (low 8B for double, low 4B for single).
            let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            if f64 {
                buf.mov_ri64(RAX, value_bits);
                buf.mov_store64(RBX, slot, RAX); // low 8B of slot = double bits
            } else {
                buf.mov_ri32(RAX, value_bits as u32);
                buf.mov_store32(RBX, slot, RAX); // low 4B of slot = single bits
            }
            Ok(())
        }
        Inst::FmovFp { rd, rn, sz } => {
            // fmov Dd,Dn / fmov Sd,Sn: register-to-register FP copy (no conversion).
            let sslot = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            if sz {
                buf.mov_load64(RAX, RBX, sslot);
                buf.mov_store64(RBX, dslot, RAX); // double: copy low 8B
            } else {
                buf.mov_load32(RAX, RBX, sslot);
                buf.mov_store32(RBX, dslot, RAX); // single: copy low 4B
            }
            Ok(())
        }
        Inst::Fcmp { rn, rm, sz } => {
            // fcmp d{rn}, d{rm} / fcmp s{rn}, s{rm}: compare and set guest NZCV.
            // Use comisd/comiss (CF=1 if a<b, ZF=1 if equal/unordered, PF=1 if
            // unordered); store_nzcv_fp maps to AArch64 NZCV.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if sz {
                buf.movq_load(0, RBX, vslot(rn));
                buf.movq_load(1, RBX, vslot(rm));
            } else {
                buf.mov_load32(RAX, RBX, vslot(rn));
                buf.movd_xmm_r32(0, RAX);
                buf.mov_load32(RAX, RBX, vslot(rm));
                buf.movd_xmm_r32(1, RAX);
            }
            if sz {
                buf.comisd(0, 1);
            } else {
                buf.comiss(0, 1);
            }
            store_nzcv_fp(buf);
            Ok(())
        }
        Inst::SimdPopcnt { rd, rn } => {
            // cnt v{rd}.8b, v{rn}.8b : per-byte bit-popcount via SWAR.
            let slot = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            buf.mov_load64(RAX, RBX, slot); // x
            // x = x - ((x >> 1) & 0x5555_5555_5555_5555)
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 1);
            buf.mov_ri64(RCX, 0x5555_5555_5555_5555);
            buf.and_rr64(RDX, RCX);
            buf.sub_rr64(RAX, RDX);
            // x = (x & 0x3333...) + ((x >> 2) & 0x3333...)
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 2);
            buf.mov_ri64(RCX, 0x3333_3333_3333_3333);
            buf.and_rr64(RDX, RCX);
            buf.and_rr64(RAX, RCX);
            buf.add_rr64(RAX, RDX);
            // x = (x + (x >> 4)) & 0x0f0f_0f0f_0f0f_0f0f
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 4);
            buf.add_rr64(RAX, RDX);
            buf.mov_ri64(RCX, 0x0f0f_0f0f_0f0f_0f0f);
            buf.and_rr64(RAX, RCX);
            let dslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            buf.mov_store64(RBX, dslot, RAX);
            Ok(())
        }
        Inst::SimdSum8 { rd, rn } => {
            // uaddlv h{rd}, v{rn}.8b : sum the 8 bytes of the slot into the
            // low 16 bits (zero-extended to the d slot). Bytes in are each an
            // 8-bit popcount (<= 8), so the sum fits well within the half.
            let slot = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            buf.mov_load64(RAX, RBX, slot); // x
            buf.mov_ri64(RCX, 0x00ff_00ff_00ff_00ff);
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 8);
            buf.and_rr64(RDX, RCX);
            buf.and_rr64(RAX, RCX);
            buf.add_rr64(RAX, RDX); // s16 = per-16 sums
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 16);
            buf.mov_ri64(RCX, 0x0000_ffff_0000_ffff);
            buf.and_rr64(RDX, RCX);
            buf.and_rr64(RAX, RCX);
            buf.add_rr64(RAX, RDX); // per-32 sums
            buf.mov_rr64(RDX, RAX);
            buf.shr_ri8(RDX, 32);
            buf.mov_ri64(RCX, 0x0000_0000_ffff_ffff);
            buf.and_rr64(RDX, RCX);
            buf.and_rr64(RAX, RCX);
            buf.add_rr64(RAX, RDX); // total
            let dslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            buf.mov_store64(RBX, dslot, RAX);
            Ok(())
        }
        Inst::Fcvt { to_d, rd, rn } => {
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if to_d {
                // fcvt d{rd}, s{rn}: single -> double (widen).
                buf.mov_load32(RAX, RBX, vslot(rn)); // single in low 32 of slot
                buf.movd_xmm_r32(0, RAX);            // xmm0 = s{rn}
                buf.cvtss2sd(0, 0);                   // xmm0 = (double) xmm0
                let dslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                buf.movq_store(RBX, dslot, 0);       // low 64 = double result
            } else {
                // fcvt s{rd}, d{rn} : double -> single (narrow).
                buf.movq_load(0, RBX, vslot(rn));     // xmm0 = double d{rn}
                buf.cvtsd2ss(0, 0);                   // xmm0 = floating single
                buf.movd_r32_xmm(RAX, 0);            // RAX = low 32 (single bits)
                let dslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                buf.mov_store32(RBX, dslot, RAX);
            }
            Ok(())
        }
        Inst::InsD1D0 { rd, rn } => {
            // mov v{rd}.d[1], v{rn}.d[0] : copy the low 64 (D[0]) of Rn into
            // the high 64 (D[1]) of Rd.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.mov_load64(RAX, RBX, vslot(rn)); // low 64 of Vn
            buf.mov_store64(RBX, vslot(rd) + 8, RAX); // high 64 of Vd
            Ok(())
        }
        Inst::Simd4s { rd, rn, rm, op: 0 } => {
            // add Vd.4s, Vn.4s, Vm.4s : 4x32-bit lane add via paddd.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.movdqu_load(0, RBX, slot(rn)); // xmm0 = Vn (128-bit)
            buf.movdqu_load(1, RBX, slot(rm)); // xmm1 = Vm
            buf.paddd(0, 1); // xmm0 = Vn + Vm (4x32)
            buf.movdqu_store(RBX, slot(rd), 0); // Vd = result
            Ok(())
        }
        Inst::Simd4s { .. } => Err("Simd4s op not implemented".to_string()),
        Inst::SimdDupD { rd, rn, index } => {
            // dup Vd.2D, Vn.D[index]: broadcast the selected 64-bit lane of Vn
            // into both 64-bit lanes of Vd. index 0 => low 64, 1 => high 64.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let src_off = slot(rn) + (index as i32) * 8;
            buf.mov_load64(RAX, RBX, src_off); // RAX = selected lane
            let dst_slot = slot(rd);
            buf.mov_store64(RBX, dst_slot, RAX); // lane 0
            buf.mov_store64(RBX, dst_slot + 8, RAX); // lane 1
            Ok(())
        }
        Inst::Ucvtf2d { rd, rn } => {
            // ucvtf Vd.2D, Vn.2D : for each of the two 64-bit lanes of Vn (viewed as
            // unsigned integers), convert to double and store into the matching lane
            // of Vd. Honest u64->f64 with a sign-corrected `cvtsi2sd`:
            //   xmm0 = (double)(int64)u          (exact & correct when u < 2^63)
            //   if u >= 2^63: xmm0 += 2^64        (reconstructs (double)u; 2^64 exact)
            // The JNS branch skips the add for non-negative u. This is the standard
            // exact u64->double conversion (no range silently mishandled).
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            for lane in 0..2 {
                let off = lane * 8;
                // RDX = u64 lane value (unsigned).
                buf.mov_load64(RDX, RBX, slot(rn) + off);
                // xmm0 = (double)(int64)RDX
                buf.cvtsi2sd(0, true, RDX);
                // SF = sign(RDX); jump over the correction when u isn't >= 2^63.
                buf.test_rr64(RDX, RDX);
                let jns = buf.jcc_rel32(0x89); // 0F 89 = JNS rel32
                // Correction only when the sign bit was set (u >= 2^63):
                //   mov rcx, [2^64 as double]; movq xmm1, rcx; addsd xmm0, xmm1
                buf.mov_ri64(RCX, 0x43f0_0000_0000_0000); // 2^64 (double bits)
                buf.movq_xmm_r64(1, RCX);
                buf.addsd(0, 1);
                let end = buf.len();
                // Patch the JNS displacement to skip the 3-insn correction block.
                let disp = (end as i64 - (jns as i64 + 4)) as i32;
                buf.bytes[jns..jns + 4].copy_from_slice(&disp.to_le_bytes());
                // store low 64 of xmm0 -> Vd lane.
                buf.movq_store(RBX, slot(rd) + off, 0);
                            }
                            Ok(())
                        }
                        Inst::ScalarUcvtf { rd, rn } => {
                            // ucvtf Dd, Dn : read Dn's low 64 bits as an unsigned integer
                            // and write the double to Dd. Honest u64->f64 (Ucvtf2d lane).
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            buf.mov_load64(RDX, RBX, slot(rn));
                            buf.cvtsi2sd(0, true, RDX);
                            buf.test_rr64(RDX, RDX);
                            let jns = buf.jcc_rel32(0x89); // JNS (sign clear -> skip correction)
                            buf.mov_ri64(RCX, 0x43f0_0000_0000_0000); // 2^64 as double
                            buf.movq_xmm_r64(1, RCX);
                            buf.addsd(0, 1);
                            let end = buf.len();
                            let disp = (end as i64 - (jns as i64 + 4)) as i32;
                            buf.bytes[jns..jns + 4].copy_from_slice(&disp.to_le_bytes());
                            buf.movq_store(RBX, slot(rd), 0);
                            Ok(())
                        }
                        Inst::ScalarScvtf { rd, rn } => {
                            // scvtf Dd, Dn : read Dn's low 64 bits as a SIGNED integer
                            // and write the double to Dd (two's-complement -> f64, signed).
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            buf.mov_load64(RDX, RBX, slot(rn));
                            buf.cvtsi2sd(0, true, RDX); // signed i64 -> f64
                            buf.movq_store(RBX, slot(rd), 0);
                            Ok(())
                        }
                        Inst::Simd2dFp { rd, rn, rm, op } => {
                            // 2xdouble lanewise FP: op Vd.2D, Vn.2D, Vm.2D. For each 64-bit lane:
                            //   xmm0 = Vn lane; xmm1 = Vm lane; xmm0 op xmm1; store to Vd lane.
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            for lane in 0..2 {
                                let off = lane * 8;
                                buf.mov_load64(RDX, RBX, slot(rn) + off); // RDX = Vn.lane
                                buf.movq_xmm_r64(0, RDX);
                                buf.mov_load64(RDX, RBX, slot(rm) + off); // RDX = Vm.lane
                                buf.movq_xmm_r64(1, RDX);
                                match op {
                                    0 => buf.divsd(0, 1), // fdiv
                                    1 => buf.mulsd(0, 1), // fmul
                                    2 => buf.addsd(0, 1), // fadd
                                    3 => buf.subsd(0, 1), // fsub
                                    _ => return Err(format!("Simd2dFp op {op} not implemented")),
                                }
                                buf.movq_store(RBX, slot(rd) + off, 0);
                            }
                            Ok(())
                        }
                        Inst::SimdDupSReg { rd, rn } => {
                            // dup Vd.4S, Wn: broadcast Wn (32-bit) into all 4 S-lanes of Vd.
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            ldg(buf, RAX, rn as u32); // Wn read (zero-extended into RAX's low 32)
                            buf.and_ri64(RAX, 0xffff_ffff);
                            for i in 0..4u32 {
                                                            buf.mov_store32(RBX, slot(rd) + (i * 4) as i32, RAX);
                                                        }
                                                        Ok(())
                                                    }
                                                    Inst::SimdOrr16 { rd, rn, rm } => {
                                                        // orr Vd.16B, Vn.16B, Vm.16B (also `mov Vd.16B,Vn.16B`
                                                        // copy when rm==rn): OR across all 16 bytes, 8 at a time.
                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                        for off in [0i32, 8i32] {
                                                            buf.mov_load64(RAX, RBX, slot(rn) + off);
                                                            buf.mov_load64(RCX, RBX, slot(rm) + off);
                                                            buf.or_rr64(RAX, RCX);
                                                            buf.mov_store64(RBX, slot(rd) + off, RAX);
                                                                                        }
                                                                                        Ok(())
                                                                                    }
                                                                                    Inst::SimdMul { rd, rn, rm, lanes } => {
                                                                                        // mul Vd.4S/Vd.2S, Vn., Vm.: per 32-bit lane, low-32 product
                                                                                        // (mod-2^32 wrap). 64-bit imul of zero-extended 32-bit operands
                                                                                        // yields the low-32 product correctly for both signed words.
                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                        for i in 0..lanes {
                                                                                            let off = (i as i32) * 4;
                                                                                            buf.mov_load32(RAX, RBX, slot(rn) + off);
                                                                                            buf.mov_load32(RCX, RBX, slot(rm) + off);
                                                                                            buf.imul_rr64(RAX, RCX); // low 32 = (a*b) mod 2^32
                                                                                            buf.mov_store32(RBX, slot(rd) + off, RAX);
                                                                                                                        }
                                                                                                                        Ok(())
                                                                                                                    }
                                                                                                                    Inst::SimdCmhi { rd, rn, rm, lanes } => {
                                                                                                                        // cmhi Vd.4S/Vd.2S, Vn., Vm.: per 32-bit lane, all-ones
                                                                                                                        // if Vn[i] > Vm[i] (unsigned), else 0. Compare unsigned
                                                                                                                        // then cmov (cmova) an all-ones mask vs 0.
                                                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                                                for i in 0..lanes {
                                                                                                                                                                                                                                                    let off = (i as i32) * 4;
                                                                                                                                                                                                                                                    buf.mov_load32(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                                                                    buf.mov_load32(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX); // unsigned: CF=1 if Vn<Vm
                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0xffff_ffff_ffff_ffff);
                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0);
                                                                                                                                                                                                                                                    buf.cmov_rr64(0x47, RDI, RDX); // cmova: RDI=ones if Vn>Vm else 0
                                                                                                                                                                                                                                                    buf.mov_store32(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                Ok(())
                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                            Inst::SimdCmhiD { rd, rn, rm } => {
                                                                                                                                                                                                                                                // cmhi Vd.2D, Vn.2D, Vm.2D (Q=1): per 64-bit lane, all-ones
                                                                                                                                                                                                                                                // if Vn[i] > Vm[i] (unsigned) else 0.
                                                                                                                                                                                                                                                let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                                                for i in 0..2u32 {
                                                                                                                                                                                                                                                    let off = (i as i32) * 8;
                                                                                                                                                                                                                                                    buf.mov_load64(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                                                                    buf.mov_load64(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX); // unsigned: CF=0 if Vn>=Vm, CF=1 if Vn<Vm
                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0xffff_ffff_ffff_ffff);
                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0);
                                                                                                                                                                                                                                                    buf.cmov_rr64(0x47, RDI, RDX); // cmova: RDI=ones if Vn>Vm else 0
                                                                                                                                                                                                                                                    buf.mov_store64(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                Ok(())
                                                                                                                                                                                                                                            }
                                                                                                                                                                                Inst::SimdCmEq { rd, rn, rm, lanes, esize } => {
            // cmeq Vd.T, Vn.T, Vm.T: each element is all-ones if Vn[i]==Vm[i]
            // else 0. Compare the esize-byte element (zero-extended via the
            // widest load that fits), then cmov all-ones vs 0, store esize bytes.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            for i in 0..lanes {
                let off = (i as i32) * (esize as i32);
                if esize == 8 {
                    buf.mov_load64(RAX, RBX, slot(rn) + off);
                    buf.mov_load64(RCX, RBX, slot(rm) + off);
                } else {
                    buf.mov_load32(RAX, RBX, slot(rn) + off);
                    buf.mov_load32(RCX, RBX, slot(rm) + off);
                    let imm = match esize {
                        4 => 0xffff_ffffu32,
                        2 => 0xffffu32,
                        _ => 0xffu32,
                    };
                    buf.and_ri64(RAX, imm);
                    buf.and_ri64(RCX, imm);
                }
                buf.cmp_rr64(RAX, RCX); // ZF=1 if equal
                buf.mov_ri64(RDI, 0);
                buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff);
                buf.cmov_rr64(0x44, RDI, RDX); // 0x44=cmove: RDI=ones if equal
                match esize {
                    8 => buf.mov_store64(RBX, slot(rd) + off, RDI),
                    4 => buf.mov_store32(RBX, slot(rd) + off, RDI),
                    2 => buf.mov_store16(RBX, slot(rd) + off, RDI),
                    _ => buf.mov_store8(RBX, slot(rd) + off, RDI),
                }
            }
            Ok(())
        }
                Inst::SimdXtn { rd, rn, dst_esize } => {
                    // xtn Vd.8b/4h/2s, Vn.<wider>: take the LOW `dst_esize` bytes of each
                    // source element (source element esize = 2*dst_esize) and pack them
                    // into dest lanes. Q=0 => 64-bit dest result (high lane of Vd zeroed).
                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let src_esize = 2 * (dst_esize as i32);
                    let lanes = 8 / (dst_esize as i32);
                    for i in 0..lanes {
                        let src_off = (i as i32) * src_esize;
                        let dst_off = (i as i32) * (dst_esize as i32);
                        // load the low `dst_esize` bytes of source element into RAX
                                        // (mov_load32 over-reads past the element for 1/2-byte lanes, but
                                        //  the extra bytes are masked out before store)
                                        buf.mov_load32(RAX, RBX, slot(rn) + src_off);
                                        match dst_esize {
                                            2 => buf.and_ri64(RAX, 0xffff),
                                            _ => buf.and_ri64(RAX, 0xff),
                                        }
                                        // store into dest lane
                                        match dst_esize {
                                            4 => buf.mov_store32(RBX, slot(rd) + dst_off, RAX),
                                            2 => buf.mov_store16(RBX, slot(rd) + dst_off, RAX),
                                            _ => buf.mov_store8(RBX, slot(rd) + dst_off, RAX),
                                        }
                    }
                    // zero the high 64 bits of Vd
                    buf.mov_ri64(RAX, 0);
                    buf.mov_store64(RBX, slot(rd) + 8, RAX);
                    Ok(())
                }
                Inst::Ld1 { rd, rn, esize, q } => {
                    // ld1r {Vt.T}, [Xn]: load `esize` bytes from [x[rn]] and
                    // replicate across (q?16:8)/esize lanes of Vd. Guest memory is
                    // host-addressable in this in-process JIT, so [x[rn]] is a
                    // direct dereference.
                    let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                    ldg(buf, RDX, rn as u32); // RDX = base address (host ptr)
                    match esize {
                        8 => buf.mov_load64(RAX, RDX, 0),
                        4 => buf.mov_load32(RAX, RDX, 0),
                        2 => buf.movzx_word_mem(RAX, RDX, 0),
                        _ => buf.movzx_byte_mem(RAX, RDX, 0),
                    }
                    let total = if q { 16i32 } else { 8i32 };
                    let mut off = 0i32;
                    while off < total {
                        match esize {
                            8 => buf.mov_store64(RBX, slot + off, RAX),
                            4 => buf.mov_store32(RBX, slot + off, RAX),
                            2 => buf.mov_store16(RBX, slot + off, RAX),
                            _ => buf.mov_store8(RBX, slot + off, RAX),
                        }
                        off += esize as i32;
                    }
                    // for q=0, zero the high 64 bits
                    if !q {
                        buf.mov_ri64(RAX, 0);
                        buf.mov_store64(RBX, slot + 8, RAX);
                    }
                    Ok(())
                }
                Inst::SimdInsD { rd, rn, dst_idx, src_idx } => {
                    // mov Vd.d[dst], Vn.d[src]: copy one 64-bit lane between vectors.
                    let src = crate::jit::VECTOR_BASE + (rn as i32)*16 + (src_idx as i32)*8;
                    let dst = crate::jit::VECTOR_BASE + (rd as i32)*16 + (dst_idx as i32)*8;
                    buf.mov_load64(RAX, RBX, src);
                    buf.mov_store64(RBX, dst, RAX);
                    Ok(())
                }
                // Vd = (Vn & Vm) | (Vd & ~Vm), over the full 16 bytes
        Inst::SimdFmovImm { rd, esize, value_bits, q } => {
            // fmov Vd.T, #imm: broadcast the immediate FP float (esize bytes,
            // 64-bit double or 32-bit single bits) into every lane of Vd.
            let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            if esize == 8 {
                buf.mov_ri64(RAX, value_bits);
            } else {
                buf.mov_ri64(RAX, value_bits & 0xffff_ffff);
            }
            let total = if q { 16i32 } else { 8i32 };
            let mut off = 0i32;
            while off < total {
                if esize == 8 {
                    buf.mov_store64(RBX, slot + off, RAX);
                } else {
                    buf.mov_store32(RBX, slot + off, RAX);
                }
                off += esize as i32;
            }
            Ok(())
        }
        Inst::VarShiftVar { rd, rn, rm, op, sf } => {
            // lslv/lsrv/asrv/rorv Rd, Rn, Rm: variable shift by register. Rn -> RAX,
            // Rm count -> RCX (low byte CL); shift via the _cl64 helpers. W masks the
            // count to 0x1f and ASR sign-extends the low 32 before arithmetic shift.
            ldg(buf, RAX, rn as u32); // value
            ldg(buf, RCX, rm as u32); // shift count
            if sf {
                buf.and_ri64(RCX, 0x3f);
            } else {
                buf.and_ri64(RCX, 0x1f);
                if op == 2 {
                    // asr (W): sign-extend low 32 before arithmetic shift
                    buf.movsxd_r64_r32(RAX, RAX);
                }
            }
            match op {
                0 => buf.shl_cl64(RAX), // lslv
                1 => buf.shr_cl64(RAX), // lsrv
                2 => buf.sar_cl64(RAX), // asrv
                _ => buf.ror_cl64(RAX), // rorv
            }
            if !sf {
                buf.and_ri64(RAX, 0xffff_ffff);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::SimdFpUnary { rd, rn, op, esize, q } => {
            // fneg/fabs/fsqrt Vd.T, Vn.T: per-lane unary FP on the vector slot.
            // fneg/fabs flip/clear the sign bit on the FP bit-pattern via GPRs;
            // fsqrt uses x86 sqrtsd. esize 8 lanes are full doubles, esize 4
            // lanes handled the same (sign-bit at bit 31; store low 32 back).
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            let sign64: u64 = if esize == 8 { 0x8000_0000_0000_0000 } else { 0x8000_0000 };
            for l in 0..lanes {
                let off = l * m;
                buf.movq_load(0, RBX, src + off); // xmm0 <- lane bits
                if op == 2 {
                    buf.sqrtsd(0, 0); // fsqrt
                } else {
                    // fneg (op 0): xor sign; fabs (op 1): and with ~sign
                    buf.movq_r64_xmm(RAX, 0);
                    buf.mov_ri64(RCX, sign64);
                    if op == 0 {
                        buf.xor_rr64(RAX, RCX);
                    } else {
                        buf.mov_ri64(RDX, sign64);
                        buf.not_r64(RDX);
                        buf.and_rr64(RAX, RDX);
                    }
                    buf.movq_xmm_r64(0, RAX);
                }
                if esize == 8 {
                    buf.movq_store(RBX, dst + off, 0);
                } else {
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, dst + off, RAX);
                }
            }
            Ok(())
        }
        Inst::FcvVec { rd, rn, signed, esize, q } => {
            // fcvtzu/fcvtzs Vd.T, Vn.T: convert each FP lan e (esize bytes) to an
            // int, truncating toward zero. Per-lane movq->cvttsd2si (signed), then
            // clamp negatives to 0 for the unsigned fcvtzu (mirrors scalar FcvtToInt).
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            for l in 0..lanes {
                buf.movq_load(0, RBX, src + l * m);
                buf.cvttsd2si(RAX, 0);
                if !signed {
                    buf.xor_rr64(RCX, RCX);
                    buf.test_rr64(RAX, RAX);
                    buf.cmov_rr64(0x48, RAX, RCX);
                }
                buf.movq_store(RBX, dst + l * m, 0);
            }
            Ok(())
        }
                Inst::SimDup { rd, rn, esize, src_idx, q } => {
                    // dup Vd.T, Vn.T[src]: broadcast element at Vn[src_idx*esize] across
                    // all q?16:8 bytes of Vd (all lanes identical).
                    let src = crate::jit::VECTOR_BASE + (rn as i32)*16 + (src_idx as i32)*(esize as i32);
                    match esize {
                        8 => buf.mov_load64(RAX, RBX, src),
                        4 => buf.mov_load32(RAX, RBX, src),
                        2 => buf.movzx_word_mem(RAX, RBX, src),
                        _ => buf.movzx_byte_mem(RAX, RBX, src),
                    }
                    let slot = crate::jit::VECTOR_BASE + (rd as i32)*16;
                    let total = if q { 16i32 } else { 8i32 };
                    let mut off = 0i32;
                    while off < total {
                        match esize {
                            8 => buf.mov_store64(RBX, slot + off, RAX),
                            4 => buf.mov_store32(RBX, slot + off, RAX),
                            2 => buf.mov_store16(RBX, slot + off, RAX),
                            _ => buf.mov_store8(RBX, slot + off, RAX),
                        }
                        off += esize as i32;
                    }
                    if !q {
                        buf.mov_ri64(RAX, 0);
                        buf.mov_store64(RBX, slot + 8, RAX);
                    }
                    Ok(())
                }
                // (2 x 64-bit halves). RAX/RCX/RDX/RDI scratch.
                                                                                                                                                                                Inst::SimdBit { rd, rn, rm } => {
                                                                                                                                                                                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                    for off in [0i32, 8] {
                                                                                                                                                                                        buf.mov_load64(RAX, RBX, slot(rn) + off); // RAX = Vn
                                                                                                                                                                                        buf.mov_load64(RCX, RBX, slot(rm) + off); // RCX = Vm
                                                                                                                                                                                        buf.and_rr64(RAX, RCX); // RAX = Vn & Vm
                                                                                                                                                                                        buf.mov_load64(RDX, RBX, slot(rd) + off); // RDX = old Vd
                                                                                                                                                                                        buf.mov_ri64(RDI, 0xffff_ffff_ffff_ffff);
                                                                                                                                                                                        buf.xor_rr64(RCX, RDI); // RCX = ~Vm
                                                                                                                                                                                        buf.and_rr64(RDX, RCX); // RDX = Vd & ~Vm
                                                                                                                                                                                        buf.or_rr64(RAX, RDX); // (Vn&Vm)|(Vd&~Vm)
                                                                                                                                                                                                                                                                                                                                                                                buf.mov_store64(RBX, slot(rd) + off, RAX);
                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                    Ok(())
                                                                                                                                                                                }
                                                                                                                                                                                Inst::SimdExt { rd, rn, rm, imm, q } => {
                                                                                                                                                                                    // ext Vd, Vn, Vm, #imm: Vd = the 128(64)-bit window of the
                                                                                                                                                                                    // concatenation {Vn(high), Vm(low)} starting at byte `imm`.
                                                                                                                                                                                    // concat words W[0..3] = Vm.lo, Vm.hi, Vn.lo, Vn.hi (byte
                                                                                                                                                                                    // addresses 0..31). result.lo = bytes imm..imm+7 of concat,
                                                                                                                                                                                    // result.hi = bytes imm+8..imm+15 (16B form). For 8B (Q=0)
                                                                                                                                                                                    // only the low 64 bits are produced and the high lane is 0.
                                                                                                                                                                                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                    // Emit 64-bit field of concat starting at byte `start` into `dst`.
                                                                                                                                                                                    let emit_bytes64 = |buf: &mut crate::x86::CodeBuf, dst: u8, start: usize| {
                                                                                                                                                                                        let wi = start / 8;
                                                                                                                                                                                        let sh = (start % 8) as u8; // bytes -> bits
                                                                                                                                                                                        if sh == 0 {
                                                                                                                                                                                            // aligned: 8 bytes directly from one concat word
                                                                                                                                                                                            match wi {
                                                                                                                                                                                                0 => buf.mov_load64(dst, RBX, slot(rm)),
                                                                                                                                                                                                1 => buf.mov_load64(dst, RBX, slot(rm) + 8),
                                                                                                                                                                                                2 => buf.mov_load64(dst, RBX, slot(rn)),
                                                                                                                                                                                                _ => buf.mov_load64(dst, RBX, slot(rn) + 8),
                                                                                                                                                                                            }
                                                                                                                                                                                        } else {
                                                                                                                                                                                            // unaligned: (W[wi] >> sh) | (W[wi+1] << (64-sh))
                                                                                                                                                                                            let (ra0, a_off, rb0, b_off) = match wi {
                                                                                                                                                                                                0 => (rm, 0, rm, 8),
                                                                                                                                                                                                1 => (rm, 8, rn, 0),
                                                                                                                                                                                                _ => (rn, 0, rn, 8),
                                                                                                                                                                                            };
                                                                                                                                                                                            buf.mov_load64(RDX, RBX, slot(ra0) + a_off);
                                                                                                                                                                                            buf.shr_ri8(RDX, sh);
                                                                                                                                                                                            buf.mov_load64(RDI, RBX, slot(rb0) + b_off);
                                                                                                                                                                                            buf.shl_ri8(RDI, 64 - sh);
                                                                                                                                                                                            buf.or_rr64(RDX, RDI);
                                                                                                                                                                                            buf.mov_rr64(dst, RDX);
                                                                                                                                                                                        }
                                                                                                                                                                                    };
                                                                                                                                                                                    if !q {
                                                                                                                                                                                        // 8B: result.lo = bytes imm..imm+7 of concat; hi lane zeroed.
                                                                                                                                                                                        emit_bytes64(buf, RAX, imm as usize);
                                                                                                                                                                                        buf.mov_store64(RBX, slot(rd), RAX);
                                                                                                                                                                                        buf.mov_ri64(RDI, 0);
                                                                                                                                                                                        buf.mov_store64(RBX, slot(rd) + 8, RDI);
                                                                                                                                                                                    } else {
                                                                                                                                                                                        emit_bytes64(buf, RAX, imm as usize);
                                                                                                                                                                                        emit_bytes64(buf, RCX, (imm as usize) + 8);
                                                                                                                                                                                        buf.mov_store64(RBX, slot(rd), RAX);
                                                                                                                                                                                        buf.mov_store64(RBX, slot(rd) + 8, RCX);
                                                                                                                                                                                    }
                                                                                                                                                                                    Ok(())
                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                    Inst::SimdLaneGp { rd, rn, esize, index, sign, wide } => {
                                                                                                                                                                                                                                                                                                        // mov/umov/smov Wd|Xd, Vn.T[index]: copy one element
                                                                                                                                                                                                                                                                                                        // (esize bytes) from the 16-byte vector slot of Vn at byte
                                                                                                                                                                                                                                                                                                        // offset index*esize into GPR rd, (sign|zero) extended.
                                                                                                                                                                                                                                                                                                        // Slot layout: [D0@+0..+7][D1@+8..+15]; lane i lives at
                                                                                                                                                                                                                                                                                                        // index*esize (e.g. s[1] = +4, d[0]=+0, d[1]=+8).
                                                                                                                                                                                                                                                                                                        let off = (index as i32) * (esize as i32);
                                                                                                                                                                                                                                                                                                        let vbase = crate::jit::VECTOR_BASE + (rn as i32) * 16 + off;
                                                                                                                                                                                                                                                                                                        if esize == 8 {
                                                                                                                                                                                                                                                                                                            buf.mov_load64(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                            stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                        } else {
                                                                                                                                                                                                                                                                                                                        // esize == 4 (.s). umov/smov: load 32-bit at byte offset
                                                                                                                                                                                                                                                                                                                        // index*esize (== vbase); zero- or sign-extend to the 64-bit
                                                                                                                                                                                                                                                                                                                        // guest slot (mov_load32 zero-extends; movsxd sign-extends).
                                                                                                                                                                                                                                                                                                                        buf.mov_load32(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                        if sign {
                                                                                                                                                                                                                                                                                                                            buf.movsxd_r64_r32(RAX, RAX);
                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                        stg(buf, rd as u32, RAX);
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
        Inst::WaitBarrier => {
            // dmb/dsb/isb — memory/cache ordering barrier; the JIT is
            // single-threaded so ordering and cache flush are irrelevant.
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
