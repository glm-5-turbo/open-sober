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
use crate::x86::{CodeBuf, RAX, RBX, RCX, RDX};

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
        ShiftKind::Ror => panic!("ror not implemented"),
    }
}

/// Translate a single instruction (writes to `buf`, no control flow yet).
pub fn translate(buf: &mut CodeBuf, _pc: u64, inst: Inst) -> Result<(), String> {
    match inst {
        Inst::MoveWide { rd, imm16, hw, opc, .. } => {
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
        Inst::AddSubImm { rd, rn, imm12, shift12, sub, .. } => {
            let imm: u64 = (imm12 as u64) << if shift12 { 12 } else { 0 };
            ldg(buf, RAX, rn as u32);
            if sub {
                buf.sub_ri64(RAX, imm as u32);
            } else {
                buf.add_ri64(RAX, imm as u32);
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::AddSubReg { rd, rn, rm, sub, shift, sh_amt, .. } => {
            ldg(buf, RAX, rn as u32);
            ldg(buf, RCX, rm as u32);
            apply_shift_const(buf, RCX, shift, sh_amt);
            if sub {
                buf.sub_rr64(RAX, RCX);
            } else {
                buf.add_rr64(RAX, RCX);
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::LdStrImm { rt, rn, imm, size, ld } => {
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
                (4, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    stg(buf, rt as u32, RAX);
                }
                (8, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (4, false) => {
                    ldg(buf, RAX, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("LdStrImm size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::Ret => {
            // return x0 in RAX, then ret (matches the JIT fn convention that the
            // epilogue also uses). $[x0] at RBX+0.
            buf.mov_load64(RAX, RBX, 0);
            buf.ret();
            Ok(())
        }
        _ => Err(format!("translate: unhandled {:?}", inst)),
    }
}