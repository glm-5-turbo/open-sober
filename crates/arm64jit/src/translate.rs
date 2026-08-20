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
        ShiftKind::Ror => panic!("ror not implemented"),
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
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
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
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
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
        Inst::Cbz { rt, imm, nonzero, .. } => {
            let target = pc.wrapping_add(imm as u64);
            ldg(buf, RAX, rt as u32); // test rt
            buf.test_rr64(RAX, RAX);
            let cc = if nonzero { 0x85 } else { 0x84 }; // jnz / jz
            let disp = buf.jcc_rel32(cc);
            fixups.push(Fixup { target_pc: target, disp_off: disp, cc });
            Ok(())
        }
        Inst::BCond { cond, imm } => {
            let target = pc.wrapping_add(imm as u64);
            match cond {
                0xE => {
                    // AL: unconditional branch via jmp
                    let disp = buf.jmp_rel32();
                    fixups.push(Fixup { target_pc: target, disp_off: disp, cc: 0xff });
                }
                0xF => {
                    // NV: never executed -> nothing to emit
                }
                c => {
                    let cc = x86_cc_for_cond(c)
                        .ok_or_else(|| format!("B.cond unsupported cond {:x}", c))?;
                    let disp = buf.jcc_rel32(cc);
                    fixups.push(Fixup { target_pc: target, disp_off: disp, cc });
                }
            }
            Ok(())
        }
        _ => Err(format!("translate: unhandled {:?}", inst)),
    }
}