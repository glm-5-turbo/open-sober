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
            ..
        } => {
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
        Inst::AddSubReg {
            rd,
            rn,
            rm,
            sub,
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
            let _ = s; // ANDS/ORRS/EORS also clear flags; reused via x86 flags if needed
            let _ = sf;
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
                _ => return Err(format!("LogicReg op {} not implemented", op)),
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
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
        Inst::LdStPair {
            rt,
            rt2,
            rn,
            imm,
            ld,
            writeback,
            preidx,
            size_64,
        } => {
            let esize = if size_64 { 8i32 } else { 4i32 };
            let imm32 = imm as i32;
            // eff base: pre-index adjusts the address by imm before the access;
            // post/offset use rn (post then adds imm for writeback).
            ldg(buf, RDX, rn as u32); // RDX = rn
            let (access_off, wb_off) = if preidx {
                (imm32, imm32) // access at rn+imm, then rn=rn+imm
            } else {
                (0i32, if writeback { imm32 } else { 0 }) // access at rn, wb adds imm
            };
            if ld {
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
        _ => Err(format!("translate: unhandled {:?}", inst)),
    }
}
