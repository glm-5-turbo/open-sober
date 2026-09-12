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
use crate::x86::{CodeBuf, RSI, RAX, RBX, RCX, RDX, RDI, R10};

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

/// Load guest register `g` into RAX as a *store source value*. In AArch64 the
/// source register field of a STORE (`str x0,[..]`) reads x31 as XZR (zero),
/// NOT SP — a store of `xzr` (extremely common: compilers zero-init stack slots
/// and objects with `str xzr,[..]`) must store 0, never the stack pointer.
/// Distinguish this from an *addressing base* (`rn`, where x31 = SP), which
/// `ldg` continues to handle directly.
#[inline]
fn ldg_src(buf: &mut CodeBuf, g: u32) {
    if g == 31 {
        buf.mov_ri64(RAX, 0); // XZR reads as zero
    } else {
        buf.mov_load64(RAX, RBX, slot(g));
    }
}

/// Store RAX into guest register `g` after a LOAD, but only if `g` is a real
/// register. When the load destination field is x31 (`ldr xzr,[..]`) ARM treats
/// it as a no-op that must NOT write the SP slot (which x31 also aliases).
#[inline]
fn stg_if_writable(buf: &mut CodeBuf, g: u32) {
    if g != 31 {
        buf.mov_store64(RBX, slot(g), RAX);
    }
}

/// Transfer `size` bytes (B=1/H=2/S=4/D=8) between the low bytes of the guest
/// vector slot `vslot` (into CpuState via RBX) and the memory address held in
/// host register `addr`. `ld=true` loads [addr]->slot; `ld=false` stores
/// slot->[addr]. Only the low `size` bytes of the 16-byte slot are touched —
/// upper lanes stay preserved (ARM scalar `ldr d0` keeps the high 64 bits).
/// Shared by the scalar FP/SIMD immediate forms (FpLdStImm, FpLdStImmUnscaled,
/// FpLdStImmWb).
#[inline]
fn fp_scalar_xfer(
    buf: &mut CodeBuf,
    addr: u8,
    vslot: i32,
    size: u8,
    ld: bool,
) -> Result<(), String> {
    match (size, ld) {
        (8, true) => {
            buf.mov_load64(RAX, addr, 0);
            buf.mov_store64(RBX, vslot, RAX);
        }
        (8, false) => {
            buf.mov_load64(RAX, RBX, vslot);
            buf.mov_store64(addr, 0, RAX);
        }
        (4, true) => {
            buf.mov_load32(RAX, addr, 0);
            buf.mov_store32(RBX, vslot, RAX);
        }
        (4, false) => {
            buf.mov_load32(RAX, RBX, vslot);
            buf.mov_store32(addr, 0, RAX);
        }
        (2, true) => {
            buf.movzx_word_mem(RAX, addr, 0);
            buf.mov_store16(RBX, vslot, RAX);
        }
        (2, false) => {
            buf.movzx_word_mem(RAX, RBX, vslot);
            buf.mov_store16(addr, 0, RAX);
        }
        (1, true) => {
            buf.movzx_byte_mem(RAX, addr, 0);
            buf.mov_store8(RBX, vslot, RAX);
        }
        (1, false) => {
            buf.movzx_byte_mem(RAX, RBX, vslot);
            buf.mov_store8(addr, 0, RAX);
        }
        (s, _) => return Err(format!("fp_scalar_xfer size {s} not implemented")),
    }
    Ok(())
}

/// Zero-extend the low 32 bits of x86 reg `r` into its upper half. AArch64
/// writes to a W (32-bit) register always zero the upper 32 bits of the
/// corresponding X register; x86 64-bit ops leave them stale, so a 32-bit data
/// value must be cleaned before it propagates (e.g. `mov w0,w1` copying a
/// negative two's-complement w1 would otherwise carry `0xffffffffffffffff`).
#[inline]
fn zext_w(buf: &mut CodeBuf, r: u8) {
    buf.shl_ri8(r, 32);
    buf.shr_ri8(r, 32);
}

/// Byte offset of `CpuState.nzcv` (after pc@256: nzcv u32 at 264).
const NZCV_OFF: i32 = 8 * 32 + 8; // 264

/// Snapshot a source vector register to the permscratch area when it aliases
/// `rd`, so a SIMD permute does not clobber a source it is still reading.
/// `slotA` (0x800) and `slotB` (0x810) are the two 16-byte scratch slots.
/// Returns the byte offset to READ the (possibly snapshotted) source from.
/// When the source does not alias rd it is read in place (no copy needed).
fn permute_source(
    buf: &mut CodeBuf,
    rd: u8,
    src: u8,
    scratch_hi: bool,
) -> i32 {
    let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
    let orig = slot(src as i32);
    if src == rd {
        // rd aliases the source. Copy the full 16B to scratch, then read from
        // there. Note rd==31 is a legit VECTOR dest (v31), NOT XZR — only an
        // actual register-alias (src == rd) needs the snapshot.
        let scratch = crate::jit::PERMSCRATCH_OFF + if scratch_hi { 16 } else { 0 };
        // copy both u64 halves: [orig..orig+8) -> [scratch..scratch+8)
        buf.mov_load64(RAX, RBX, orig);
        buf.mov_store64(RBX, scratch, RAX);
        buf.mov_load64(RAX, RBX, orig + 8);
        buf.mov_store64(RBX, scratch + 8, RAX);
        scratch
    } else {
        orig
    }
}
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
        // C = CF(bit0) && !PF(bit2) -> bit29. This is the BORROW convention that
        // x86_cc_for_cond expects (HS=JAE=!CF, LS=JBE=CF||ZF, HI=JA, LO=JB),
        // NOT the true ARM-FP carry value. ARM's FP compare sets C=1 for
        // greater/equal/unordered, 0 for less; `ls` (= C==0 || Z==1) is how gcc
        // encodes FP `<=`. After a subtraction the stored C is x86-CF (borrow)
        // and JBE(CF||ZF) already works, so the FP C must be stored in the same
        // borrow sense (= !ARM_FP_C = CF && !PF) for the ls/hi/lo/hs conditions
        // to evaluate correctly. Fixes `nn<=0` on NaN (was true, must be false).
        buf.mov_rr64(RCX, RAX);      // CF
        buf.and_ri64(RCX, 1);
        buf.mov_rr64(RDI, RAX);      // PF
        buf.shr_ri8(RDI, 2);
        buf.and_ri64(RDI, 1);
        buf.xor_ri64(RDI, 1);        // !PF
        buf.and_rr64(RCX, RDI);      // CF && !PF (borrow-sense carry)
        buf.shl_ri8(RCX, 29);
        buf.or_rr64(RDX, RCX);
        // Z = ZF(bit6) && !PF(bit2) -> bit30. ARM FP-compare sets the Z flag
        // (==) for ORDERED equality ONLY: for an unordered (NaN) compare the
        // guest Z must be 0 so b.eq/csel.eq/b.gt/b.le all stay false (IEEE:
        // every NaN comparison is "not equal"). x86 comisd sets ZF=1 for BOTH
        // equality AND unordered, so ZF alone gives the wrong Z; mask PF (set
        // exactly when unordered) back out.
        buf.mov_rr64(RCX, RAX);      // ZF
        buf.shr_ri8(RCX, 6);
        buf.and_ri64(RCX, 1);
        buf.mov_rr64(RDI, RAX);      // PF
        buf.shr_ri8(RDI, 2);
        buf.and_ri64(RDI, 1);
        buf.xor_ri64(RDI, 1);        // !PF
        buf.and_rr64(RCX, RDI);      // ZF && !PF
        buf.shl_ri8(RCX, 30);
        buf.or_rr64(RDX, RCX);
        // N = CF && !ZF -> bit31. AArch64 FP compare sets N=1 for the ordered
        // "less-than" (d<f) case; x86 comisd clears CF only when A>B or A==B.
        // (Earlier this was hardcoded N=0, which made b.mi/b.lt/b.gt/b.le all
        // wrong: b.gt evaluated N==V as 0==0 for EVERY ordered non-equal pair.)
        //         N = (CF)        & (!ZF)
        buf.mov_rr64(RCX, RAX);
        buf.and_ri64(RCX, 1); // CF (bit0)
        buf.mov_rr64(RDI, RAX);
        buf.shr_ri8(RDI, 6);
        buf.and_ri64(RDI, 1); // ZF (bit6)
        buf.xor_ri64(RDI, 1); // !ZF
        buf.and_rr64(RCX, RDI); // N (0/1)
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
/// `sf` = operand size: when the guest op is 32-bit (`!sf`) and the shift is
/// ASR, the value has been zero-extended to 64 bits, so its bit-31 is NOT the
/// sign bit and a plain 64-bit `sar` would shift in zeros and give the wrong
/// (non-negative) result — e.g. compiler magic-division `sub w1,w1,w2,asr#31`
/// turns the sign-correction `-1` into `+1`, corrupting signed quotients.
/// Sign-extend the 32-bit value to 64 bits first so the sar replicates bit 31.
fn apply_shift_const(buf: &mut CodeBuf, x: u8, kind: ShiftKind, amt: u8, sf: bool) {
    if amt == 0 {
        return;
    }
    // BUGFIX (Session 99): this used `mov cl, amt; shl x, cl`. Both callers pass
    // x == RCX (the Rm value being shifted), so `mov rcx, amt` CLOBBERED the value
    // with the shift count and `shl rcx, cl` gave (amt << amt) — e.g. the array
    // index `add x1,x2,x0,lsl#3` became x2+24 (constant, not x0<<3), making loops
    // read the SAME element every iteration (fclamp -O2 returned 10 instead of 9).
    // Use the immediate-shift forms (C1 /4..7 ib) instead — no CL scratch, so the
    // shifted value stays in `x`.
    match kind {
        ShiftKind::Lsl => buf.shl_ri8(x, amt),
        ShiftKind::Lsr => buf.shr_ri8(x, amt),
        ShiftKind::Asr => {
            if !sf {
                buf.movsxd_r64_r32(x, x); // sign-extend bit-31 before 64-bit asr
            }
            buf.sar_ri8(x, amt);
        }
        ShiftKind::Ror => buf.ror_ri8(x, amt),
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
            rd, imm16, hw, opc, sf,
        } => {
            let val = (imm16 as u64) << ((hw as u64) * 16);
            match opc {
                2 => {
                    // movn: NOT the immediate. For a 32-bit (W) dest ARM zero-
                    // extends to the 64-bit register, so `movn w0,#2` writes
                    // 0x00000000fffffffd, NOT 0xfffffffffffffffd. mov_guest_imm
                    // would take the imm32 short-cut (`mov r32` sign-extends),
                    // leaving the upper 32 bits set; truncate first.
                    let v = !val;
                    mov_guest_imm(buf, rd as u32, if sf { v } else { v & 0xffff_ffff });
                }
                1 => {
                    // movk: read-modify-write — OR `imm16<<shift` into bits
                    // [shift, shift+16), preserving all other bits. Multi-part
                    // 64-bit constant build is `movz xD,#lo ; movk xD,#hi,lsl#16
                    // (or lsl#32/48)`; treating movk as a full replace corrupted
                    // the constant (e.g. 0x28bb1 built as movz 0x8bb1 then movk
                    // 0x2 lsl#16 came out 0x20000, breaking comparisons).
                    let shift = (hw as u32) * 16;
                    let mut clear = !(0xffffu64 << shift);
                    if !sf {
                        clear &= 0xffff_ffff; // W-dest zero-extends to 64 bits
                    }
                    ldg(buf, RAX, rd as u32);
                    buf.mov_ri64(RCX, clear);
                    buf.and_rr64(RAX, RCX);
                    buf.mov_ri64(RCX, (imm16 as u64) << shift);
                    buf.or_rr64(RAX, RCX);
                    stg(buf, rd as u32, RAX);
                }
                _ => mov_guest_imm(buf, rd as u32, val), // movz
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
            sf,
            ..
        } => {
            let imm: u64 = (imm12 as u64) << if shift12 { 12 } else { 0 };
            // rn XZR-vs-SP: `sub sp,sp,#imm` (non-S) reads rn=31 as SP, but the
            // flag-setting form (e.g. `cmp wzr,#imm`) reads rn=31 as XZR=0.
            if s && rn == 31 {
                buf.mov_ri64(RAX, 0);
            } else {
                ldg(buf, RAX, rn as u32);
            }
            if !sf {
                zext_w(buf, RAX); // 32-bit: high garbage in rn is ignored
            }
            if sub {
                if sf {
                    buf.sub_ri64(RAX, imm as u32);
                } else {
                    buf.sub_ri32(RAX, imm as u32); // 32-bit flags + upper-clear
                }
            } else if sf {
                buf.add_ri64(RAX, imm as u32);
            } else {
                buf.add_ri32(RAX, imm as u32); // `adds w…` N/V/Z/C from 32-bit result
            }
            if s {
                // ADDS/CMN (the non-subtract, flag-setting add) sets the ARM C
                // flag to the ADD's carry-out, but x86_cc_for_cond's HS/LO/HI/LS
                // conditions assume the stored C is the SUBTRACT-borrow
                // convention. gcc compiles `x > 0xffffffffffff0000` as
                // `cmn x,#0x10000; b.ls` — the add carries (ARM C=1, ls=false),
                // so the stored C must be !carry for b.ls/b.hi to evaluate
                // right. Complement CF before packing.
                if !sub {
                    buf.cmc(); // borrow-convention C = !carry-out
                }
                store_nzcv(buf); // N/Z/C/V -> CpuState.nzcv
            }
            // rd==31 writes SP for ADD/SUB (unlike logical ops where it's XZR and
            // discarded). The exception is cmp/cmn (s==1, rd==31) which must NOT
            // clobber SP. `sub sp,sp,#imm` (every function prologue) must write.
            if rd != 31 || !s {
                if !sf {
                    zext_w(buf, RAX); // W write zero-extends into X
                }
                stg(buf, rd as u32, RAX); // rd==31 -> writes the SP slot
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
            sf,
            sp_operand,
            ..
        } => {
            // rn/Rd XZR-vs-SP: the shift-register form (bit21=0) uses register 31 as
            // XZR (zero) in EVERY operand — `neg x6,x6` (sub x6,xzr,x6) must read rn=31
            // as 0, NOT SP. Only the extended-register form (bit21=1, e.g. `sub sp,sp,
            // x1`) treats rn=31 and rd=31 as the stack pointer. qemu-verified:
            //   neg x6,x6 = 0xcb0603e6 (bit21=0) -> rn=31 is XZR
            //   sub sp,sp,x1 = 0xcb2163ff (bit21=1) -> rn=31, rd=31 are SP.
            // `negs w1,w0` (subs,w1,wzr,w0; bit21=0, S=1) must read rn=31 as XZR too.
            // Rm is always a GPR (XZR=0), never SP.
            let rn_is_sp = sp_operand && rn == 31;
            if rn_is_sp {
                ldg(buf, RAX, 31); // extended form reads rn=31 as SP
            } else if rn == 31 {
                buf.mov_ri64(RAX, 0); // shifted form reads rn=31 as XZR
            } else {
                ldg(buf, RAX, rn as u32);
            }
            if rm == 31 {
                buf.mov_ri64(RCX, 0);
            } else {
                ldg(buf, RCX, rm as u32);
            }
            if !sf {
                zext_w(buf, RAX); // 32-bit add/sub: zero high garbage in operands
                zext_w(buf, RCX);
            }
            apply_shift_const(buf, RCX, shift, sh_amt, sf);
            if sub {
                if sf {
                    buf.sub_rr64(RAX, RCX);
                } else {
                    buf.sub_rr32(RAX, RCX); // `subs w…`: 32-bit flags + upper-clear
                }
            } else if sf {
                buf.add_rr64(RAX, RCX);
            } else {
                buf.add_rr32(RAX, RCX); // `adds w…`: N/V/Z/C from the 32-bit result
            }
            if s {
                if !sub {
                    buf.cmc(); // ADDS/CMN carries in borrow-convention (see AddSubImm)
                }
                store_nzcv(buf); // N/Z/C/V -> CpuState.nzcv
            }
            // rd==31: shifted-register form (bit21=0) discards the result (XZR) —
            // `neg xd,xm` does NOT touch SP. Only the extended-register form
            // (bit21=1, `sub sp,sp,x0`) writes rd=31 as the stack pointer; and
            // cmp/cmn (s==1) discard regardless. So write iff rd is a real reg,
            // or rd=31 in the SP-operand form that isn't a pure compare.
            if rd != 31 || (sp_operand && !s) {
                if !sf {
                    zext_w(buf, RAX); // W write zero-extends into X
                }
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::AddSubExt { rd, rn, rm, sub, s, sf, opt, shift } => {
            // add/sub Xd, Xn|SP, Rm, <opt> #<shift>: Rd = Xn + (ext(Rm)<<shift).
            // Extend Rm per `opt` (RXTB/UXTH/UXTW/SXTB/SXTH/SXTW are 32-bit-or-
            // narrow; UXTX/SXTX keep the full 64-bit Rm). rn/rd of 31 = SP.
            // Compute ext(Rm)<<shift in RAX, Xn(SP) in RDI, add/sub, store.
            if rm == 31 {
                buf.mov_ri64(RAX, 0); // W31/X31 is always XZR, never SP
            } else {
                ldg(buf, RAX, rm as u32);
            }
            match opt {
                0 => buf.and_ri64(RAX, 0xff), // UXTB
                1 => buf.and_ri64(RAX, 0xffff), // UXTH
                2 => buf.zero_ext_r32(RAX),   // UXTW
                3 => {}                       // UXTX (no-op)
                4 => {
                    // SXTB: sign-extend byte via shifts
                    buf.shl_ri8(RAX, 56);
                    buf.sar_ri8(RAX, 56);
                }
                5 => {
                    // SXTH
                    buf.shl_ri8(RAX, 48);
                    buf.sar_ri8(RAX, 48);
                }
                6 => buf.movsxd_r64_r32(RAX, RAX), // SXTW
                _ => {}                            // 7 = SXTX (no-op)
            }
            if shift > 0 && shift <= 3 {
                buf.shl_ri8(RAX, shift);
            }
            ldg(buf, RDI, rn as u32); // rn=31 -> x[31] = SP (extended form)
            if sub {
                buf.sub_rr64(RDI, RAX); // RDI = Rn - ext
                if s {
                    store_nzcv(buf);
                }
                buf.mov_rr64(RAX, RDI);
            } else {
                buf.add_rr64(RAX, RDI); // RAX = Rn + ext
                if s {
                    store_nzcv(buf);
                }
            }
            if !s {
                if !sf {
                    zext_w(buf, RAX); // 32-bit op: zero-extend the result
                }
                stg(buf, rd as u32, RAX); // rd=31 writes SP (extended form)
            }
            Ok(())
        }
        Inst::AddCarry { rd, rn, rm, sf, s, sub } => {
            // adc/sbc/adcs/sbcs Xd, Xn, Xm: Rd = Xn +/- Xm +/- carry.
            // AArch64 adds the previous C flag (NZCV bit29). SBC subtracts the
            // borrow (1 - C). Read the stored carry into x86 CF via
            // load_nzcv_to_eflags (preserves RAX/RCX), then use native adc/sbb.
            ldg(buf, RAX, rn as u32);
            ldg(buf, RCX, rm as u32);
            if !sf {
                // 32-bit form: zero the upper halves so the 64-bit adc/sbb below
                // yields exactly the 32-bit carry semantics.
                buf.shl_ri8(RAX, 32);
                buf.shr_ri8(RAX, 32);
                buf.shl_ri8(RCX, 32);
                buf.shr_ri8(RCX, 32);
            }
            // Inject stored C into CF (last op = popfq); RAX/RCX are preserved.
            load_nzcv_to_eflags(buf);
            // The stored C (nzcv bit29) is in the b.cond "borrow" convention:
            // store_nzcv for `adds` stores !carry-out, for `subs` stores borrow.
            // ADC/SBC consume the TRUE ARM carry, and in BOTH cases
            // TRUE_C = !stored_C, so
            //   adc adds TRUE_C = !C_s      -> cmc then native adc, and
            //   sbc subtracts 1-TRUE_C = C_s -> native sbb directly.
            if sub {
                buf.sbb_rr64(RAX, RCX); // RAX = rn - rm - C_s = rn - rm - (1-TRUE_C)
                if s {
                    // ARM sbcs sets C = not-borrow; report it in borrow-convention
                    // (= the x86 CF left by sbb), matching `subs`.
                    store_nzcv(buf);
                }
            } else {
                buf.cmc(); // CF = TRUE_C = !C_s
                buf.adc_rr64(RAX, RCX); // RAX = rn + rm + TRUE_C
                if s {
                    // adcs is an add: store C_s = !carry-out (like `adds`) so a
                    // later b.cond reads the right borrow-convention C.
                    buf.cmc();
                    store_nzcv(buf);
                }
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
            if rm == 31 {
                buf.mov_ri64(RCX, 0);
            } else {
                ldg(buf, RCX, rm as u32);
            }
            if !sf {
                zext_w(buf, RAX); // 32-bit ops: ignore high garbage in operands
                zext_w(buf, RCX);
            }
            apply_shift_const(buf, RCX, shift, sh_amt, sf);
            match op {
                0 => buf.and_rr64(RAX, RCX), // AND
                1 => buf.or_rr64(RAX, RCX),  // ORR
                2 => buf.xor_rr64(RAX, RCX), // EOR
                // Inverted (N=1) variants: AND/NOT, OR/NOT, XOR/NOT (BIC/ORN/EON).
                4 => {
                    buf.not_r64(RCX);
                    if !sf {
                        zext_w(buf, RCX); // 64-bit `not` sets high bits; 32-bit must not
                    }
                    buf.and_rr64(RAX, RCX); // BIC
                }
                5 => {
                    buf.not_r64(RCX);
                    if !sf {
                        zext_w(buf, RCX);
                    }
                    buf.or_rr64(RAX, RCX); // ORN
                }
                6 => {
                    buf.not_r64(RCX);
                    if !sf {
                        zext_w(buf, RCX);
                    }
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
                            if !sf {
                                zext_w(buf, RAX); // W write zero-extends into X
                            }
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
                            buf.zero_ext_r32(RAX);
                        }
                        if rd != 31 {
                            stg(buf, rd as u32, RAX);
                        }
                        Ok(())
                    }
                    Inst::MteTag { load, rt, rn, wb, wb_off } => {
                    // No MTE on the host and no tag state in the JIT: an
                    // allocation-tag STORE (stg/stzg/st2g) leaves memory
                    // untouched, but a writeback form (post/pre-index: bit10
                    // set) advances the base register Xn by the signed,
                    // granule-scaled immediate — a real architectural side
                    // effect (glibc memset/stg loops rely on it). The only
                    // load (ldg) returns tag 0 into the guest Xt register.
                    if load && rt != 31 {
                        buf.mov_ri64(RAX, 0);
                        stg(buf, rt as u32, RAX); // tag 0
                    } else if wb && rn != 31 {
                        // Xn += wb_off (both post- and pre-index end with the
                        // base advanced by the immediate).
                        ldg(buf, RAX, rn as u32);
                        if wb_off != 0 {
                            if wb_off > 0 && wb_off <= 0x7fffffff {
                                buf.add_ri64(RAX, wb_off as u32);
                            } else if wb_off < 0 && -wb_off <= 0x7fffffff {
                                buf.sub_ri64(RAX, -wb_off as u32);
                            } else {
                                buf.mov_ri64(RCX, wb_off as u64);
                                buf.add_rr64(RAX, RCX);
                            }
                        }
                        stg(buf, rn as u32, RAX);
                    }
                    Ok(())
                }
                Inst::CacheMaintain { zva, rt } => {
                    // Cache maintenance in the direct-mapped single-threaded JIT
                    // (guest==host, warm shared memory) is a no-op — except `dc
                    // zva` which zeros the 16-byte cache line at [Xt] (the block
                    // size we advertise via dczid_el0).
                    if zva && rt != 31 {
                        ldg(buf, RAX, rt as u32); // RAX = base address
                        buf.mov_ri64(RCX, 0);
                        buf.mov_store64(RAX, 0, RCX);   // [base+0]
                        buf.mov_store64(RAX, 8, RCX);   // [base+8]
                    }
                    Ok(())
                }
                Inst::MulHigh { rd, rn, rm, signed } => {
                    // umulh/smulh Xd, Xn, Xm: high 64 bits of the 128-bit product.
                    // x86 one-operand mul/imul: RDX:RAX = RAX * rm, high in RDX.
                    ldg(buf, RAX, rn as u32);  // multiplicand
                    ldg(buf, RCX, rm as u32);  // multiplier
                    if signed {
                        buf.imul_high_r64(RCX); // RDX:RAX = RAX*RCX (signed)
                    } else {
                        buf.mul_high_r64(RCX);  // RDX:RAX = RAX*RCX (unsigned)
                    }
                    if rd != 31 {
                        stg(buf, rd as u32, RDX); // high half -> Rd
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
                            if ra != 31 {
                                // ra is a real register: += / -= it. For `mul`,
                                // ra==31 means XZR (accumulate 0), NOT SP — ldg
                                // would read the stack pointer and corrupt the
                                // product with it.
                                ldg(buf, RDI, ra as u32);
                                if signed {
                                    // MSUB: Wd = Wa - Wn*Wm  (ra - rn*rm), NOT
                                    // rn*rm - ra. n - q*d compiled to msub was
                                    // returning -48 for 1298-25*50 (should be 48)
                                    // until this direction was fixed.
                                    buf.sub_rr64(RDI, RAX); // RDI = ra - rn*rm
                                    buf.mov_rr64(RAX, RDI);
                                } else {
                                    buf.add_rr64(RAX, RDI); // MADD: rn*rm + ra
                                }
                            }
                        }
                        if !sf {
                            buf.zero_ext_r32(RAX);
                        }
                        if rd != 31 {
                            stg(buf, rd as u32, RAX);
                        }
                        Ok(())
                    }
        Inst::MulLong { rd, rn, rm, ra, signed, sub } => {
            // smull/umull/smaddl/umaddl/smsubl/umsubl: 32-bit Rn*Rm -> 64-bit
            // product, optionally accumulated. Sign/zero-extend both operands
            // to 64 first, then imul: the low 64 of the signed product is
            // bit-identical to the unsigned one, and a 32x32 product always
            // fits in 64 bits (correct full result either way).
            ldg(buf, RAX, rn as u32);
            if signed {
                buf.movsxd_r64_r32(RAX, RAX);
            } else {
                buf.zero_ext_r32(RAX);
            }
            ldg(buf, RCX, rm as u32);
            if signed {
                buf.movsxd_r64_r32(RCX, RCX);
            } else {
                buf.zero_ext_r32(RCX);
            }
            buf.imul_rr64(RAX, RCX); // RAX = Rn * Rm (long)
            if ra != 31 {
                ldg(buf, RDI, ra as u32);
                if sub {
                    // msubl/umsubl: RAX = Ra - Rn*Rm
                    buf.neg_r64(RAX);
                    buf.add_rr64(RAX, RDI);
                } else {
                    buf.add_rr64(RAX, RDI); // maddl/umaddl: RAX = Rn*Rm + Ra
                }
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
                buf.zero_ext_r32(RAX);
            }
            if sf {
                // lzcnt rax, rax. REX.W (0x48) MUST come AFTER the F3 prefix and
                // immediately before the 0F opcode: emitted as `48 F3 0F BD` the
                // CPU ignores REX.W (it must be the last prefix) and executes a
                // 32-bit lzcnt eax — clz(x) with x<2^32 returns 32-len(x), not
                // 64-len(x) (real repro: clz(0x16136740) returned 3, oracle 35).
                buf.bytes.extend_from_slice(&[0xf3, 0x48, 0x0f, 0xbd, 0xc0]);
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
                        // BUGFIX: mask with the FULL 64-bit value (mov_ri64 into
                        // R10 + and_rr64), NOT `and_ri64(m as u32)` which truncated
                        // every mask to 32 bits — 0x5555..55, 0x3333..33, ... are
                        // 64-bit patterns, so only the LOW 32 bits of x were being
                        // reversed and the high 32 used a corrupt (low-32-only) mask
                        // (rbit x; clz x = ctz returned 198+ vs oracle 10). RAX is
                        // the running result; RCX holds the shifted-high piece.
                        buf.mov_rr64(RCX, RAX);
                        buf.shr_ri8(RCX, *sh);
                        buf.mov_ri64(R10, *m);
                        buf.and_rr64(RCX, R10); // (x >> sh) & m
                        buf.and_rr64(RAX, R10); // x & m  (note: NOT or'ing x in; RAX is masked then shifted)
                        buf.shl_ri8(RAX, (1u32 << k as u32) as u8); // (x & m) << sh
                        buf.or_rr64(RAX, RCX); // high-shifted | low-shifted
                    }
                }
            }
            if !sf {
                buf.zero_ext_r32(RAX);
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
            // CSEL is a data-processing family: register operand 31 is XZR (0),
            // NEVER SP (that distinction only exists in the add/sub extended-
            // register forms). `cset/cinc/csneg` rely on rn=rm=31 reading as 0.
            if rn == 31 {
                buf.mov_ri64(RDI, 0); // then: XZR
            } else {
                ldg(buf, RDI, rn as u32); // then: rn
            }
            if rm == 31 {
                buf.mov_ri64(R10, 0); // else: XZR
            } else {
                ldg(buf, R10, rm as u32); // else: f(rm)
            }
            match op {
                0 => {}
                1 => buf.add_ri64(R10, 1),   // csinc / cset / cinc
                2 => buf.not_r64(R10),       // csinv
                3 => buf.neg_r64(R10),       // csneg
                _ => return Err(format!("CSel op {} not implemented", op)),
            }
            // 32-bit (W) destination: the result is the low 32 bits ZERO-
            // extended to the 64-bit register. The not/neg/inc transforms and
            // rn may carry high garbage (e.g. csinv ~5 = 0xfffffffffffffffa,
            // csneg -5 = 0xfffffffffffffffb); without truncation the upper
            // half silently leaks into x0.
            if !sf {
                // zero_ext_r32 (mov r32,r32, no REX.W): the high-register-aware
                // way to clear the upper 32 bits. zext_w's shl/shr path uses
                // 0x48-only (no REX.B) emitters and would corrupt R10.
                buf.zero_ext_r32(RDI);
                buf.zero_ext_r32(R10);
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
            sext,
        } => {
            // address = rn + imm*size (scaled byte offset)
            ldg(buf, RDX, rn as u32); // pointer operand into RDX
            let off = (imm as i32).checked_mul(size as i32).unwrap_or(0);
            if off != 0 {
                buf.lea64(RDX, RDX, off);
            }
            if ld && sext {
                // Sign-extending load (ldrsw/ldrsh/ldrsb): load `size` bytes and
                // sign-extend into the full 64-bit X dest (bit23+bit22==0 form).
                match size {
                    4 => {
                        buf.mov_load32(RAX, RDX, 0);
                        buf.movsxd_r64_r32(RAX, RAX);
                        stg_if_writable(buf, rt as u32);
                    }
                    2 => {
                        buf.movzx_word_mem(RAX, RDX, 0);
                        buf.shl_ri8(RAX, 48);
                        buf.sar_ri8(RAX, 48); // 16->64 sign-extend
                        stg_if_writable(buf, rt as u32);
                    }
                    1 => {
                        buf.movzx_byte_mem(RAX, RDX, 0);
                        buf.shl_ri8(RAX, 56);
                        buf.sar_ri8(RAX, 56); // 8->64 sign-extend
                        stg_if_writable(buf, rt as u32);
                    }
                    s => return Err(format!("LdStrImm sign-extend size {s} not implemented (pc {pc:#x})")),
                }
                return Ok(());
            }
            match (size, ld) {
                (8, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (8, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (4, true) => {
                    buf.mov_load32(RAX, RDX, 0); // w zero-extends
                    stg_if_writable(buf, rt as u32);
                }
                (4, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0); // ldrh zero-extends
                    stg_if_writable(buf, rt as u32);
                }
                (2, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (1, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0); // ldrb zero-extends
                    stg_if_writable(buf, rt as u32);
                }
                (1, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store8(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("LdStrImm size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::LseAtomic { op, size64, rs, rn, rt } => {
            // Single-threaded emulation: old = [Xn]; [Xn] = f(old, Rs); Rt = old.
            // op: 0=LDADD 1=LDCLR 2=LDEOR 3=LDSET 4=SWP.
            ldg(buf, RDX, rn as u32); // address
            if size64 {
                buf.mov_load64(RAX, RDX, 0);
            } else {
                buf.mov_load32(RAX, RDX, 0); // W: 32-bit load, zero-extends
            }
            buf.mov_rr64(RDI, RAX);      // RDI holds the old value -> Rt later
            ldg(buf, RCX, rs as u32);
            if size64 {
                match op {
                    0 => buf.add_rr64(RAX, RCX),
                    1 => {
                        buf.not_r64(RCX);
                        buf.and_rr64(RAX, RCX);
                    }
                    2 => buf.xor_rr64(RAX, RCX),
                    3 => buf.or_rr64(RAX, RCX),
                    _ => buf.mov_rr64(RAX, RCX), // SWP
                }
            } else {
                buf.zero_ext_r32(RCX);
                match op {
                    0 => buf.add_rr64(RAX, RCX),
                    1 => {
                        buf.not_r64(RCX);
                        buf.and_rr64(RAX, RCX);
                    }
                    2 => buf.xor_rr64(RAX, RCX),
                    3 => buf.or_rr64(RAX, RCX),
                    _ => buf.mov_rr64(RAX, RCX),
                }
            }
            // write the updated value back to [Xn]
            if size64 {
                buf.mov_store64(RDX, 0, RAX);
            } else {
                buf.mov_store32(RDX, 0, RAX);
            }
            // Rt = old value (zero-extended for W)
            if rt != 31 {
                if !size64 {
                    zext_w(buf, RDI);
                }
                buf.mov_rr64(RAX, RDI);
                stg(buf, rt as u32, RAX);
            }
            Ok(())
        }
        Inst::LdStrImmWb {
            rt,
            rn,
            imm9,
            size,
            ld,
            sext,
            writeback,
            pre,
        } => {
            // pre-index [Xn,#imm9]!  -> access at Xn+imm9, then Xn += imm9
            // post-index [Xn],#imm9   -> access at Xn,     then Xn += imm9
            // unscaled   [Xn,#imm9]   -> access at Xn+imm9, no writeback
            let access_off = if writeback && !pre { 0 } else { imm9 };
            ldg(buf, RDX, rn as u32); // base
            if ld && sext {
                // sign-extending load (ldrsw/ldrsh/ldrsb) at [RDX+access_off]
                match size {
                    4 => {
                        buf.mov_load32(RAX, RDX, access_off);
                        buf.movsxd_r64_r32(RAX, RAX);
                        stg_if_writable(buf, rt as u32);
                    }
                    2 => {
                        buf.movzx_word_mem(RAX, RDX, access_off);
                        buf.shl_ri8(RAX, 48);
                        buf.sar_ri8(RAX, 48);
                        stg_if_writable(buf, rt as u32);
                    }
                    1 => {
                        buf.movzx_byte_mem(RAX, RDX, access_off);
                        buf.shl_ri8(RAX, 56);
                        buf.sar_ri8(RAX, 56);
                        stg_if_writable(buf, rt as u32);
                    }
                    s => return Err(format!("LdStrImmWb sign-extend size {s} not implemented")),
                }
            } else {
                match (size, ld) {
                    (8, true) => {
                        buf.mov_load64(RAX, RDX, access_off);
                        stg_if_writable(buf, rt as u32);
                    }
                    (8, false) => {
                        ldg_src(buf, rt as u32);
                        buf.mov_store64(RDX, access_off, RAX);
                    }
                    (4, true) => {
                        buf.mov_load32(RAX, RDX, access_off);
                        stg_if_writable(buf, rt as u32);
                    }
                    (4, false) => {
                        ldg_src(buf, rt as u32);
                        buf.mov_store32(RDX, access_off, RAX);
                    }
                    (2, true) => {
                        buf.movzx_word_mem(RAX, RDX, access_off);
                        stg_if_writable(buf, rt as u32);
                    }
                    (2, false) => {
                        ldg_src(buf, rt as u32);
                        buf.mov_store16(RDX, access_off, RAX);
                    }
                    (1, true) => {
                        buf.movzx_byte_mem(RAX, RDX, access_off);
                        stg_if_writable(buf, rt as u32);
                    }
                    (1, false) => {
                        ldg_src(buf, rt as u32);
                        buf.mov_store8(RDX, access_off, RAX);
                    }
                    (s, _) => {
                        return Err(format!("LdStrImmWb size {s} not implemented"))
                    }
                }
            }
            // writeback: Xn += imm9 (signed; add_ri64 sign-extends the imm32).
            if writeback {
                ldg(buf, RAX, rn as u32);
                if imm9 != 0 {
                    buf.add_ri64(RAX, imm9 as u32);
                }
                stg(buf, rn as u32, RAX);
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
                    stg_if_writable(buf, rt as u32);
                }
                (3, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (2, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (1, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (1, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (0, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (0, false) => {
                    ldg_src(buf, rt as u32);
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
                    stg_if_writable(buf, rt as u32);
                }
                (3, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store64(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (2, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (2, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store32(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (1, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (1, false) => {
                    ldg_src(buf, rt as u32);
                    buf.mov_store16(RDX, 0, RAX);
                    stg0(buf, rs as u32);
                }
                (0, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    stg_if_writable(buf, rt as u32);
                }
                (0, false) => {
                    ldg_src(buf, rt as u32);
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
                buf.zero_ext_r32(RAX);
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Extr { rd, rn, rm, lsb, sf } => {
            // EXTR: Xd = (Xn << (bits - lsb)) | (Xm >> lsb)  — ARM concatenates
            // Xn as the HIGH word and Xm as the LOW word of a 2*bits value and
            // shifts right by lsb. (The Ror alias rm==rn is handled separately.)
            // NOTE: the earlier implementation had the operand order INVERTED
            // ((Xn >> lsb) | (Xm << (bits-lsb))); the rotate case rn==rm is
            // symmetric so it hid the bug, but gcc's real 128-bit shifts
            // (e.g. `extr x1, x2, x1, #32` for a cross-word shift) got the two
            // operands swapped and returned garbage.
            let bits = if sf { 64u32 } else { 32u32 };
            let lsb = lsb & (bits - 1);
            let hi_part = (bits - lsb) & (bits - 1); // Xn << (bits-lsb)
            // low part: Xm >> lsb
            ldg(buf, RAX, rm as u32);
            if lsb != 0 {
                buf.shr_ri8(RAX, lsb as u8);
            }
            // high part: Xn << (bits-lsb). lsb==0 would shift by `bits` (=0 in a
            // 64-bit reg), so force it to 0 explicitly (the high word plays no role).
            ldg(buf, R10, rn as u32);
            if lsb == 0 {
                buf.mov_ri64(R10, 0);
            } else if hi_part != 0 {
                buf.shl_ri8(R10, hi_part as u8);
            }
            buf.or_rr64(RAX, R10);
            if !sf {
                buf.zero_ext_r32(RAX);
            }
            stg(buf, rd as u32, RAX);
            Ok(())
        }
        Inst::Svc { imm: _ } => {
            // Route the supervisor call to the host `guest_svc` dispatcher (this
            // is the hookpoint for real AArch64->host syscall routing).
            let addr = crate::jit::guest_svc as usize as u64;
            // Record the post-svc guest address (pc+4) into CpuState.svc_next
            // BEFORE the call. `clone` (220) re-enters jit_run at this address
            // for the child thread, so the child continues right after the svc.
            // pc is the current instruction's guest address (translate passes it).
            buf.mov_ri64(RAX, pc.wrapping_add(4));
            buf.mov_store64(RBX, crate::jit::SVC_NEXT_OFF, RAX); // state.svc_next = pc+4
            // guest_svc(st) is extern "C" fn(*mut CpuState)->u64: arg0 (state)
            // must go in RDI explicitly. RDI at block entry does hold the state
            // pointer (we are f(state)), so the FIRST inline call worked by
            // accident — but a prior host call clobbers RDI (caller-saved), so
            // any later svc passed garbage. This was a real latent bug.
            buf.mov_rr64(RDI, RBX); // arg0 = state
            buf.mov_ri64(RAX, addr);
            // The JIT block body runs at host RSP ≡ 8 (mod 16) — the correct
            // callee-entry alignment for guest-to-guest BL (call_rel32) — but
            // SysV requires RSP ≡ 0 at the *call* site of a host function. An
            // inline host callee (guest_svc, guest_sha1stem) does its own
            // aligned stack work (prologue push, SSE locals; e.g. format! in
            // tracing), so a misaligned call faults. Align around the call.
            buf.sub_ri64(4, 8); // RSP(4) -= 8  ->  RSP ≡ 0 mod 16 at the call
            buf.call_r64(RAX); // guest_svc(st); returns the syscall result in RAX
            buf.add_ri64(4, 8); // RSP += 8  ->  back to block-entry alignment
            // Thread-exit AND signal-redirect early-return. `state.pc` is NOT
            // updated per-instruction during block execution (it holds the
            // block-entry address), so we can't compare it to `svc_next`.
            // A signal redirect is decided FIRST, BEFORE the syscall-result
            // store: when a self-delivered signal set `redirect_request =
            // handler`, guest x0 must stay the signal handler's signo argument
            // (set by signals::begin_handler), NOT the syscall return — so we
            // yield to the dispatcher without overwriting x0. Only on the
            // non-redirect path do we store the syscall result (x0) and then
            // early-return when `pc == 0` (a spawned child's thread-local
            // `exit`). Both redirect and pc==0 return from the block so the
            // dispatcher loop re-reads `state.pc`.
            buf.mov_load64(RCX, RBX, crate::jit::REDIRECT_OFF); // RCX = redirect
            buf.test_rr64(RCX, RCX);
            buf.jne_rel8(16); // redirect != 0 -> jump to the yield `ret` (16 on)
            stg(buf, 0, RAX); // system value -> guest x0 (AArch64 return reg)
            buf.mov_load64(RAX, RBX, crate::jit::PC_OFF); // RAX = state.pc
            buf.test_rr64(RAX, RAX);
            buf.jne_rel8(2); // pc != 0 -> skip both `ret`s, continue inline
            buf.ret(); // pc == 0: thread-local exit -> return to the dispatcher
            buf.ret(); // redirect != 0: yield to the dispatcher (x0 keeps signo)
            Ok(()) // <-- continue inline after the rets (pc != 0, redirect == 0)
        }
        Inst::Brk { imm } => {
            // Guest breakpoint (brk #imm): on a real AArch64 CPU this traps
            // (SIGTRAP). Mirror that for the JIT by halting the run loop
            // gracefully — set guest pc = 0, the same sentinel `run_loop`
            // treats as a clean halt (it returns Ok(x[0])).
            let _ = imm;
            buf.mov_ri64(RAX, 0);
            buf.mov_store64(RBX, crate::jit::PC_OFF, RAX); // state.pc = 0
            Ok(())
        }
        Inst::Udf { imm } => {
            // Undefined instruction (udf #imm): a real AArch64 CPU faults here
            // (Undefined Instruction exception). Mirror Brk by halting the run
            // loop gracefully (state.pc = 0 sentinel).
            let _ = imm;
            buf.mov_ri64(RAX, 0);
            buf.mov_store64(RBX, crate::jit::PC_OFF, RAX); // state.pc = 0
            Ok(())
        }
        Inst::BitField { rd, rn, immr, imms, sf, arith, insert } => {
            let bits = if sf { 64u32 } else { 32u32 };
            // ---- BFM insert (opc=01): merge bits of Rn into Rd, preserving
            // Rd's bits outside the field. BFXIL (immr<=imms, copy in place) and
            // BFI/BFC (immr>imms, wrap) BOTH merge; they are NOT the UBFM/SBFM
            // shift/extract aliases below, which must not fire for insert==true
            // (e.g. a bfi whose immr/imms coincidentally satisfy the ROR
            // shortcut 15+48+1==64 would be mis-compiled as a rotate).
            if insert {
                let mask: u64;
                if immr <= imms {
                    // BFXIL: field spans [immr, imms] in place — Rn's bits are
                    // already at the target position, so mask in place (no shift).
                    let width = (imms - immr + 1) as u32;
                    mask = (((1u64 << width) - 1) << immr) & (if sf { u64::MAX } else { 0xffff_ffff });
                    ldg(buf, RAX, rn as u32); // Rn
                } else {
                    // BFI/BFC: wrap, lsb=(bits-immr)&(bits-1), width=imms+1.
                    // Rn's low `width` bits are shifted up to `lsb`, THEN masked.
                    let lsb = (bits - immr) & (bits - 1);
                    let width = imms + 1;
                    mask = (((1u64 << width) - 1) << lsb) & (if sf { u64::MAX } else { 0xffff_ffff });
                    ldg(buf, RAX, rn as u32); // Rn
                    buf.shl_ri8(RAX, lsb as u8); // field << lsb
                }
                ldg(buf, R10, rd as u32); // old Rd
                buf.mov_ri64(RCX, mask);
                buf.and_rr64(RAX, RCX); // field of Rn
                // rd = (rd & ~mask) | (Rn & mask)
                buf.not_r64(RCX);
                buf.and_rr64(R10, RCX);
                buf.or_rr64(R10, RAX);
                buf.mov_rr64(RAX, R10);
                if !sf {
                    buf.zero_ext_r32(RAX);
                }
                if rd != 31 {
                    stg(buf, rd as u32, RAX);
                }
                return Ok(());
            }
            // ---- UBFM/SBFM (insert=false): shift / extract / sign-extend.
            ldg(buf, RAX, rn as u32); // load Rn
            if !sf && !arith && !insert {
                // W-form logical bitfield (lsr w, ubfx, lsl w, ror w, etc.): the
                // source register is only 32 bits wide, so the UPPER 32 bits of
                // the 64-bit slot (left by a prior X-form write such as a 64-bit
                // madd/mul) must be discarded BEFORE any shift/rotate. Otherwise
                // high-bit guest garbage shifts down into the low result —
                // e.g. `ldr x1; madd x1,x1,x4,x3; lsr w5,w1,#24` pulled bits 32-55
                // of x1 into the extracted byte (real repro: 8652 -> 0x37562e2cc).
                buf.zero_ext_r32(RAX);
            }
            if imms == bits - 1 {
                // LSR (logical) or ASR (arithmetic/sign) by immr
                let sh = (immr & (bits - 1)) as u8;
                if arith {
                    if sf {
                        buf.sar_ri8(RAX, sh);
                    } else {
                        buf.sar32_ri8(RAX, sh);
                    }
                } else {
                    buf.shr_ri8(RAX, sh);
                }
            } else if immr == (imms + 1) % bits {
                // LSL (alias) : shift left by (bits-1-imms)
                let sh = ((bits - 1 - imms) & (bits - 1)) as u8;
                buf.shl_ri8(RAX, sh);
            } else if immr > imms {
                // UBFIZ (logical) / SBFIZ (arith): zero- or sign-extending
                // shift-left. Xd = (Rn << lsb) & mask, upper bits zeroed
                // (UBFIZ) or sign-replicated (SBFIZ). Old Rd is DISCARDED; the
                // old code merged old Rd here (BFI behavior), so `ubfiz
                // x4,x0,#7,#32` kept stale upper bits of x4.
                let lsb = (bits - immr) & (bits - 1);
                let width = imms + 1;
                let mask = ((1u64 << width) - 1) << lsb;
                buf.shl_ri8(RAX, lsb as u8); // Rn << lsb
                buf.mov_ri64(RCX, mask);
                buf.and_rr64(RAX, RCX); // field only (zero-extended)
                if arith {
                    // SBFIZ: sign-extend the field from bit (lsb+width-1).
                    let se = (bits - (lsb + width)) as u8;
                    buf.shl_ri8(RAX, se);
                    if sf { buf.sar_ri8(RAX, se); } else { buf.sar32_ri8(RAX, se); }
                }
            } else {
                // general UBFM/SBFM extract: (Rn >> immr) & low(width) bits,
                // then optionally sign-extend from `width`.
                let width = imms - immr + 1;
                let sh = (immr & (bits - 1)) as u8;
                buf.shr_ri8(RAX, sh); // drop low immr bits
                // keep only `width` low bits
                if width < bits {
                    let mask: u64 = (1u64 << width) - 1;
                    // BUGFIX: `and_ri64(RAX, m as u32)` emits a 64-bit AND with a
                    // SIGN-EXTENDED imm32, so any mask with bit31 set (width>=32:
                    // mask = 0xffffffff) silently became 0xffffffffffffffff — a
                    // no-op that left the high 32 bits of the shifted value
                    // corrupting the extract (ubfx x,#25,#32 returned garbage).
                    // Mask must fit in the positive imm32 range to use the fast
                    // path; otherwise load the full 64-bit mask.
                    if mask <= 0x7fff_ffff {
                        buf.and_ri64(RAX, mask as u32);
                    } else {
                        buf.mov_ri64(RCX, mask);
                        buf.and_rr64(RAX, RCX);
                    }
                }
                if arith {
                    // sign-extend the `width`-bit field to `bits`:
                    // shift left to push the sign bit to the top, then arithmetic
                    // shift right back (replicates the sign). For a 32-bit field
                    // the sign must be taken from bit31, not bit63 (which a 64-bit
                    // `sar` would read on a zero-extended value).
                    let se = (bits - width) as u8;
                    buf.shl_ri8(RAX, se);
                    if sf { buf.sar_ri8(RAX, se); } else { buf.sar32_ri8(RAX, se); }
                }
            }
            if !sf {
                buf.zero_ext_r32(RAX);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::SysReg { sysreg, rt, read } => {
            // sysreg==0: tpidr_el0 (CpuState.tpidr). sysreg==1: cntfrq_el0
            // (counter tick rate in Hz) read as a fixed constant. Decode only
            // produces these; write to cntfrq is not generated.
            //
            // NOTE: rt is a GUEST register index. The value must be committed to
            // the guest file with stg(buf, rt, <host>) — a bare buf.mov_ri64(rt,
            // ..) only writes HOST register rt and the guest slot stays stale
            // (latent bug: every mrs xN,<cnt|nzcv|dczid> was a silent no-op).
            if sysreg == 1 {
                // mrs xN, cntfrq_el0  ->  xN = 100_000_000 (100 MHz counter).
                // Constant Hz: the guest divides/downscales counter deltas with
                // this, so a fixed, self-consistent rate is honest for boot.
                if read && rt != 31 {
                    buf.mov_ri64(RAX, 100_000_000);
                    stg(buf, rt as u32, RAX);
                }
                return Ok(());
            }
            if sysreg == 3 {
                // mrs xN, cntvct_el0  ->  xN = CpuState.cntvct (live monotonic
                // counter, stamped by the run loop between guest blocks).
                if read && rt != 31 {
                    buf.mov_load64(RAX, RBX, crate::jit::CNTVCT_OFF);
                    stg(buf, rt as u32, RAX);
                }
                return Ok(());
            }
            if sysreg == 4 {
                // mrs xN, nzcv  ->  xN = CpuState.nzcv (packed N=31,Z=30,C=29,V=28).
                if read && rt != 31 {
                    buf.mov_load32(RAX, RBX, NZCV_OFF);
                    stg(buf, rt as u32, RAX);
                }
                if !read && rt != 31 {
                    // msr nzcv, xN: shift xN's low 4 flag bits up to NZCV@28..31.
                    ldg_src(buf, rt as u32);
                    buf.and_ri64(RAX, 0x0f);
                    buf.shl_ri8(RAX, 28);
                    buf.mov_store32(RBX, NZCV_OFF, RAX);
                }
                return Ok(());
            }
            if sysreg == 5 {
                // mrs xN, dczid_el0  ->  xN = 0x4 (16-byte DC ZVA block, DZP=0).
                if read && rt != 31 {
                    buf.mov_ri64(RAX, 0x4);
                    stg(buf, rt as u32, RAX);
                }
                return Ok(());
            }
            if sysreg == 6 || sysreg == 7 || sysreg == 8 {
                // GCS / SME-TLS / MIDR registers the JIT neither enables nor
                // models: sysreg 6 = gcspr_el0 (armv9 GCS pointer; 0 when GCS
                // disabled), sysreg 7 = tpidr2_el0 (SME second TLS pointer; 0
                // without SME), sysreg 8 = midr_el1 (implementer/part; 0 =
                // unknown core so glibc picks generic non-SME/SVE paths).
                // Both read 0 on a fresh EL0 context that never enables the
                // feature — matching what a real core reports at boot.
                // (All three read 0; writes are no-ops.)
                if read && rt != 31 {
                    buf.mov_ri64(RAX, 0);
                    stg(buf, rt as u32, RAX);
                }
                return Ok(());
            }
            if sysreg == 9 {
                // msr fpcr, xN: FP control write. The JIT pins all FP rounding
                // per-op (roundsd modes, cvt*), never reading FPCR, so accept any
                // value silently. (read never produced for fpcr.)
                return Ok(());
            }
            if sysreg == 10 {
                // mrs xN, ctr_el0: cache-type register. Report 16-byte lines:
                // IminLine(15:0)=2, DminLine(19:16)=2, Cwg(23:20)=0, DIC/IDC=0.
                if read && rt != 31 {
                    buf.mov_ri64(RAX, 0x0002_0002); // Dmin=2<<16 | Imin=2
                    stg(buf, rt as u32, RAX);
                }
                return Ok(());
            }
            if read {
                // Rt = [RBX + TPIDR_OFF]  (commit to the guest slot, same fix as
                // the other MRS reads: rt is a guest register, not a host one)
                if rt != 31 {
                    buf.mov_load64(RAX, RBX, crate::jit::TPIDR_OFF);
                    stg(buf, rt as u32, RAX);
                }
            } else {
                // tpidr_el0 = Rt
                if rt != 31 {
                    ldg_src(buf, rt as u32);
                    buf.mov_store64(RBX, crate::jit::TPIDR_OFF, RAX);
                }
            }
            Ok(())
        }
        Inst::Fma3 { rd, rn, rm, ra, sz, sub, neg } => {
            // Scalar 3-source FP: Dd = Da +- (Dn × Dm), optionally negated.
            //   fmadd(0,0)=ra+rn*rm  fmsub(1,0)=ra-rn*rm
            //   fnmadd(0,1)=-(ra+rn*rm)  fnmsub(1,1)=rn*rm-ra
            // xmm0=rn*rm, xmm1=rm(srca), xmm2=ra, xmm3=scratch(negate).
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if sz {
                buf.movq_load(0, RBX, vslot(rn));
                buf.movq_load(1, RBX, vslot(rm));
                buf.mulsd(0, 1); // xmm0 = rn*rm
                buf.movq_load(2, RBX, vslot(ra));
                let result = match (sub, neg) {
                    (false, false) => {
                        buf.addsd(2, 0); // ra + prod
                        2
                    }
                    (true, false) => {
                        buf.subsd(2, 0); // ra - prod
                        2
                    }
                    (false, true) => {
                        buf.addsd(2, 0); // ra + prod
                        buf.pxor_xmm(3, 3); // 0.0
                        buf.subsd(3, 2); // 0 - (ra+prod) = negate
                        3
                    }
                    (true, true) => {
                        buf.subsd(0, 2); // prod - ra
                        0
                    }
                };
                buf.movq_store(RBX, vslot(rd), result);
            } else {
                let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                let ld = |buf: &mut CodeBuf, x: u8, r: u8| {
                    buf.mov_load32(RAX, RBX, f(r));
                    buf.movd_xmm_r32(x, RAX);
                };
                // xmm0 = rn, xmm1 = rm, then xmm0 *= xmm1 (rcx), xmm2 = ra.
                ld(buf, 0, rn);
                ld(buf, 1, rm);
                buf.mulss(0, 1); // xmm0 = rn*rm
                ld(buf, 2, ra);
                match (sub, neg) {
                    (false, false) => buf.addss(2, 0), // xmm2 = ra + prod
                    (true, false) => buf.subss(2, 0),  // xmm2 = ra - prod
                    (false, true) => {
                        buf.addss(2, 0); // xmm2 = ra + prod
                        buf.pxor_xmm(3, 3);
                        buf.subss(3, 2); // 0 - xmm2
                    }
                    (true, true) => {
                        buf.movd_xmm_r32(2, RAX); // refresh xmm2 unused; do prod - ra
                        buf.subss(0, 2); // xmm0 = prod - ra
                    }
                }
                let result = if sub && neg { 0 } else if neg { 3 } else { 2 };
                buf.movd_r32_xmm(RAX, result);
                buf.mov_store32(RBX, f(rd), RAX);
            }
            Ok(())
        }
        Inst::FpScalar { rd, rn, rm, op, sz, half } => {
            // scalar FP on d/s/h regs. d-reg = low 8 bytes of CpuState.v[reg].slot
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16; // low 8B of a 16B slot
            // ops 0-3 (fmov/fabs/fneg) are double-only; single handled for 4-7 below.
            if !sz && op <= 3 && !half {
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
                    if half {
                        // Half-precision (FP16) scalar arithmetic: promote both h
                        // operands (low 16 bits of each slot) to f32 via F16C
                        // vcvtph2ps, do the f32 op, then demote to f16 (RN) and
                        // store the low 16 bits. Encodings (F16C, immediate 0 = RN):
                        //   vcvtph2ps xmm,xmm  = C4 E2 79 13 /r (reg form)
                        //   vcvtps2ph $0,xmm,xmm = C4 E3 79 1D /r 00
                        let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                        buf.mov_load32(RAX, RBX, f(rn)); // h{rn} in low 16
                        buf.movd_xmm_r32(0, RAX);        // xmm0 low 32 = h{rn}
                        buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                        buf.mov_load32(RAX, RBX, f(rm));
                        buf.movd_xmm_r32(1, RAX);
                        buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc9]); // vcvtph2ps xmm1,xmm1
                        let opcode: u8 = match op {
                            4 => 0x59, // mulss
                            5 => 0x58, // addss
                            6 => 0x5c, // subss
                            7 => 0x5e, // divss
                            _ => unreachable!(),
                        };
                        buf.bytes.extend_from_slice(&[0xf3, 0x0f, opcode, 0xc1]); // opss xmm0,xmm1
                        buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                        buf.movd_r32_xmm(RAX, 0); // RAX low 16 = f16 result
                        buf.mov_store32(RBX, f(rd), RAX); // store low 32 (f16 in low 16)
                    } else if sz {
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
            // scalar 1-source FP: fsqrt / frint{mpz} / fabs / fneg.
            // d-reg = low 8B of yate.v[reg]; s-reg = low 4B.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if !sz {
                // single-precision: operate on the low 32 bits.
                buf.mov_load32(RAX, RBX, vslot(rn));
                match op {
                    5 => { // fabs s: clear sign bit
                        buf.mov_ri64(RCX, 0x7fff_ffff);
                        buf.and_rr64(RAX, RCX);
                    }
                    6 => { // fneg s: flip sign bit
                        buf.mov_ri64(RCX, 0x8000_0000);
                        buf.xor_rr64(RAX, RCX);
                    }
                    _ => {
                // frintm/p/z (floor/ceil/trunc) and fsqrt via double round-trip.
                buf.movd_xmm_r32(0, RAX);        // move the loaded float bits into xmm0.low32
                                                 // (cvtss2sd reads xmm0 low32, not RAX)
                buf.cvtss2sd(0, 0);              // xmm0 = (double)xmm0.low32
                match op {
                    0 => buf.sqrtsd(0, 0),       // fsqrt s
                    1 => buf.roundsd(0, 0, 0b01), // frintm s: floor
                    2 => buf.roundsd(0, 0, 0b10), // frintp s: ceil
                    3 => buf.roundsd(0, 0, 0b11), // frintz s: trunc (toward zero; 0b00 is round-to-NEAREST)
                    7 => {
                        // frinta s: round half-away = sign*floor(|x|+0.5) on the sd'd val
                        buf.movq_r64_xmm(RAX, 0);
                        buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                        buf.and_rr64(RAX, RCX);
                        buf.movq_xmm_r64(2, RAX);             // sign mask
                        buf.movq_r64_xmm(RAX, 0);
                        buf.mov_ri64(RCX, 0x7fff_ffff_ffff_ffff);
                        buf.and_rr64(RAX, RCX);
                        buf.movq_xmm_r64(0, RAX);             // |x|
                        buf.mov_ri64(RAX, 0x3fe0_0000_0000_0000); // 0.5
                        buf.movq_xmm_r64(1, RAX);
                        buf.addsd(0, 1);                       // |x|+0.5
                        buf.roundsd(0, 0, 0x01);               // floor
                        buf.pxor_xmm(0, 2);                    // sign
                    }
                    8 | 9 => buf.roundsd(0, 0, 0x00), // frintn/frintx s: nearest, ties-even
                    _ => return Err(format!("FpUnary single ilp-{op} not implemented")),
                }
                buf.cvtsd2ss(0, 0);              // back to single (xmm0 low 32)
                buf.movd_r32_xmm(RAX, 0);          // xmm0 low32 -> RAX
                buf.mov_store32(RBX, vslot(rd), RAX); // store s-reg
                return Ok(());
            }
                }
                buf.mov_store32(RBX, vslot(rd), RAX);
                return Ok(());
            }
            buf.movq_load(0, RBX, vslot(rn));
            match op {
                0 => buf.sqrtsd(0, 0),  // fsqrt  d{rd}, d{rn}
                1 => buf.roundsd(0, 0, 0x01), // frintm: round toward -inf (floor)
                2 => buf.roundsd(0, 0, 0x02), // frintp: round toward +inf (ceil)
                3 => buf.roundsd(0, 0, 0x03), // frintz: round toward zero (trunc)
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
                7 => {
                    // frinta d{rd}, d{rn}: round half-away-from-zero = sign*floor(|x|+0.5).
                    buf.movq_r64_xmm(RAX, 0);
                    buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                    buf.and_rr64(RAX, RCX);                // RAX = sign bit
                    buf.movq_xmm_r64(2, RAX);              // xmm2 = sign mask
                    buf.movq_r64_xmm(RAX, 0);
                    buf.mov_ri64(RCX, 0x7fff_ffff_ffff_ffff);
                    buf.and_rr64(RAX, RCX);                // RAX = |x|
                    buf.movq_xmm_r64(0, RAX);              // xmm0 = |x|
                    buf.mov_ri64(RAX, 0x3fe0_0000_0000_0000); // 0.5 (double)
                    buf.movq_xmm_r64(1, RAX);
                    buf.addsd(0, 1);                       // xmm0 = |x| + 0.5
                    buf.roundsd(0, 0, 0x01);               // floor(|x|+0.5)
                    buf.pxor_xmm(0, 2);                    // reapply sign bit
                }
                8 | 9 => buf.roundsd(0, 0, 0x00), // frintn/frintx d: round to nearest, ties-even
                _ => return Err(format!("FpUnary op {op} not implemented")),
            }
            buf.movq_store(RBX, vslot(rd), 0);
            Ok(())
        }
        Inst::Fabd { rd, rn, rm, sz } => {
            // fabd Vd, Dn, Dm = |dn - dm| (scalar). Compute a-b in xmm,
            // round-trip the bit pattern to a GPR, clear the sign bit, and store.
            // Honest for finite floats; NaN stays NaN (sign-bit clear keeps it a NaN).
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if sz {
                buf.movq_load(0, RBX, vslot(rn));
                buf.movq_load(1, RBX, vslot(rm));
                buf.subsd(0, 1); // xmm0 = rn - rm
                buf.movq_r64_xmm(RDX, 0); // RDX = bits(rn - rm)
                buf.mov_ri64(RDI, 0x7fff_ffff_ffff_ffff); // ~signbit
                buf.and_rr64(RDX, RDI); // clear bit 63 (|x|)
                buf.movq_xmm_r64(0, RDX); // back to xmm
                buf.movq_store(RBX, vslot(rd as u8), 0);
            } else {
                // single precision: subss + clear bit31 in the low 32 bits.
                buf.mov_load32(RAX, RBX, vslot(rn));
                buf.movd_xmm_r32(0, RAX);
                buf.mov_load32(RAX, RBX, vslot(rm));
                buf.movd_xmm_r32(1, RAX);
                buf.bytes.extend_from_slice(&[0xf3, 0x0f, 0x5c, 0xc1]); // subss xmm0,xmm1
                buf.movd_r32_xmm(RAX, 0); // RAX = bits(rn - rm)
                buf.mov_ri64(RCX, 0x7fff_ffff); // ~signbit (32-bit)
                buf.and_rr64(RAX, RCX); // clear bit 31 (|x|)
                buf.mov_store32(RBX, vslot(rd as u8), RAX);
            }
            Ok(())
        }
        Inst::FcvtToInt {
            rd,
            rn,
            mode,
            sf,
            unsigned,
            src_sng,
            fbits,
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
            if fbits > 0 {
                // Fixed-point scale: result = Fn * 2^fbits, then truncate-to-int.
                // Load 2^fbits as a double and multiply.
                let scale_bits = (1.0f64 * 2.0f64.powi(fbits as i32)).to_bits();
                buf.mov_ri64(RAX, scale_bits);
                buf.movq_xmm_r64(1, RAX);
                buf.mulsd(0, 1);
            }
            if mode == 2 {
                if unsigned {
                    // fcvtau: round to nearest (MXCSR) then cvttsd2si. x86 has no
                    // native unsigned-as-round-away; cvttsd2si is exact for [0,2^63)
                    // which covers every real W-dest conversion (matches the fcvtas
                    // ties-even-vs-away caveat). Saturation to negative for d>=2^63 is
                    // an acceptable edge for this cold path.
                    buf.roundsd(0, 0, 0x00); // roundsd nearest-even
                    buf.cvttsd2si(RAX, 0);
                } else {
                    buf.cvtsd2si(RAX, 0); // fcvtas: round to nearest (MXCSR, default even)
                }
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
                // fcvtzu: truncate toward zero to an UNSIGNED 64-bit value, valid
                // over [0, 2^64). x86 cvttsd2si is signed: exact in [0,2^63),
                // saturates to INT64_MIN (0x8000..0) for anything >= 2^63, which
                // silently corrupts the high half if used directly. Correct roads:
                //   d < 2^63          : cvttsd2si exact; negative/NaN -> 0
                //   2^63 <= d < 2^64  : 2^63 + (int64)(d - 2^63), exact
                //   d >= 2^64         : saturate to u64::MAX
                buf.mov_ri64(RCX, 0x43e0_0000_0000_0000); // 2^63 as a double
                buf.movq_xmm_r64(1, RCX);
                buf.comisd(0, 1); // CF=1 iff d < 2^63
                let jc = buf.jcc_rel32(0x82); // JB: d < 2^63 -> signed path
                // --- big path: d >= 2^63 ---
                buf.subsd(0, 1); // xmm0 = d - 2^63
                buf.cvttsd2si(RAX, 0); // RAX=(int64)(d-2^63); INT64_MIN if d>=2^64
                buf.test_rr64(RAX, RAX);
                let js = buf.jcc_rel32(0x88); // JS: d >= 2^64 -> clamp to MAX
                buf.mov_ri64(RCX, 0x8000_0000_0000_0000); // 2^63
                buf.add_rr64(RAX, RCX); // = 2^63+(d-2^63), exact in [2^63,2^64)
                let jmp_done = buf.jmp_rel32();
                let max_at = buf.len(); // clamp path: d >= 2^64
                buf.mov_ri64(RAX, u64::MAX);
                let jmp2 = buf.jmp_rel32();
                let signed_at = buf.len(); // signed path: d < 2^63
                buf.cvttsd2si(RAX, 0); // [0,2^63) exact; d<0 -> truncated negative
                buf.xor_rr64(RCX, RCX);
                buf.test_rr64(RAX, RAX);
                buf.cmov_rr64(0x48, RAX, RCX); // negative d -> 0
                let done = buf.len();
                let mut patch = |at: usize, target: usize| {
                    let disp = (target as i64 - (at as i64 + 4)) as i32;
                    buf.bytes[at..at + 4].copy_from_slice(&disp.to_le_bytes());
                };
                patch(jc, signed_at);
                patch(js, max_at);
                patch(jmp_done, done);
                patch(jmp2, done);
            } else {
                buf.cvttsd2si(RAX, 0); // fcvtzs: truncate toward zero
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::FmovGp { f, sz, half, rd, rn } => {
            // FMOV core <-> scalar FP. Double(d/x) uses the low 64, single(s/w)
            // the low 32, half (h/w) the low 16 of the vector slot.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if f {
                            // FP -> GP
                            if half {
                                buf.mov_load16(RAX, RBX, vslot(rn));
                            } else if sz {
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
                if half {
                    buf.mov_store16(RBX, vslot(rd), RAX);
                } else if sz {
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
                    Inst::FmovImm16 { rd, value_bits } => {
                        // fmov Hd,#imm: write the f16 value into the low 2B of the slot.
                        let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                        buf.mov_ri32(RAX, value_bits as u32);
                        buf.mov_store16(RBX, slot, RAX);
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
                        Inst::FMaxMin { rd, rn, rm, sz, op } => {
                            // fmax/fmin/fmaxnm/fminnm: FP max/min (x86 maxss/sd ignore NaN; ok here).
                            // op: 4=fmax,5=fmin,6=fmaxnm,7=fminnm -> max index, rn/x0 vs rm/0.
                            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            let is_max = matches!(op, 4 | 6);
                            if sz {
                                buf.movq_load(0, RBX, vslot(rn));
                                buf.movq_load(1, RBX, vslot(rm));
                                if is_max { buf.maxsd(0, 1); } else { buf.minsd(0, 1); }
                                buf.movq_store(RBX, vslot(rd), 0);
                            } else {
                                buf.mov_load32(RAX, RBX, vslot(rn));
                                buf.movd_xmm_r32(0, RAX);
                                buf.mov_load32(RAX, RBX, vslot(rm));
                                buf.movd_xmm_r32(1, RAX);
                                if is_max { buf.maxss(0, 1); } else { buf.minss(0, 1); }
                                                                buf.movd_r32_xmm(RAX, 0);
                                                                                                                        buf.mov_store32(RBX, vslot(rd), RAX);
                                                                                                                    }
                                                                                                                    Ok(())
                                                                                                                }
                                                                                                    Inst::SimdFpPair3 { rd, rn, rm, add, min, nm, q } => {
                                                                                                                                                                        // faddp/fmaxp/fminp/fmaxnmp/fminnmp Vd.T,Vn.T,Vm.T:
                                                                                                                                                                        // halves of Vd = pairwise-reduce Vn (low half) then Vm.
                                                                                                                                                                        // Q=0 (.2s): 2 src lanes -> 1 dst lane each.
                                                                                                                                                                        // Q=1 (.4s): 4 src lanes -> 2 dst lanes each.
                                                                                                                                                                        let _ = nm;
                                                                                                                                                                        let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                        let nsrc = if q { 4 } else { 2 };
                                                                                                                                                                        let mut dst_idx = 0;
                                                                                                                                                                        for perreg in 0..2 {
                                                                                                                                                                            let base = if perreg == 0 { vslot(rn) } else { vslot(rm) };
                                                                                                                                                                            for i in 0..nsrc / 2 {
                                                                                                                                                                                buf.mov_load32(RAX, RBX, base + 4 * (2 * i));
                                                                                                                                                                                buf.movd_xmm_r32(0, RAX);
                                                                                                                                                                                buf.mov_load32(RAX, RBX, base + 4 * (2 * i + 1));
                                                                                                                                                                                buf.movd_xmm_r32(1, RAX);
                                                                                                                                                                                if add { buf.addss(0, 1); }
                                                                                                                                                                                else if min { buf.minss(0, 1); } else { buf.maxss(0, 1); }
                                                                                                                                                                                buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                                                buf.mov_store32(RBX, vslot(rd) + 4 * dst_idx, RAX);
                                                                                                                                                                                dst_idx += 1;
                                                                                                                                                                            }
                                                                                                                                                                        }
                                                                                                                                                                        Ok(())
                                                                                                                                                                    }
                                                                                                                                                                        Inst::FpPair { rd, rn, sz, min, nm } => {
                                                                                                                                                                        // fmaxp/fminp/fmaxnmp/fminnmp Vd, Vn:
                                                                                                                                                                        // pairwise reduce Vn's two elements (.2s or
                                                                                                                                                                        // .2d) into a scalar result in Vd. x86
                                                                                                                                                                        // maxss/sd ignore NaN (ok for fmaxp/nm here).
                                                                                                                                                                        let _ = nm; // skip-NaN; maxss/sd ~= fmaxnm path
                                                                                                                                                                        let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                        if sz {
                                                                                                                                                                            // Vd.D = max/min(Vn.D[0], Vn.D[1])
                                                                                                                                                                            buf.movq_load(0, RBX, vslot(rn));
                                                                                                                                                                            buf.movq_load(1, RBX, vslot(rn) + 8);
                                                                                                                                                                            if min { buf.minsd(0, 1); } else { buf.maxsd(0, 1); }
                                                                                                                                                                            buf.movq_store(RBX, vslot(rd), 0);
                                                                                                                                                                        } else {
                                                                                                                                                                            // Vd.S = max/min(Vn.S[0], Vn.S[1])
                                                                                                                                                                            buf.mov_load32(RAX, RBX, vslot(rn));
                                                                                                                                                                            buf.movd_xmm_r32(0, RAX);
                                                                                                                                                                            buf.mov_load32(RAX, RBX, vslot(rn) + 4);
                                                                                                                                                                            buf.movd_xmm_r32(1, RAX);
                                                                                                                                                                            if min { buf.minss(0, 1); } else { buf.maxss(0, 1); }
                                                                                                                                                                            buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                                            buf.mov_store32(RBX, vslot(rd), RAX);
                                                                                                                                                                        }
                                                                                                                                                                        Ok(())
                                                                                                                                                                    }
                                                                                                        Inst::FMaxV { rd, rn, min } => {
                                                                                                        // fmaxv/fminv Sd, Vn.4s: reduce the 4
                                                                                                        // single-precision lanes of Vn to a max/
                                                                                                        // min into scalar Sd (low 32-bit of V[rd]).
                                                                                                        let base = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                                                                                        buf.mov_load32(RAX, RBX, base);
                                                                                                        buf.movd_xmm_r32(0, RAX);
                                                                                                        for i in 1..4 {
                                                                                                            buf.mov_load32(RAX, RBX, base + 4 * i);
                                                                                                            buf.movd_xmm_r32(1, RAX);
                                                                                                            if min { buf.minss(0, 1); } else { buf.maxss(0, 1); }
                                                                                                        }
                                                                                                        buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                let dbase = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                                                                                                                                buf.mov_store32(RBX, dbase, RAX);
                                                                                                                                                Ok(())
                                                                                                                                            }
                                                                                                                                            Inst::Fmla { rd, rn, rm, el64, q, sub } => {
                                                                                                                                                // fmla/fmls Vd.T, Vn, Vm: per-lane Vd = Vd +/- Vn*Vm.
                                                                                                                                                // el64 => .2d (2 double lanes); else q => .4s (4 single);
                                                                                                                                                // else => .2s (2 single).
                                                                                                                                                let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                                                                                                                                let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                                                                                                                                let mb = crate::jit::VECTOR_BASE + (rm as i32) * 16;
                                                                                                                                                let lanes = if el64 { 2 } else if q { 4 } else { 2 };
                                                                                                                                                for l in 0..lanes {
                                                                                                                                                                    if el64 {
                                                                                                                                                                        // xmm0=Vn[l]; xmm1=Vm[l]; xmm1=Vn*Vm; xmm0=Vd[l];
                                                                                                                                                                        // Vd +- Vn*Vm via addss/subss(0,1) => correct sign for
                                                                                                                                                                        // fmls (Vd - Vn*Vm), NOT Vn*Vm - Vd.
                                                                                                                                                                        buf.movq_load(0, RBX, nb + 8 * l); // Vn[l] dbl
                                                                                                                                                                        buf.movq_load(1, RBX, mb + 8 * l); // Vm[l] dbl
                                                                                                                                                                        buf.mulsd(1, 0);                   // xmm1 = Vn*Vm
                                                                                                                                                                        buf.movq_load(0, RBX, db + 8 * l); // Vd[l]
                                                                                                                                                                        if sub { buf.subsd(0, 1); } else { buf.addsd(0, 1); }
                                                                                                                                                                        buf.movq_store(RBX, db + 8 * l, 0); // Vd[l] = result
                                                                                                                                                                    } else {
                                                                                                                                                                        // Same ordering for the .4s/.2s single-precision path:
                                                                                                                                                                        // product in xmm1, accumulator Vd in xmm0, so the
                                                                                                                                                                        // subtract has the correct operand order.
                                                                                                                                                                        buf.mov_load32(RAX, RBX, nb + 4 * l);
                                                                                                                                                                        buf.movd_xmm_r32(0, RAX);           // xmm0 = Vn[l]
                                                                                                                                                                        buf.mov_load32(RAX, RBX, mb + 4 * l);
                                                                                                                                                                        buf.movd_xmm_r32(1, RAX);           // xmm1 = Vm[l]
                                                                                                                                                                        buf.mulss(1, 0);                    // xmm1 = Vn*Vm
                                                                                                                                                                        buf.mov_load32(RAX, RBX, db + 4 * l);
                                                                                                                                                                        buf.movd_xmm_r32(0, RAX);           // xmm0 = Vd[l]
                                                                                                                                                                        if sub { buf.subss(0, 1); } else { buf.addss(0, 1); }
                                                                                                                                                                        buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                                        buf.mov_store32(RBX, db + 4 * l, RAX);
                                                                                                                                                                    }
                                                                                                                                                                }
                                                                                                                                                Ok(())
                                                                                                                                            }
                                                                                                                                            Inst::FmlaEl { rd, rn, vlm, idx, el64, q, sub } => {
                                                                                                                                                            // fmla/fmls Vd.T, Vn, Vm.el[idx]: Vd[j] += Vn[j]*Vm.el(idx), per lane.
                                                                                                                                                            let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                                                                                                                                            let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                                                                                                                                            let mb = crate::jit::VECTOR_BASE + (vlm as i32) * 16;
                                                                                                                                                            let lanes = if el64 { 2 } else if q { 4 } else { 2 };
                                                                                                                                                            // broadcast scalar element into xmm2
                                                                                                                                                            if el64 {
                                                                                                                                                                buf.movq_load(2, RBX, mb + (idx as i32) * 8);
                                                                                                                                                            } else {
                                                                                                                                                                buf.mov_load32(RAX, RBX, mb + (idx as i32) * 4);
                                                                                                                                                                buf.movd_xmm_r32(2, RAX);
                                                                                                                                                            }
                                                                                                                                                            for l in 0..lanes {
                                                                                                                                                                if el64 {
                                                                                                                                                                    buf.movq_load(0, RBX, db + 8 * l);
                                                                                                                                                                    buf.movq_load(1, RBX, nb + 8 * l);
                                                                                                                                                                    buf.mulsd(1, 2);                // xmm1 = Vn[l]*Vm.el
                                                                                                                                                                    if sub { buf.subsd(0, 1); } else { buf.addsd(0, 1); }
                                                                                                                                                                    buf.movq_store(RBX, db + 8 * l, 0);
                                                                                                                                                                } else {
                                                                                                                                                                    buf.mov_load32(RAX, RBX, db + 4 * l);
                                                                                                                                                                    buf.movd_xmm_r32(0, RAX);
                                                                                                                                                                    buf.mov_load32(RAX, RBX, nb + 4 * l);
                                                                                                                                                                    buf.movd_xmm_r32(1, RAX);
                                                                                                                                                                    buf.mulss(1, 2);
                                                                                                                                                                    if sub { buf.subss(0, 1); } else { buf.addss(0, 1); }
                                                                                                                                                                    buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                                                                        buf.mov_store32(RBX, db + 4 * l, RAX);
                                                                                                                                                                                                    }
                                                                                                                                                                                                }
                                                                                                                                                                                                Ok(())
                                                                                                                                                                                            }
                                                                                                                                                                                            Inst::VecFpArith { rd, rn, rm, op, q } => {
                                                                                                                                                                                                // Vector single-precision FP two-source arithmetic:
                                                                                                                                                                                                // Vd[l] = f(Vn[l], Vm[l]) over .4s (q) or .2s lanes.
                                                                                                                                                                                                // op 0=add 1=sub 2=mul 3=div 4=max 5=min 6=fmaxnm
                                                                                                                                                                                                // 7=fminnm. (The NaN-propagation edge of maxnm/minnm
                                                                                                                                                                                                // differs from x86 maxss/minss only when one operand
                                                                                                                                                                                                // is NaN; a documented approximation for graphics.)
                                                                                                                                                                                                let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                                                                                                                                                                                let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                                                                                                                                                                                let mb = crate::jit::VECTOR_BASE + (rm as i32) * 16;
                                                                                                                                                                                                let lanes = if q { 4 } else { 2 };
                                                                                                                                                                                                for l in 0..lanes {
                                                                                                                                                                                                    buf.mov_load32(RAX, RBX, nb + 4 * l);
                                                                                                                                                                                                    buf.movd_xmm_r32(0, RAX);
                                                                                                                                                                                                    buf.mov_load32(RAX, RBX, mb + 4 * l);
                                                                                                                                                                                                    buf.movd_xmm_r32(1, RAX);
                                                                                                                                                                                                    match op {
                                                                                                                                                                                                        0 => buf.addss(0, 1),
                                                                                                                                                                                                        1 => buf.subss(0, 1),
                                                                                                                                                                                                        2 => buf.mulss(0, 1),
                                                                                                                                                                                                        3 => buf.divss(0, 1),
                                                                                                                                                                                                        4 | 6 => buf.maxss(0, 1),
                                                                                                                                                                                                        8 | 9 => {
                                                                                                                                                                                                            // frecps = 2 - Vn*Vm ; frsqrts = (3 - Vn*Vm)/2
                                                                                                                                                                                                            buf.mulss(0, 1); // xmm0 = Vn*Vm
                                                                                                                                                                                                            buf.mov_ri32(RAX, 0x4000_0000u32); // 2.0f32
                                                                                                                                                                                                            buf.movd_xmm_r32(1, RAX); // 2.0
                                                                                                                                                                                                            buf.subss(1, 0);   // xmm1 = 2 - prod
                                                                                                                                                                                                            if op == 9 {
                                                                                                                                                                                                                // /2: multiply by 0.5
                                                                                                                                                                                                                buf.mov_ri32(RAX, 0x3f00_0000u32); // 0.5
                                                                                                                                                                                                                buf.movd_xmm_r32(3, RAX);
                                                                                                                                                                                                                buf.mulss(1, 3);
                                                                                                                                                                                                            }
                                                                                                                                                                                                            buf.movd_r32_xmm(RAX, 1);
                                                                                                                                                                                                            buf.mov_store32(RBX, db + 4 * l, RAX);
                                                                                                                                                                                                            continue;
                                                                                                                                                                                                        }
                                                                                                                                                                                                        _ => buf.minss(0, 1), // 5 / 7
                                                                                                                                                                                                    }
                                                                                                                                                                                                    buf.movd_r32_xmm(RAX, 0);
                                                                                                                                                                                                    buf.mov_store32(RBX, db + 4 * l, RAX);
                                                                                                                                                                                                }
                                                                                                                                                                                                Ok(())
                                                                                                                                                                                            }
                                                                                                                                                                                            Inst::VecFpCmp { rd, rn, rm, esize, op, q } => {
            // fcmeq/fcmgt/fcmge Vd.T, Vn.T, Vm.T: per-lane result = all-ones
            // if Vn op Vm, else 0 (the mask gcc ANDs/sub-adds to count lanes).
            // Compare via comiss/comisd (CF=1 if Vn<Vm; ZF=1 if equal), then
            // setcc AL, movzx, neg -> all-ones or 0 in the low 32/64 bits.
            let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let mb = crate::jit::VECTOR_BASE + (rm as i32) * 16;
            let es = esize as i32;
            let lanes = if q { 16 / es } else { 8 / es };
            let cc = match op {
                0 => 4u8,  // fcmeq  : sete (ZF)
                1 => 7u8,  // fcmgt  : seta (CF=0 && ZF=0)
                _ => 3u8,  // fcmge  : setae (CF=0)
            };
            for l in 0..lanes {
                let off = l * es;
                if esize == 8 {
                    buf.movq_load(0, RBX, nb + off);
                    buf.movq_load(1, RBX, mb + off);
                    buf.comisd(0, 1);
                } else {
                    buf.mov_load32(RAX, RBX, nb + off);
                    buf.movd_xmm_r32(0, RAX);
                    buf.mov_load32(RAX, RBX, mb + off);
                    buf.movd_xmm_r32(1, RAX);
                    buf.comiss(0, 1);
                }
                buf.setcc_rm8(cc, 0); // AL = 0/1
                buf.movzx_r32_r8(RAX, RAX);
                buf.neg_r64(RAX); // low 32/64: 0 -> 0, +1 -> all-ones
                if esize == 8 {
                    buf.mov_store64(RBX, db + off, RAX);
                } else {
                    buf.mov_store32(RBX, db + off, RAX);
                }
            }
            Ok(())
        }
        Inst::VecFpCmpZero { rd, rn, op, esize, q } => {
            // fcmeq/fcmgt/fcmge/fcmlt/fcmle Vd.T, Vn.T, #0.0: per-lane = all-ones
            // if Vn op 0.0 else 0 (mirrors VecFpCmp with a zeroed second operand).
            // comiss/comisd(x, 0): CF=1 if x<0, ZF=1 if x==0; unordered(NaN) sets
            // all three, so seta/setae/sete/setb/setbe match the existing VecFpCmp
            // NaN precedent (fcmgt/ge safe; eq/lt/le NaN-imperfect as there).
            let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let es = esize as i32;
            let lanes = if q { 16 / es } else { 8 / es };
            let cc = match op {
                0 => 4u8, // fcmeq : sete (ZF)
                1 => 7u8, // fcmgt : seta (CF=0 && ZF=0)
                2 => 3u8, // fcmge : setae (CF=0)
                3 => 2u8, // fcmlt : setb (CF=1)
                _ => 6u8, // fcmle : setbe (CF=1 || ZF=1)
            };
            for l in 0..lanes {
                let off = l * es;
                buf.pxor_xmm(1, 1); // xmm1 = +0.0
                if esize == 8 {
                    buf.movq_load(0, RBX, nb + off);
                    buf.comisd(0, 1);
                } else {
                    buf.mov_load32(RAX, RBX, nb + off);
                    buf.movd_xmm_r32(0, RAX);
                    buf.comiss(0, 1);
                }
                buf.setcc_rm8(cc, 0); // AL = 0/1
                buf.movzx_r32_r8(RAX, RAX);
                buf.neg_r64(RAX); // low 32/64: 0 -> 0, +1 -> all-ones
                if esize == 8 {
                    buf.mov_store64(RBX, db + off, RAX);
                } else {
                                    buf.mov_store32(RBX, db + off, RAX);
                                }
                            }
                            Ok(())
                        }
                        Inst::WidenShl { rd, rn, dst_esize, nlanes, signed, upper } => {
                                                 // shll/s hll2 vd.Td, vn.Ts: widen nlanes low (upper half) elements
                                // of vn, sign/zero extend to dst_esize-byte lanes.
                                let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                let src_esize = (dst_esize / 2) as i32;
                                let uh = if upper { 8 } else { 0 }; // shll2 reads the upper 8 bytes
                                for rev in 0..(nlanes as i32) {
                                    let i = (nlanes as i32) - 1 - rev;
                                    let so = nb + uh + i * src_esize;
                                    let doff = db + i * (dst_esize as i32);
                                    match dst_esize {
                                        8 => {
                                            buf.mov_load32(RCX, RBX, so);
                                            if signed {
                                                buf.movsxd_r64_r32(RAX, RCX);
                                            } else {
                                                buf.mov_rr64(RAX, RCX);
                                            }
                                            buf.mov_store64(RBX, doff, RAX);
                                        }
                                        4 => {
                                            if signed {
                                                buf.movsx_word_mem(RAX, RBX, so);
                                            } else {
                                                buf.movzx_word_mem(RAX, RBX, so);
                                            }
                                            buf.mov_store32(RBX, doff, RAX);
                                        }
                                        _ => {
                                            if signed {
                                                buf.movsx_byte_mem(RAX, RBX, so);
                                            } else {
                                                buf.movzx_byte_mem(RAX, RBX, so);
                                            }
                                            buf.mov_store16(RBX, doff, RAX);
                                        }
                                    }
                                }
                                Ok(())
                            }
                            Inst::SimdAdalp { rd, rn, src_esize, n_pairs, signed, upper, acc } => {
                                // sadalp/uadalp (acc=true) or saddlp/uaddlp (acc=false,
                                // pairwise-add-long) Vd.Td, Vn.Ts: for each adjacent pair
                                // (2i,2i+1) of src_esize-byte srcs, write (acc=false) or add
                                // into (acc=true) the dst lane of width 2*src_esize.
                                let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16 + if upper { 8 } else { 0 };
                                let se = src_esize as i32;
                                let np = n_pairs as i32;
                                let de = 2 * se;
                                for i in 0..np {
                                    let ss = nb + 2 * i * se;
                                    let dd = db + i * de;
                                    // RAX = widened(Vn[2i]); RCX = widened(Vn[2i+1])
                                    if se == 4 { buf.mov_load32(RAX, RBX, ss); buf.mov_load32(RCX, RBX, ss + 4); }
                                    else if se == 2 {
                                        if signed { buf.movsx_word_mem(RAX, RBX, ss); buf.movsx_word_mem(RCX, RBX, ss + 2); }
                                        else { buf.movzx_word_mem(RAX, RBX, ss); buf.movzx_word_mem(RCX, RBX, ss + 2); }
                                    } else {
                                        if signed { buf.movsx_byte_mem(RAX, RBX, ss); buf.movsx_byte_mem(RCX, RBX, ss + 1); }
                                        else { buf.movzx_byte_mem(RAX, RBX, ss); buf.movzx_byte_mem(RCX, RBX, ss + 1); }
                                    }
                                    buf.add_rr64(RAX, RCX);
                                    // Write EXACTLY `de` (dst lane) bytes — never overrun the
                                    // neighbouring lane (store64 on a 4-byte lane clobbers lane+1).
                                    if acc {
                                        if de >= 4 { buf.mov_load64(R10, RBX, dd); buf.add_rr64(RAX, R10); }
                                        else { buf.mov_load32(R10, RBX, dd); buf.add_rr64(RAX, R10); }
                                        if de == 8 { buf.mov_store64(RBX, dd, RAX); }
                                        else if de == 4 { buf.mov_store32(RBX, dd, RAX); }
                                        else { buf.mov_store16(RBX, dd, RAX); }
                                    } else {
                                        if de == 8 { buf.mov_store64(RBX, dd, RAX); }
                                        else if de == 4 { buf.mov_store32(RBX, dd, RAX); }
                                        else { buf.mov_store16(RBX, dd, RAX); }
                                    }
                                }
                                Ok(())
                            }
                            Inst::SimdAddl { rd, rn, rm, esrc, sign, sub, upper } => {
                                // saddl/uaddl/subl/usubl Vd.T, Vn.T, Vm.T: widen each esrc-byte
                                // element of Vn and Vm (low or upper half) to 2*esrc and add/sub.
                                // Each source element is read EXACTLY esrc bytes and widened to the
                                // 64-bit reg (sign- or zero-extended), so a later lane's high bytes
                                // never pollute the sum; the widened result is stored EXACTLY de bytes
                                // (de = 2*esrc) so it never overruns into the neighbouring lane.
                                // IN-PLACE ALIASING: the widened (2*esrc) dest write
                                // at i*2*esrc overlaps the narrow source bytes of
                                // lane i+1 (at (i+1)*esrc), so when rd aliases rn or
                                // rm a read-then-write loop clobbers the still-needed
                                // source -- e.g. gcc's `uaddl v0.4s, v0.4h, v1.4h`
                                // (rd==rn) turned lane 1's sum into garbage. Snapshot
                                // each source that aliases rd to permscratch first.
                                let vbase = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                let db = vbase(rd);
                                let nb = if rn == rd { permute_source(buf, rd, rn, false) } else { vbase(rn) };
                                let mb = if rm == rd { permute_source(buf, rd, rm, true) } else { vbase(rm) };
                                let se = esrc as i32;
                                let de = 2 * se;
                                let lanes: i32 = 8 / se; // 8-source bytes -> 4 + or (16/2...) 8/se
                                let uh = if upper { 8 } else { 0 };
                                let ld = |b: &mut crate::x86::CodeBuf, reg: u8, at: i32| match se {
                                    4 => {
                                        b.mov_load32(reg, RBX, at);
                                        if sign { b.movsxd_r64_r32(reg, reg); }
                                    }
                                    2 => {
                                        if sign { b.movsx_word_mem(reg, RBX, at); }
                                        else { b.movzx_word_mem(reg, RBX, at); }
                                    }
                                    _ => {
                                        if sign { b.movsx_byte_mem(reg, RBX, at); }
                                        else { b.movzx_byte_mem(reg, RBX, at); }
                                    }
                                };
                                for i in 0..lanes {
                                    let so_n = nb + uh + i * se;
                                    let so_m = mb + uh + i * se;
                                    let dd = db + i * de;
                                    ld(buf, RAX, so_n);
                                    ld(buf, RCX, so_m);
                                    if sub { buf.sub_rr64(RAX, RCX); } else { buf.add_rr64(RAX, RCX); }
                                    match de {
                                        8 => buf.mov_store64(RBX, dd, RAX),
                                        4 => buf.mov_store32(RBX, dd, RAX),
                                        _ => buf.mov_store16(RBX, dd, RAX),
                                    }
                                }
                                Ok(())
                            }
                            Inst::SimdSatAdd { rd, rn, rm, esize, sub, unsigned, q } => {
                                // sqadd/uqadd/sqsub/uqsub: per-lane saturating add/sub.
                                let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                let se = esize as i32;
                                let lanes: i32 = if q { 16 / se } else { 8 / se };
                                let full: u64 = if se == 8 { u64::MAX } else { (1u64 << (se * 8)) - 1 };
                                // Signed bounds as 64-bit SIGN-EXTENDED constants.
                                // smax is the positive max (0x7F.. its low bits);
                                // smin must be the NEGATIVE bound so the 64-bit
                                // compare sees a sign-extended result as < it
                                // (for .4s/.8h/.16b the raw 0x80000000 bit-pattern
                                // as u64 is a positive value, so RAX = 15 would be
                                // "less" than 0x80000000 and wrongly clamp).
                                let smax: u64 = if se == 8 { i64::MAX as u64 } else { ((1i64 << (se * 8 - 1)) - 1) as u64 };
                                let smin: u64 = if se == 8 { i64::MIN as u64 } else { (-(1i64 << (se * 8 - 1))) as u64 };
                                for i in 0..lanes {
                                    let so_n = vslot(rn) + i * se;
                                    let so_m = vslot(rm) + i * se;
                                    let dd = vslot(rd) + i * se;
                                    // Load ONE lane, SIGN-extended for the signed
                                    // path so the 64-bit clamps below see negative
                                    // results correctly (a zero-extended 0x80 lane
                                    // would read as +128, never clamp to smin).
                                    // Unsigned lanes load zero-extended as before.
                                    if unsigned {
                                        match se {
                                            8 => { buf.mov_load64(RAX, RBX, so_n); buf.mov_load64(RCX, RBX, so_m); }
                                            4 => { buf.mov_load32(RAX, RBX, so_n); buf.mov_load32(RCX, RBX, so_m); }
                                            2 => { buf.movzx_word_mem(RAX, RBX, so_n); buf.movzx_word_mem(RCX, RBX, so_m); }
                                            _ => { buf.movzx_byte_mem(RAX, RBX, so_n); buf.movzx_byte_mem(RCX, RBX, so_m); }
                                        }
                                    } else {
                                        match se {
                                            1 => {
                                                buf.movsx_byte_mem(RAX, RBX, so_n);
                                                buf.movsx_byte_mem(RCX, RBX, so_m);
                                            }
                                            2 => {
                                                buf.movsx_word_mem(RAX, RBX, so_n);
                                                buf.movsx_word_mem(RCX, RBX, so_m);
                                            }
                                            4 => {
                                                buf.mov_load32(RAX, RBX, so_n);
                                                buf.movsxd_r64_r32(RAX, RAX);
                                                buf.mov_load32(RCX, RBX, so_m);
                                                buf.movsxd_r64_r32(RCX, RCX);
                                            }
                                            _ => {
                                                buf.mov_load64(RAX, RBX, so_n);
                                                buf.mov_load64(RCX, RBX, so_m);
                                            }
                                        }
                                    }
                                    if unsigned {
                                        if sub {
                                            // uqsub: diff = Vn - Vm; clamp 0 on borrow
                                            buf.mov_ri64(R10, 0);
                                            buf.cmp_rr64(RCX, RAX);   // CF=1 if Vm>Vn
                                            buf.sub_rr64(RAX, RCX);
                                            buf.cmov_rr64(0x42, RAX, R10); // cmovb -> 0 if underflow
                                        } else {
                                            // uqadd: sum; clamp to full on carry
                                            buf.mov_ri64(R10, full);
                                            buf.add_rr64(RAX, RCX);
                                            buf.cmov_rr64(0x42, RAX, R10); // cmovb (carry) -> full
                                        }
                                    } else {
                                        if sub { buf.sub_rr64(RAX, RCX); } else { buf.add_rr64(RAX, RCX); }
                                        buf.mov_ri64(R10, smax as u64);
                                        buf.cmp_rr64(RAX, R10);
                                        buf.cmov_rr64(0x4f, RAX, R10);   // cmovg -> smax
                                        buf.mov_ri64(R10, smin as u64);
                                        buf.cmp_rr64(RAX, R10);
                                        buf.cmov_rr64(0x4c, RAX, R10);   // cmovl -> smin
                                    }
                                    // Store a single lane back (width matches lane).
                                    match se {
                                        8 => buf.mov_store64(RBX, dd, RAX),
                                        4 => buf.mov_store32(RBX, dd, RAX),
                                        2 => buf.mov_store16(RBX, dd, RAX),
                                        _ => buf.mov_store8(RBX, dd, RAX),
                                    }
                                }
                            Ok(())
                            }
                            Inst::FmulScalar { rd, rn, rm, double, neg } => {
                                            // fmul/fnmul Sd/Dd, Sn, Sm: rd = (+/-)(rn*rm) scalar.
                                            let dn = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                            let nn = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                            let mn = crate::jit::VECTOR_BASE + (rm as i32) * 16;
                                            if double {
                                                buf.movq_load(0, RBX, nn);
                                                buf.movq_load(1, RBX, mn);
                                                buf.mulsd(0, 1);
                                                if neg {
                                                    buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                                                    buf.movq_xmm_r64(1, RCX);
                                                    buf.pxor_xmm(0, 1);
                                                }
                                                buf.movq_store(RBX, dn, 0);
                                            } else {
                                                buf.mov_load32(RCX, RBX, nn);
                                                buf.movd_xmm_r32(0, RCX);
                                                buf.mov_load32(RCX, RBX, mn);
                                                buf.movd_xmm_r32(1, RCX);
                                                buf.mulss(0, 1);
                                                if neg {
                                                    buf.mov_ri64(RCX, 0x8000_0000);
                                                    buf.movd_xmm_r32(1, RCX);
                                                    buf.pxor_xmm(0, 1);
                                                }
                                                buf.movd_r32_xmm(RCX, 0);
                                                buf.mov_store32(RBX, dn, RCX);
                                            }
                                            Ok(())
                                        }
                                        Inst::SminMax { rd, rn, rm, max, unsigned, esize, q } => {
                                            // smin/smax/umin/umax Vd.T, Vn, Vm: per-lane min/max.
                                            let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
                                            let mb = crate::jit::VECTOR_BASE + (rm as i32) * 16;
                                            let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                            let lanes: i32 = if esize == 4 { if q { 4 } else { 2 } } else { if q { 16 } else { 8 } };
                                                                                        for i in 0..lanes {
                                                                                            let e = esize as i32;
                                                                                            let so = nb + i * e;
                                                                                            let mo = mb + i * e;
                                                                                            // load A=Vn[i], B=Vm[i], sign/zero-extended
                                                                                            if e == 4 {
                                                                                                buf.mov_load32(RAX, RBX, so);
                                                                                                if !unsigned { buf.movsxd_r64_r32(RAX, RAX); }
                                                                                                buf.mov_load32(RCX, RBX, mo);
                                                                                                if !unsigned { buf.movsxd_r64_r32(RCX, RCX); }
                                                                                            } else {
                                                                                                if unsigned {
                                                                                                    buf.movzx_byte_mem(RAX, RBX, so);
                                                                                                    buf.movzx_byte_mem(RCX, RBX, mo);
                                                                                                } else {
                                                                                                    buf.movsx_byte_mem(RAX, RBX, so);
                                                                                                    buf.movsx_byte_mem(RCX, RBX, mo);
                                                                                                }
                                                                                            }
                                                                                            buf.cmp_rr64(RCX, RAX); // flags = B - A
                                                                                            if max {
                                                                                                if unsigned { buf.cmov_rr64(0x47, RAX, RCX); } // cmova
                                                                                                else { buf.cmov_rr64(0x4F, RAX, RCX); } // cmovg
                                                                                            } else {
                                                                                                if unsigned { buf.cmov_rr64(0x42, RAX, RCX); } // cmovb
                                                                                                else { buf.cmov_rr64(0x4C, RAX, RCX); } // cmovl
                                                                                            }
                                                                                            if e == 4 { buf.mov_store32(RBX, db + i * e, RAX); }
                                                                                            else { buf.mov_store8(RBX, db + i * e, RAX); }
                                                                                        }
                                                                                        Ok(())
                                                                                    }
                                                                                    Inst::ScvtfFixed { rd, rn, to_double, sf, unsigned, fbits } => {
                                            // ucvtf/scvtf Dd,Rn,#fbits: convert int to float, then /2^fbits.
                                            let vslot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                                            ldg(buf, RAX, rn as u32);
                                            if unsigned && sf {
                                                // 64-bit unsigned: cvtsi2sd + the +2^64 correction, then scale.
                                                ldg(buf, RDX, rn as u32);
                                                buf.cvtsi2sd(0, true, RDX);
                                                buf.test_rr64(RDX, RDX);
                                                let jns = buf.jcc_rel32(0x89); // JNS: skip if u < 2^63
                                                buf.mov_ri64(RCX, 0x43f0_0000_0000_0000);
                                                buf.movq_xmm_r64(1, RCX);
                                                buf.addsd(0, 1);
                                                let end = buf.len();
                                                let disp = (end as i64 - (jns as i64 + 4)) as i32;
                                                buf.bytes[jns..jns + 4].copy_from_slice(&disp.to_le_bytes());
                                            } else {
                                                buf.cvtsi2sd(0, sf, RAX);
                                            }
                                            // divide by 2^fbits (normal-power double in xmm1)
                                            buf.mov_ri64(RCX, (((1023 + fbits as u64) << 52)));
                                            buf.movq_xmm_r64(1, RCX);
                                            buf.divsd(0, 1);
                                            if to_double {
                                                buf.movq_store(RBX, vslot, 0);
                                            } else {
                                                buf.cvtsd2ss(0, 0);
                                                buf.movd_r32_xmm(RAX, 0);
                                                buf.mov_store32(RBX, vslot, RAX);
                                            }
                                            Ok(())
                                        }
        Inst::Fcmp { rn, rm, against_zero, sz, half } => {
            // fcmp d{rn}, d{rm} / fcmp d{rn}, #0.0 / fcmp s{rn}, s{rm}:
            // compare and set guest NZCV. Use comisd/comiss (CF=1 if a<b,
            // ZF=1/PF=1 if unordered); store_nzcv_fp maps to AArch64 NZCV.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if sz {
                buf.movq_load(0, RBX, vslot(rn));
                if against_zero {
                    buf.pxor_xmm(1, 1); // xmm1 = +0.0
                } else {
                    buf.movq_load(1, RBX, vslot(rm));
                }
            } else {
                // half: load the low f16 half, promote to f32 (F16C), comiss.
                if half {
                    buf.mov_load16(RAX, RBX, vslot(rn));
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                    if against_zero {
                        buf.pxor_xmm(1, 1); // xmm1 = +0.0f
                    } else {
                        buf.mov_load16(RAX, RBX, vslot(rm));
                        buf.movd_xmm_r32(1, RAX);
                        buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc9]); // vcvtph2ps xmm1,xmm1
                    }
                } else {
                    buf.mov_load32(RAX, RBX, vslot(rn));
                    buf.movd_xmm_r32(0, RAX);
                    if against_zero {
                        buf.pxor_xmm(1, 1); // xmm1 = +0.0f
                    } else {
                        buf.mov_load32(RAX, RBX, vslot(rm));
                        buf.movd_xmm_r32(1, RAX);
                    }
                }
            }
            if sz {
                buf.comisd(0, 1);
            } else {
                buf.comiss(0, 1); // works for both promoted-half and single
            }
            store_nzcv_fp(buf);
            Ok(())
        }
        Inst::Fccmp { rn, rm, nzcv, cond, sz } => {
            // fccmp Dn, Dm, #nzcv, <cond>: if cond(guest NZCV) do FP compare -> NZCV;
            // else NZCV = nzcv. Load guest flags, jcc to the compare path when cond
            // true, else write the immediate nzcv, then patch both rel32 wires.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let jcc = x86_cc_for_cond(cond)
                .ok_or_else(|| format!("Fccmp: bad cond {cond:#x}"))?;
            load_nzcv_to_eflags(buf);              // eflags = guest NZCV (cond)
            let j_cond = buf.jcc_rel32(jcc);       // jump to fp-compare when cond TRUE
            // cond FALSE: NZCV = nzcv immediate
            buf.mov_ri64(RAX, (nzcv as u64) << 28);
            buf.mov_store32(RBX, NZCV_OFF, RAX);
            let j_end = buf.jmp_rel32();           // skip over the fp-compare path
            let fp_off = buf.len() as i32;         // start of cond-TRUE path
            if sz {
                buf.movq_load(0, RBX, vslot(rn));
                buf.movq_load(1, RBX, vslot(rm));
                buf.comisd(0, 1);
            } else {
                buf.mov_load32(RAX, RBX, vslot(rn));
                buf.movd_xmm_r32(0, RAX);
                buf.mov_load32(RAX, RBX, vslot(rm));
                buf.movd_xmm_r32(1, RAX);
                buf.comiss(0, 1);
            }
            store_nzcv_fp(buf);
            let tail_off = buf.len() as i32;
            // patch displacements: relative to disp_off+4
            let jd = j_cond as i32;
            let d1 = ((fp_off - (jd + 4)) as u32).to_le_bytes();
            buf.bytes[jd as usize..(jd + 4) as usize].copy_from_slice(&d1);
            let je = j_end as i32;
            let d2 = ((tail_off - (je + 4)) as u32).to_le_bytes();
            buf.bytes[je as usize..(je + 4) as usize].copy_from_slice(&d2);
            Ok(())
        }
        Inst::CcMp { rn, rm, imm, nzcv, cond, cmn, sf, is_reg } => {
            // ccmp/ccmn Rn, <Rm|#imm>, #nzcv, <cond>: if cond(guest NZCV) holds,
            // the flags become `Rn op <Rm|imm>` (ccmn=add, ccmp=subtract); else
            // they become the 4-bit `nzcv` immediate. Flag-only, no writeback —
            // same branch-then-patch structure as Fccmp above.
            let jcc = x86_cc_for_cond(cond)
                .ok_or_else(|| format!("CcMp: bad cond {cond:#x}"))?;
            load_nzcv_to_eflags(buf);              // eflags = guest NZCV (cond eval)
            let j_cond = buf.jcc_rel32(jcc);       // jump to the compare path when cond TRUE
            // cond FALSE: NZCV = the nzcv immediate (N[3]Z[2]C[1]V[0] -> bits 31/30/29/28)
            buf.mov_ri64(RAX, (nzcv as u64) << 28);
            buf.mov_store32(RBX, NZCV_OFF, RAX);
            let j_end = buf.jmp_rel32();           // skip the compare path
            let cmp_off = buf.len() as i32;        // start of cond-TRUE path
            // cond TRUE: compute Rn op <Rm|imm> and set flags from the result.
            if rn == 31 {
                buf.mov_ri64(RAX, 0);              // Rn = XZR (0)
            } else {
                ldg(buf, RAX, rn as u32);
            }
            if !sf {
                zext_w(buf, RAX); // 32-bit compare: zero high garbage
            }
            if is_reg {
                if rm == 31 {
                    buf.mov_ri64(RCX, 0);
                } else {
                    ldg(buf, RCX, rm as u32);
                }
                if !sf {
                    zext_w(buf, RCX);
                }
            } else {
                buf.mov_ri64(RCX, imm as u64);
            }
            if cmn {
                // cmn: Rn + src (op=add)
                if sf {
                    buf.add_rr64(RAX, RCX);
                } else {
                    buf.add_rr32(RAX, RCX);
                }
            } else if sf {
                // ccmp: Rn - src (same sub semantics as `cmp` -> branch conds agree)
                buf.sub_rr64(RAX, RCX);
            } else {
                buf.sub_rr32(RAX, RCX);
            }
            store_nzcv(buf);
            let tail_off = buf.len() as i32;
            // patch displacements: disp relative to disp_off+4
            let jd = j_cond as i32;
            let d1 = ((cmp_off - (jd + 4)) as u32).to_le_bytes();
            buf.bytes[jd as usize..(jd + 4) as usize].copy_from_slice(&d1);
            let je = j_end as i32;
            let d2 = ((tail_off - (je + 4)) as u32).to_le_bytes();
            buf.bytes[je as usize..(je + 4) as usize].copy_from_slice(&d2);
            Ok(())
        }
        Inst::FcvtTzReg { rd, rn, dbl, unsigned } => {
            // fcvtzs/fcvtzu Dd,Dn / Sd,Sn: store the trunc toward-zero int of a FP
            // scalar back into the destination FP reg as an integer bit-pattern.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let dst = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            if dbl {
                buf.movq_load(0, RBX, vslot(rn));
                buf.cvttsd2si(RAX, 0);
            } else {
                buf.mov_load32(RAX, RBX, vslot(rn));
                buf.movd_xmm_r32(0, RAX);
                buf.cvtss2sd(0, 0);
                buf.cvttsd2si(RAX, 0);
            }
            if unsigned {
                // fcvtzu: clamp negatives to 0 (trunc-toward-zero unsigned).
                buf.mov_ri64(RCX, 0);
                buf.test_rr64(RAX, RAX);
                buf.cmov_rr64(0x48, RAX, RCX); // cmovs RAX,RCX (RAX<0 -> 0)
            }
            if dbl { buf.mov_store64(RBX, dst(rd), RAX); }
            else { buf.mov_store32(RBX, dst(rd), RAX); }
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
        Inst::FcvtHalf { rd, rn, op } => {
            // scalar FP16 <-> FP32/FP64 convert. Op:
            // 0=H->S (fcvt s,h)  1=S->H (fcvt h,s)  2=H->D (fcvt d,h)  3=D->H (fcvt h,d).
            // Reuses the F16C promote (vcvtph2ps) / demote (vcvtps2ph, imm 0 = RN).
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            match op {
                0 => {
                    buf.mov_load32(RAX, RBX, vslot(rn)); // h{rn} in low16
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                    buf.movd_r32_xmm(RAX, 0); // f32 in low32
                    buf.mov_store32(RBX, vslot(rd), RAX);
                }
                1 => {
                    buf.mov_load32(RAX, RBX, vslot(rn)); // f32 in low32
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                    buf.movd_r32_xmm(RAX, 0); // f16 in low16 (upper zeroed)
                    buf.mov_store32(RBX, vslot(rd), RAX);
                }
                2 => {
                    buf.mov_load32(RAX, RBX, vslot(rn)); // h{rn} in low16
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                    buf.cvtss2sd(0, 0); // (double) f32
                    buf.movq_store(RBX, vslot(rd), 0); // f64
                }
                3 => {
                    buf.movq_load(0, RBX, vslot(rn)); // f64
                    buf.cvtsd2ss(0, 0); // f32 in low32
                    buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                    buf.movd_r32_xmm(RAX, 0); // f16 in low16
                    buf.mov_store32(RBX, vslot(rd), RAX);
                }
                _ => return Err(format!("FcvtHalf: bad op {op}")),
            }
            Ok(())
        }
        Inst::Urhadd { rd, rn, rm, bytes } => {
            // urhadd Vd.T, Vn.T, Vm.T = (a+b+1)>>1 per byte lane. Byte lanes on
            // a 16B slot: load each byte of Vn and Vm, sum, +1, >>1, store.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            for i in 0..bytes {
                let off = i as i32;
                buf.mov_load32(RAX, RBX, slot(rn) + off);
                buf.and_ri64(RAX, 0xff);
                buf.mov_load32(RDX, RBX, slot(rm) + off);
                buf.and_ri64(RDX, 0xff);
                buf.add_rr64(RAX, RDX); // a+b
                buf.add_ri64(RAX, 1);   // a+b+1
                buf.mov_ri64(RCX, 1);
                buf.shr_cl64(RAX);      // (a+b+1)>>1
                buf.mov_store8(RBX, slot(rd) + off, RAX);
            }
            Ok(())
        }
        Inst::SimdFp16As { rd, rn, rm, op, q } => {
            // fadd/fsub/fmul/fdiv/fmax/fmin Vd.8h/.4h, Vn, Vm (half-precision
            // 3-same): per-h lane promote (clean, even with rd aliasing rn/rm
            // because each lane's sources are read before its dest write), op in
            // f32, demote. F16C:
            //   vcvtph2ps xmm,xmm = C4 E2 79 13 /r ; vcvtps2ph $0 = C4 E3 79 1D /r 00.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 8 } else { 4 }; // half-precision lanes
            // SSE scalar opcodes (F3 0F op rC1) for the pure-binary ops 0-7:
            // fadd/sub/mul/div/max/min/maxnm/minnm -> addss/subss/mulss/divss/
            // maxss/minss/maxss/minss (fmaxnm & fminnm behave like fmax/fmin for
            // the finite values the engine uses; NaNs differ only in propagate).
            let opcode: u8 = match op {
                0 => 0x58, // fadd  addss
                1 => 0x5c, // fsub  subss
                2 => 0x59, // fmul  mulss
                3 => 0x5e, // fdiv  divss
                4 => 0x5f, // fmax  maxss
                5 => 0x5d, // fmin  minss
                6 => 0x5f, // fmaxnm maxss
                7 => 0x5d, // fminnm minss
                _ => 0x59, // fmul lamine; op 8/9 take the accumulate path
            };
            // fmla/fmls accumulate BEFORE loading Vm into xmm1 (so we have a free
            // reg to read Vd). Binary ops (0-7) and fmla/fmls(8/9) share the
            // promote-Vn/promote-Vm -> op -> demote shape; fmla/fmls add/sub Vd.
            let inv = |buf: &mut crate::x86::CodeBuf, x: u8| {
                // fmla/fmls to SSE: mulss xmm2(=Vn),xmm1(=Vm); addss/subss xmm0(=Vd)
                buf.bytes.extend_from_slice(&[0xf3, 0x0f, 0x59, 0xca]); // mulss xmm1,xmm2
                if x == 8 {
                    buf.bytes.extend_from_slice(&[0xf3, 0x0f, 0x58, 0xc1]); // addss xmm0,xmm1
                } else {
                    buf.bytes.extend_from_slice(&[0xf3, 0x0f, 0x5c, 0xc1]); // subss xmm0,xmm1
                }
            };
            for i in 0..lanes {
                let off = i * 2;
                // Vn[lan] promote -> xmm2 (fmla/mlf) or xmm0 (binary via xmm1 op)
                buf.mov_load32(RAX, RBX, f(rn) + off);
                buf.movd_xmm_r32(2, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xd2]); // vcvtph2ps xmm2,xmm2
                // Vm[lan] promote -> xmm1
                buf.mov_load32(RAX, RBX, f(rm) + off);
                buf.movd_xmm_r32(1, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc9]); // vcvtph2ps xmm1,xmm1
                if op == 8 || op == 9 {
                    // fmla Vd[l] += Vn[l]*Vm[l] (promote Vd -> xmm0, mul, add/sub)
                    buf.mov_load32(RAX, RBX, f(rd) + off);
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                    inv(buf, op);
                } else {
                    // pure binary: Vn(xmm2) op Vm(xmm1) -> xmm0 (movaps xmm2->xmm0)
                    buf.bytes.extend_from_slice(&[0x0f, 0x28, 0xc2]); // movaps xmm0,xmm2
                    buf.bytes.extend_from_slice(&[0xf3, 0x0f, opcode, 0xc1]); // opss xmm0,xmm1
                }
                // demote -> f16, store 2 bytes at Vd[lan]
                buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                buf.movd_r32_xmm(RAX, 0);
                buf.mov_store16(RBX, f(rd) + off, RAX);
            }
            Ok(())
        }
        Inst::SimdFp16BEl { rd, rn, vlm, idx, op, q } => {
            // fmla/fmls/fmul Vd.8h/.4h, Vn, Vm.h[idx]: per-lane
            //   Vd[l] = (fmla/fmls) Vd[l] ± Vn[l]·splat(Vm.h[idx])  (in f32),
            //   fmul = splat(Vm.h[idx])·Vn[l]  (no accumulate).
            // Splat Vm.h[idx] into xmm2 ONCE (hoisted: rd==rm would clobber the
            // element inside the lane loop), then promote->op->demote per lane
            // via F16C. Uses the same raw-VEX f16 pattern as SimdFp16As.
            let db = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let nb = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let mb = crate::jit::VECTOR_BASE + (vlm as i32) * 16;
            let lanes: i32 = if q { 8 } else { 4 };
            // splat Vm.h[idx] -> xmm2 (as f32)
            buf.mov_load16(RAX, RBX, mb + (idx as i32) * 2);
            buf.movd_xmm_r32(2, RAX);
            buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xd2]); // vcvtph2ps xmm2,xmm2
            for l in 0..lanes {
                let off = l * 2;
                buf.mov_load32(RAX, RBX, db + off);
                buf.movd_xmm_r32(0, RAX); // Vd[l] (f16 in low16; may be garbage for fmul)
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0 (promote Vd)
                buf.mov_load32(RAX, RBX, nb + off);
                buf.movd_xmm_r32(1, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc9]); // vcvtph2ps xmm1,xmm1 (Vn[l])
                buf.mulss(1, 2); // xmm1 = Vn[l] * splat
                match op {
                    0 => buf.addss(0, 1), // fmla: Vd[l] + term
                    1 => buf.subss(0, 1), // fmls: Vd[l] - term
                    _ => {
                        // fmul: result = term in xmm1; copy to xmm0 so the demote
                        // below is the same xmm0->xmm0 vcvtps2ph for ALL ops.
                        buf.bytes.extend_from_slice(&[0x0f, 0x28, 0xc1]); // movaps xmm0,xmm1
                    }
                }
                // demote result xmm0 -> f16, store 2 bytes
                buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                buf.movd_r32_xmm(RAX, 0);
                buf.mov_store16(RBX, db + off, RAX);
            }
            Ok(())
        }
        Inst::SimdHadd { rd, rn, rm, unsigned, esize, q } => {
            // uhadd/shadd: per-lane floor((a+b)/2) = (a>>1)+(b>>1)+((a&1)&(b&1)).
            // For signed (shadd) use arithmetic shifts (sar) on sign-extended
            // values so the floor rounds toward -inf, matching ARM. esize-stride.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let e = esize as i32; // 1, 2, or 4 bytes
            for lane in 0..lanes {
                let off = lane * e;
                if unsigned {
                    // zero-extend loads, logical >>1
                    match e {
                        1 => { buf.movzx_byte_mem(RAX, RBX, f(rn) + off); buf.movzx_byte_mem(RCX, RBX, f(rm) + off); }
                        2 => { buf.mov_load16(RAX, RBX, f(rn) + off); buf.mov_load16(RCX, RBX, f(rm) + off); }
                        _ => { buf.mov_load32(RAX, RBX, f(rn) + off); buf.mov_load32(RCX, RBX, f(rm) + off); }
                    }
                } else {
                    // sign-extended loads, arithmetic >>1
                    match e {
                        1 => { buf.movsx_byte_mem(RAX, RBX, f(rn) + off); buf.movsx_byte_mem(RCX, RBX, f(rm) + off); }
                        2 => { buf.movsx_word_mem(RAX, RBX, f(rn) + off); buf.movsx_word_mem(RCX, RBX, f(rm) + off); }
                        _ => { buf.mov_load32(RAX, RBX, f(rn) + off); buf.movsxd_r64_r32(RAX, RAX);
                               buf.mov_load32(RCX, RBX, f(rm) + off); buf.movsxd_r64_r32(RCX, RCX); }
                    }
                }
                // floor term: (a>>1)+(b>>1) then +((a&1)&(b&1)).
                buf.mov_rr64(RDX, RAX); // RDX = a (save for carry)
                if unsigned { buf.shr_ri8(RAX, 1); } else { buf.sar_ri8(RAX, 1); }
                if unsigned { buf.shr_ri8(RCX, 1); } else { buf.sar_ri8(RCX, 1); }
                buf.add_rr64(RAX, RCX);
                // carry = (a&1)&(b&1): RDX &= b (reload b low), &= 1
                buf.mov_load16(RCX, RBX, f(rm) + off); // RCX = b (bit0)
                buf.and_rr64(RDX, RCX);
                buf.and_ri64(RDX, 1);
                buf.add_rr64(RAX, RDX);
                // store esize-low
                match e {
                    1 => buf.mov_store8(RBX, f(rd) + off, RAX),
                    2 => buf.mov_store16(RBX, f(rd) + off, RAX),
                    _ => buf.mov_store32(RBX, f(rd) + off, RAX),
                }
            }
            Ok(())
        }
        Inst::SimdHsub { rd, rn, rm, unsigned, esize, q } => {
            // shsub/uhsub: per-lane floor((a-b)/2). Compute diff=a-b in 64-bit then
            // shift right 1: arithmetic (sar) for signed, logical (shr) for unsigned.
            // Storing only the low esize bits yields the correct mod-2^esize floor
            // in both cases (unsigned (a-b) wraps, but floor((a-b)/2) mod 2^esize
            // of the wrapping difference equals the ARM result).
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let e = esize as i32;
            for lane in 0..lanes {
                let off = lane * e;
                if unsigned {
                    match e {
                        1 => { buf.movzx_byte_mem(RAX, RBX, f(rn) + off); buf.movzx_byte_mem(RCX, RBX, f(rm) + off); }
                        2 => { buf.mov_load16(RAX, RBX, f(rn) + off); buf.mov_load16(RCX, RBX, f(rm) + off); }
                        _ => { buf.mov_load32(RAX, RBX, f(rn) + off); buf.mov_load32(RCX, RBX, f(rm) + off); }
                    }
                } else {
                    match e {
                        1 => { buf.movsx_byte_mem(RAX, RBX, f(rn) + off); buf.movsx_byte_mem(RCX, RBX, f(rm) + off); }
                        2 => { buf.movsx_word_mem(RAX, RBX, f(rn) + off); buf.movsx_word_mem(RCX, RBX, f(rm) + off); }
                        _ => { buf.mov_load32(RAX, RBX, f(rn) + off); buf.movsxd_r64_r32(RAX, RAX);
                               buf.mov_load32(RCX, RBX, f(rm) + off); buf.movsxd_r64_r32(RCX, RCX); }
                    }
                }
                buf.sub_rr64(RAX, RCX); // diff = a - b
                if unsigned { buf.shr_ri8(RAX, 1); } else { buf.sar_ri8(RAX, 1); }
                match e {
                    1 => buf.mov_store8(RBX, f(rd) + off, RAX),
                    2 => buf.mov_store16(RBX, f(rd) + off, RAX),
                    _ => buf.mov_store32(RBX, f(rd) + off, RAX),
                }
            }
            Ok(())
        }
        Inst::SimdTbl { rd, rn, rm, len2, tbx, q } => {
            // tbl/tbx Vd.16B|8B, {Vn..Vn+len2}, Vm: per byte lane i, idx=Vm[i].
            // Table = len2+1 contiguous 16-byte regs starting at slot(rn); because
            // they're contiguous, table byte `idx` lives at slot(rn)+idx. If
            // idx < 16*(len2+1): Vd[i]=table[idx]; else Vd[i]=0 (tbl) / keep (tbx).
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 16 } else { 8 };
            let table_bytes = 16 * (len2 as i32 + 1);
            for i in 0..lanes {
                // idx = Vm[i]
                buf.movzx_byte_mem(RAX, RBX, f(rm) + i);
                buf.cmp_ri64(RAX, table_bytes as u32);
                // If idx >= table_bytes, skip the table lookup (keep Vd[i] for tbx,
                // or remember to clear it below for tbl).
                let j_ge = buf.jcc_rel32(0x83); // JAE (unsigned >=)
                // table[idx] -> AL
                buf.movzx_byte_mem_idxd(RAX, RBX, RAX, f(rn));
                buf.mov_store8(RBX, f(rd) + i, RAX);
                let j_end = buf.jmp_rel32();
                let ge_target = buf.bytes.len() as i64;
                let disp_ge = (ge_target - (j_ge as i64 + 4)) as i32;
                buf.bytes[j_ge..j_ge + 4].copy_from_slice(&disp_ge.to_le_bytes());
                // for tbl (not tbx), clear Vd[i] when idx out of table range:
                if !tbx {
                    buf.mov_ri64(RAX, 0);
                    buf.mov_store8(RBX, f(rd) + i, RAX);
                }
                let end_target = buf.bytes.len() as i64;
                let disp_end = (end_target - (j_end as i64 + 4)) as i32;
                buf.bytes[j_end..j_end + 4].copy_from_slice(&disp_end.to_le_bytes());
            }
            Ok(())
        }
        Inst::SimdFreFrsqrte { rd, rn, sqrt, esize, q } => {
            // frecpe/frsqrte Vd.T, Vn.T: per-lane approximate reciprocal or
            // 1/sqrt. esize 4 -> rcpss/rsqrtss on the promoted 32-bit; esize 8
            // -> rcpsd/rsqrtsd on the 64-bit. Keep 32-bit for esize==4 (load
            // f32, op, store f32); for esize 8 load/store f64.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let e = esize as i32;
            // SSE: F3 0F 53 = rcpss (32-bit), F2 0F 53 /r = rcpsd (64-bit);
            // F3 0F 52 = rsqrtss, F2 0F 52 = rsqrtsd.
            for lane in 0..lanes {
                let off = lane * e;
                if e == 4 {
                    buf.mov_load32(RAX, RBX, f(rn) + off);
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(if sqrt { &[0xf3, 0x0f, 0x52, 0xc0] } else { &[0xf3, 0x0f, 0x53, 0xc0] });
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, f(rd) + off, RAX);
                } else {
                    buf.movq_load(0, RBX, f(rn) + off);
                    buf.bytes.extend_from_slice(if sqrt { &[0xf2, 0x0f, 0x52, 0xc0] } else { &[0xf2, 0x0f, 0x53, 0xc0] });
                    buf.movq_store(RBX, f(rd) + off, 0);
                }
            }
            Ok(())
        }
        Inst::SimdAbd { rd, rn, rm, signed: _signed, esize, q } => {
            // uabd/sabd Vd.T, Vn.T, Vm.T: per-lane |Vn - Vm|. Both uabd and sabd
            // compute the magnitude of the two's-complement (esize) difference,
            // so one path serves both: diff = (Vn - Vm) mod 2^esize; if the
            // esize sign bit is SET, diff = -diff. Per-lane on the GP registers
            // (tiny element counts: 8/16/8/4/4/2 lanes).
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let e = esize as i32; // 1, 2, or 4 bytes per element
            for lane in 0..lanes {
                let off = lane * e;
                let (sb, hb): (i32, i32) = match e {
                    1 => (8, 7),
                    2 => (16, 15),
                    _ => (32, 31),
                };
                // RAX = Vn[lane], RCX = Vm[lane] (zero-extended in the reg)
                match e {
                    1 => { buf.mov_load16(RAX, RBX, f(rn) + off); buf.mov_load16(RCX, RBX, f(rm) + off); }
                    2 => { buf.mov_load16(RAX, RBX, f(rn) + off); buf.mov_load16(RCX, RBX, f(rm) + off); }
                    _ => { buf.mov_load32(RAX, RBX, f(rn) + off); buf.mov_load32(RCX, RBX, f(rm) + off); }
                }
                buf.sub_rr64(RAX, RCX); // RAX = diff (low esize bits correct)
                // If the esize sign bit (e*8-1) of the diff is SET, RAX = -RAX.
                let signbit = (e * 8 - 1) as u8;
                buf.bytes.extend_from_slice(&[0x48, 0x0f, 0xba, 0xe0, signbit]); // bt RAX, signbit (CF=bit)
                let jcc = buf.jcc_rel32(0x83); // JNB: jump if CF=0 (diff non-negative)
                // negate: RAX = 0 - RAX
                buf.mov_ri64(RCX, 0);
                buf.sub_rr64(RCX, RAX);
                buf.mov_rr64(RAX, RCX);
                let end = buf.len();
                let disp = (end as i64 - (jcc as i64 + 4)) as i32;
                buf.bytes[jcc..jcc + 4].copy_from_slice(&disp.to_le_bytes());
                // store the esize-low bits
                match e {
                    1 => buf.mov_store8(RBX, f(rd) + off, RAX),
                    2 => buf.mov_store16(RBX, f(rd) + off, RAX),
                    _ => buf.mov_store32(RBX, f(rd) + off, RAX),
                }
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
                Inst::Simd4s { rd, rn, rm, op: 1 } => {
                    // sub Vd.4s, Vn.4s, Vm.4s : 4x32-bit lane subtract via psubd.
                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    buf.movdqu_load(0, RBX, slot(rn));
                    buf.movdqu_load(1, RBX, slot(rm));
                    buf.psubd(0, 1); // xmm0 = Vn - Vm (4x32)
                    buf.movdqu_store(RBX, slot(rd), 0);
                    Ok(())
                }
        Inst::SimdFpToInt { rd, rn, unsigned, esize, q, mode } => {
            // fcvtas Vd.T, Vn.T : round FP vector lanes to integer lanes.
            // Per-lane, modeled on the scalar FcvtToInt. The decode gate only
            // ever emits signed fcvtas (mode=2, unsigned=false) for the real
            // boot path; truncate (fcvtzs, mode=0) is the only other mode this
            // Inst can carry and is handled directly. esize 4 loads f32
            // (promote to f64); esize 8 loads f64.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: u32 = if q {
                if esize == 8 { 2 } else { 4 }
            } else {
                if esize == 8 { 1 } else { 2 }
            };
            let _ = unsigned;
            let elem = esize as i32; // 4 or 8 bytes per element
            for lane in 0..lanes {
                let off = lane as i32 * elem;
                if esize == 8 {
                    buf.movq_load(0, RBX, slot(rn) + off); // xmm0 = f64 lane
                } else {
                    buf.mov_load32(RAX, RBX, slot(rn) + off); // RAX = f32 bits
                    buf.movd_xmm_r32(0, RAX);
                    buf.cvtss2sd(0, 0); // promote to f64
                }
                if mode == 2 {
                    // fcvtas: round-to-nearest (MXCSR nearest; ARM is ties-
                    // away, x86 ties-even — acceptable for render/color, and
                    // matches the scalar FcvtToInt path exactly).
                    buf.cvtsd2si(RAX, 0);
                } else {
                    // mode 0 (fcvtzs): truncate toward zero.
                    buf.cvttsd2si(RAX, 0);
                }
                // store int32 lane (.2s/.4s) or int64 (esize 8).
                if esize == 8 {
                    buf.mov_store64(RBX, slot(rd) + off, RAX);
                } else {
                    buf.mov_store32(RBX, slot(rd) + (lane as i32) * 4, RAX);
                }
            }
            Ok(())
        }
        Inst::SimdI2Fp16 { rd, rn, unsigned, q } => {
            // scvtf/ucvtf Vd.4H/.8H, Vn: per halfword lane, integer -> f16.
            //   signed: load s16, cvtsi2ss (f32), demote f16; store 2 bytes.
            //   unsigned: load u16 (zero-extend), cvtsi2ss (signed is fine since
            //   u16 <= 65535 < i32::MAX), demote.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 8 } else { 4 };
            for l in 0..lanes {
                let off = l * 2;
                if unsigned {
                    buf.mov_load16(RAX, RBX, f(rn) + off); // zero-extend u16
                } else {
                    buf.movsx_word_mem(RAX, RBX, f(rn) + off); // sign-extend s16
                }
                buf.cvtsi2ss(0, false, RAX); // xmm0 = (float)int32 -> low f32
                // demote to 16-bit float (F16C imm 0 = round-nearest)
                buf.bytes.extend_from_slice(&[0xc4, 0xe3, 0x79, 0x1d, 0xc0, 0x00]); // vcvtps2ph $0,xmm0,xmm0
                buf.movd_r32_xmm(RAX, 0);
                buf.mov_store16(RBX, f(rd) + off, RAX);
            }
            Ok(())
        }
        Inst::Pmull1q { rd, rn, rm, hi } => {
            // pmull/pmull2 Vd.1Q, Vn.1D, Vm.1D : 64x64 carry-less (polynomial)
            // multiply -> 128-bit. x86 PCLMULQDQ with imm=0 (low64 x low64),
            // loading the selected 64-bit half of Vn/Vm into the low lane first.
            // pmull (hi=false): element 0 (bytes 0..7); pmull2 (hi=true): element
            // 1 (bytes 8..15). Result (128b) is stored to Vd's 16-byte slot.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let n_off = slot(rn) + if hi { 8 } else { 0 };
            let m_off = slot(rm) + if hi { 8 } else { 0 };
            let d_off = slot(rd);
            // xmm0 = low64 of selected itmem of Vn, high64 zeroed (movq_load zeroes).
            buf.movq_load(0, RBX, n_off);
            buf.movq_load(1, RBX, m_off);
            buf.pclmulq(0, 1, 0x00); // xmm0 = clmul(xmm0.low64, xmm1.low64)
            buf.movdqu_store(RBX, d_off, 0); // 128-bit result to Vd
            Ok(())
        }
        Inst::SimdFp16Cmpz { rd, rn, op, q } => {
            // fcmeq/fcmgt/fcmge/fcmle/fcmlt Vd.4H/.8H, Vn, #0 : per-lane OP against
            // +0.0, all-ones or 0 mask (16-bit each). Promote half->f32 (F16C),
            // compare to 0.0f32 with ucomiss, setcc, neg to build 0xffff/0x0000.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 8 } else { 4 };
            for l in 0..lanes {
                let off = l * 2;
                buf.mov_load16(RAX, RBX, f(rn) + off); // 0F B7 = MOVZX -> r32
                buf.movd_xmm_r32(0, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0 (F16C)
                // xmm1 = 0.0f32
                buf.pxor_xmm(1, 1);
                buf.comiss(0, 1);
                // setcc AL based on op, then neg to 0xffff/0.
                let cc: u8 = match op {
                    0 => 0x04, // eq => ZF set => E
                    1 => 0x07, // gt => (CF=0 && ZF=0) => A (above)
                    2 => 0x03, // ge => CF clear => AE/NC
                    3 => 0x02, // lt => CF set => B
                    _ => 0x06, // le => (CF set | ZF set) => BE/NA
                };
                buf.setcc_rm8(cc, 0);
                buf.movzx_r32_r8(RAX, RAX);
                buf.neg_r64(RAX); // 0->0, 1->0xffff_ffff_ffff_ffff
                buf.mov_store16(RBX, f(rd) + off, RAX); // low 16 = mask
            }
            Ok(())
        }
        Inst::SimdFp16Cmp { rd, rn, rm, op, q } => {
            // fcmeq/fcmge/fcmgt Vd.4H/.8H, Vn, Vm : per-lane cond(Vn,Vm) -> all-ones
            // or 0. Promote both halves to f32 (F16C), comiss, setcc, neg.
            let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes: i32 = if q { 8 } else { 4 };
            for l in 0..lanes {
                let off = l * 2;
                buf.mov_load16(RAX, RBX, f(rn) + off);
                buf.movd_xmm_r32(0, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0
                buf.mov_load16(RAX, RBX, f(rm) + off);
                buf.movd_xmm_r32(1, RAX);
                buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc9]); // vcvtph2ps xmm1,xmm1 (F16C)
                buf.comiss(0, 1); // sets flags for xmm0 vs xmm1
                let cc: u8 = match op {
                    0 => 0x04, // fcmeq: ZF -> E
                    1 => 0x03, // fcmge: CF clear -> AE
                    _ => 0x07, // fcmgt: CF&ZF clear -> A
                };
                buf.setcc_rm8(cc, 0);
                buf.movzx_r32_r8(RAX, RAX);
                buf.neg_r64(RAX);
                buf.mov_store16(RBX, f(rd) + off, RAX);
            }
            Ok(())
        }
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
                        Inst::ScalarUcvtf { rd, rn, sng } => {
                            // ucvtf Dd/Dd or Sd,Sd : read Dn's low bits as an
                            // UNSIGNED integer and write the float to Dd/Sd.
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            if sng {
                                // S-form: low 32-bit lane as unsigned i32 -> f32.
                                // Use src64=true signed-i64 convert on a zero-extended
                                // RAX: max u32 (0xffffffff) is < 2^63, so an unsigned
                                // u32 -> f32 conversion is EXACTLY the i64->f32 of the
                                // zero-extended value (no sign/2^63 correction needed
                                // within the u32 range).
                                buf.mov_load32(RAX, RBX, slot(rn));
                                buf.cvtsi2ss(0, true, RAX);   // u32(>=0 as i64) -> f32
                                buf.movd_r32_xmm(RCX, 0);
                                buf.mov_store32(RBX, slot(rd), RCX);
                            } else {
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
                            }
                            Ok(())
                        }
                        Inst::ScalarScvtf { rd, rn, sng } => {
                            // scvtf Dd, Dn : read Dn's low 64 bits as a SIGNED integer
                            // and write the double to Dd (two's-complement -> f64, signed).
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            if sng {
                                buf.mov_load32(RAX, RBX, slot(rn));   // low 32 as signed i32
                                buf.cvtsi2ss(0, false, RAX);          // i32 -> f32
                                buf.movd_r32_xmm(RCX, 0);
                                buf.mov_store32(RBX, slot(rd), RCX);
                            } else {
                            buf.mov_load64(RDX, RBX, slot(rn));
                            buf.cvtsi2sd(0, true, RDX); // signed i64 -> f64
                            buf.movq_store(RBX, slot(rd), 0);
                            }
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
                                    4 => buf.maxsd(0, 1), // fmax
                                    5 => buf.minsd(0, 1), // fmin
                                    // fmaxnm/fminnm behave like max/min on the finite
                                    // values the engine uses (NaN propagation differs).
                                    6 => buf.maxsd(0, 1), // fmaxnm
                                    7 => buf.minsd(0, 1), // fminnm
                                    // fabd = |Vn - Vm|: subsd then clear the sign bit.
                                    8 => {
                                        buf.subsd(0, 1); // xmm0 = a - b
                                        buf.mov_ri64(RDX, 0x7fff_ffff_ffff_ffff);
                                        buf.movq_xmm_r64(1, RDX);
                                        buf.pand(0, 1); // clear sign bit (|a-b|)
                                    }
                                    _ => return Err(format!("Simd2dFp op {op} not implemented")),
                                }
                                buf.movq_store(RBX, slot(rd) + off, 0);
                            }
                            Ok(())
                        }
                        Inst::SimdDupGp { rd, rn, esize, q } => {
                            // dup Vd.T, Wn/Xn: broadcast the element read from GPR rn
                            // into all `lanes` of Vd. esize 1/2/4: low 32 of Wn; esize 8: Xn.
                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                            if esize == 8 {
                                ldg(buf, RAX, rn as u32); // Xn low 64
                            } else {
                                ldg(buf, RAX, rn as u32);
                                buf.zero_ext_r32(RAX); // zero-extend Wn
                                let m = ((1u64 << (8 * esize)) - 1) as u32;
                                buf.and_ri64(RAX, m);
                            }
                            let lanes = ((if q { 16 } else { 8 }) / esize as i32) as u32;
                            for i in 0..lanes {
                                match esize {
                                    1 => buf.mov_store8(RBX, slot(rd) + (i * 1) as i32, RAX),
                                    2 => buf.mov_store16(RBX, slot(rd) + (i * 2) as i32, RAX),
                                    4 => buf.mov_store32(RBX, slot(rd) + (i * 4) as i32, RAX),
                                    _ => buf.mov_store64(RBX, slot(rd) + (i * 8) as i32, RAX),
                                }
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
                                                                                                                                                                                                            Inst::SimdMulH { rd, rn, rm, lanes } => {
                                                                                                                                                                                                                // mul Vd.8H/.4H, Vn., Vm.: per 16-bit lane, low-16 product
                                                                                                                                                                                                                // (mod-2^16 wrap). 64-bit imul of zero-extended 16-bit operands yields
                                                                                                                                                                                                                // the correct low-16 for signed/unsigned halfwords (imul truncates).
                                                                                                                                                                                                                let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                for i in 0..lanes {
                                                                                                                                                                                                                    let off = (i as i32) * 2;
                                                                                                                                                                                                                    buf.movzx_word_mem(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                                    buf.movzx_word_mem(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                                    buf.imul_rr64(RAX, RCX);
                                                                                                                                                                                                                    buf.mov_store16(RBX, slot(rd) + off, RAX);
                                                                                                                                                                                                                }
                                                                                                                                                                                                                Ok(())
                                                                                                                                                                                                            }
                                                                                                                    Inst::SimdMla { rd, rn, rm, lanes, sub } => {
                                                                                        // mla/mls Vd.4S/2S, Vn., Vm.: Vd = Vd +/- Vn*Vm per 32-bit lane.
                                                                                        // low-32 of the product, accumulate into the existing Vd lane.
                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                        for i in 0..lanes {
                                                                                            let off = (i as i32) * 4;
                                                                                            buf.mov_load32(RAX, RBX, slot(rn) + off);
                                                                                            buf.mov_load32(RCX, RBX, slot(rm) + off);
                                                                                            buf.imul_rr64(RAX, RCX); // low 32 = (a*b) mod 2^32
                                                                                            if sub {
                                                                                                // Vd = Vd - Vn*Vm
                                                                                                buf.mov_load32(RDX, RBX, slot(rd) + off);
                                                                                                buf.sub_rr64(RDX, RAX);
                                                                                                buf.mov_store32(RBX, slot(rd) + off, RDX);
                                                                                            } else {
                                                                                                // Vd = Vd + Vn*Vm
                                                                                                buf.mov_load32(RDX, RBX, slot(rd) + off);
                                                                                                buf.add_rr64(RDX, RAX);
                                                                                                buf.mov_store32(RBX, slot(rd) + off, RDX);
                                                                                            }
                                                                                        }
                                                                                        Ok(())
                                                                                                                    }
                                                                                    Inst::SimdMlaEl { rd, rn, rm, index, lanes, sub } => {
                                                                                        // mla/mls Vd.4S/2S, Vn., Vm.S[idx]: Vd[i] = Vd[i] +/- Vn[i]*Vm.el
                                                                                        // (Vm's indexed single element broadcast to every lane).
                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                        let mel = slot(rm) + (index as i32) * 4;
                                                                                        for i in 0..lanes {
                                                                                            let off = (i as i32) * 4;
                                                                                            buf.mov_load32(RAX, RBX, slot(rn) + off);
                                                                                            buf.mov_load32(RCX, RBX, mel); // broadcast element
                                                                                            buf.imul_rr64(RAX, RCX);
                                                                                            if sub {
                                                                                                buf.mov_load32(RDX, RBX, slot(rd) + off);
                                                                                                buf.sub_rr64(RDX, RAX);
                                                                                                buf.mov_store32(RBX, slot(rd) + off, RDX);
                                                                                            } else {
                                                                                                buf.mov_load32(RDX, RBX, slot(rd) + off);
                                                                                                buf.add_rr64(RDX, RAX);
                                                                                                buf.mov_store32(RBX, slot(rd) + off, RDX);
                                                                                            }
                                                                                        }
                                                                                        Ok(())
                                                                                                                    }
                                                                                                                    Inst::SimdMull { rd, rn, rm, res_esize, unsigned, q, acc, sub } => {
                                                                                                        // smull/umull/smlal/umlal: widen each src element to res_esize and
                                                                                                        // multiply (or add to the existing result if acc).
                                                                                                        // src_es = res_esize/2; lanes = (8 source bytes)/src_es = 16/res_esize
                                                                                                        // (2 for .2s->.2d, 4 for .4h->.4s, 8 for .8b->.8h). Store EXACTLY
                                                                                                        // res_esize bytes per lane so we never overrun the next element.
                                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                        let src_es: i32 = (res_esize as i32) / 2;
                                                                                                        let lanes = (16 / res_esize as i32) as usize;
                                                                                                        let uphalf = if q { 8 } else { 0 }; // smull2 reads the upper reg half
                                                                                                        // In-place widening alias: the wide (res_esize) write of lane i at
                                                                                                        // i*res_esize clobbers the narrow (src_es) operand bytes lane i+1 reads
                                                                                                        // at (i+1)*src_es when rd aliases a source (`smull v0.2d, v0.2s, …`).
                                                                                                        let rn_base = if rn == rd { permute_source(buf, rd, rn, false) } else { slot(rn) };
                                                                                                        let rm_base = if rm == rd { permute_source(buf, rd, rm, true) } else { slot(rm) };
                                                                                                        for i in 0..lanes {
                                                                                                            let soff = uphalf + (i as i32) * src_es;
                                                                                                            match src_es {
                                                                                                                4 => { buf.mov_load32(RAX, RBX, rn_base+soff); buf.mov_load32(RCX, RBX, rm_base+soff); }
                                                                                                                2 => { buf.movzx_word_mem(RAX, RBX, rn_base+soff); buf.movzx_word_mem(RCX, RBX, rm_base+soff); }
                                                                                                                _ => { buf.movzx_byte_mem(RAX, RBX, rn_base+soff); buf.movzx_byte_mem(RCX, RBX, rm_base+soff); }
                                                                                                            }
                                                                                                            if !unsigned {
                                                                                                                // sign-extend the zero-extended operand up to 64 bits (shift by (64-8*src))
                                                                                                                let se = 64 - 8 * (res_esize as u16 / 2);
                                                                                                                buf.shl_ri8(RAX, (se % 64) as u8);
                                                                                                                buf.sar_ri8(RAX, (se % 64) as u8);
                                                                                                                buf.shl_ri8(RCX, (se % 64) as u8);
                                                                                                                buf.sar_ri8(RCX, (se % 64) as u8);
                                                                                                            }
                                                                                                            buf.imul_rr64(RAX, RCX);
                                                                                                            let doff = (i as i32) * (res_esize as i32);
                                                                                                            if acc {
                                                                                                                                                            match res_esize {
                                                                                                                                                                8 => buf.mov_load64(RDX, RBX, slot(rd)+doff),
                                                                                                                                                                4 => buf.mov_load32(RDX, RBX, slot(rd)+doff),
                                                                                                                                                                _ => buf.movzx_word_mem(RDX, RBX, slot(rd)+doff),
                                                                                                                                                            }
                                                                                                                                                            if sub {
                                                                                                                                                                buf.sub_rr64(RDX, RAX); // smlsl/umlsl: Vd = Vd - prod
                                                                                                                                                            } else {
                                                                                                                                                                buf.add_rr64(RDX, RAX);
                                                                                                                                                            }
                                                                                                                match res_esize {
                                                                                                                    8 => buf.mov_store64(RBX, slot(rd)+doff, RDX),
                                                                                                                    4 => buf.mov_store32(RBX, slot(rd)+doff, RDX),
                                                                                                                    _ => buf.mov_store16(RBX, slot(rd)+doff, RDX),
                                                                                                                }
                                                                                                            } else {
                                                                                                                match res_esize {
                                                                                                                    8 => buf.mov_store64(RBX, slot(rd)+doff, RAX),
                                                                                                                    4 => buf.mov_store32(RBX, slot(rd)+doff, RAX),
                                                                                                                    _ => buf.mov_store16(RBX, slot(rd)+doff, RAX),
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                        Ok(())
                                                                                                    }
                                                                                                                    Inst::SimdMullEl { rd, rn, rm, index, res_esize, unsigned, q, acc, sub } => {
                                        // Integer widening multiply by element:
                                        // Vd[i] +=/|= Vn[i] * Vm[index], where the
                                        // m operand is ONE element (selected by
                                        // `index`) broadcast to every lane. Same
                                        // widen/mul/accumulate per lane as
                                        // SimdMull, but AM: m reads the single
                                        // indexed element from slot(rm) instead
                                        // of lane i. Session 44 (fuzzer-caught).
                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                        let src_es: i32 = (res_esize as i32) / 2;
                                        let lanes = (16 / res_esize as i32) as usize;
                                        let uphalf = if q { 8 } else { 0 };
                                        // m element base (guest indexed by esize bytes)
                                        let mbase = slot(rm) + (index as i32) * src_es;
                                        let rn_base = if rn == rd { permute_source(buf, rd, rn, false) } else { slot(rn) };
                                        for i in 0..lanes {
                                            let soff = uphalf + (i as i32) * src_es;
                                            match src_es {
                                                4 => { buf.mov_load32(RAX, RBX, rn_base+soff); buf.mov_load32(RCX, RBX, mbase); }
                                                2 => { buf.movzx_word_mem(RAX, RBX, rn_base+soff); buf.movzx_word_mem(RCX, RBX, mbase); }
                                                _ => { buf.movzx_byte_mem(RAX, RBX, rn_base+soff); buf.movzx_byte_mem(RCX, RBX, mbase); }
                                            }
                                            if !unsigned {
                                                let se = 64 - 8 * (res_esize as u16 / 2);
                                                buf.shl_ri8(RAX, (se % 64) as u8);
                                                buf.sar_ri8(RAX, (se % 64) as u8);
                                                buf.shl_ri8(RCX, (se % 64) as u8);
                                                buf.sar_ri8(RCX, (se % 64) as u8);
                                            }
                                            buf.imul_rr64(RAX, RCX);
                                            let doff = (i as i32) * (res_esize as i32);
                                            if acc {
                                                match res_esize {
                                                    8 => buf.mov_load64(RDX, RBX, slot(rd)+doff),
                                                    4 => buf.mov_load32(RDX, RBX, slot(rd)+doff),
                                                    _ => buf.movzx_word_mem(RDX, RBX, slot(rd)+doff),
                                                }
                                                if sub {
                                                    buf.sub_rr64(RDX, RAX);
                                                } else {
                                                    buf.add_rr64(RDX, RAX);
                                                }
                                                match res_esize {
                                                    8 => buf.mov_store64(RBX, slot(rd)+doff, RDX),
                                                    4 => buf.mov_store32(RBX, slot(rd)+doff, RDX),
                                                    _ => buf.mov_store16(RBX, slot(rd)+doff, RDX),
                                                }
                                            } else {
                                                match res_esize {
                                                    8 => buf.mov_store64(RBX, slot(rd)+doff, RAX),
                                                    4 => buf.mov_store32(RBX, slot(rd)+doff, RAX),
                                                    _ => buf.mov_store16(RBX, slot(rd)+doff, RAX),
                                                }
                                            }
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
                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX);
                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0); // default 0
                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff); // ones
                                                                                                                                                                                                                                                                                                                                                                                    buf.cmov_rr64(0x47, RDI, RDX); // cmova(above): if Vn>Vm, RDI=ones
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
                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX); // unsigned: CF=0 if Vn>=Vm
                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0); // default 0
                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff); // ones
                                                                                                                                                                                                                                                                                                                                                                                    buf.cmov_rr64(0x47, RDI, RDX); // cmova(above): if Vn>Vm, RDI=ones
                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_store64(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                    Ok(())
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            Inst::SimdCmhiH { rd, rn, rm, lanes } => {
                                                                                                                            // cmhi Vd.8H/.4H: per 16-bit lane, all-ones if Vn[i] >
                                                                                                                            // Vm[i] (unsigned), else 0. Compare 16-bit unsigned, cmova.
                                                                                                                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                            for i in 0..lanes {
                                                                                                                                                                                            let off = (i as i32) * 2;
                                                                                                                                                                                            buf.mov_load16(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                            buf.mov_load16(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                            buf.and_ri64(RAX, 0xffff); // clear garbage high bits (mov_load16 zx->32 only)
                                                                                                                                                                                            buf.and_ri64(RCX, 0xffff);
                                                                                                                                                                                            buf.cmp_rr64(RAX, RCX);
                                                                                                                                                                                                                                                            buf.mov_ri64(RDI, 0); // default 0
                                                                                                                                                                                                                                                            buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff); // ones
                                                                                                                                                                                                                                                            buf.cmov_rr64(0x47, RDI, RDX); // cmova(above): if Vn>Vm, RDI=ones
                                                                                                                                                                                                                                                            buf.mov_store16(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                        }
                                                                                                                            Ok(())
                                                                                                                        }
                                                                                                                        Inst::SimdCmhiB { rd, rn, rm, lanes, ge } => {
                                                                                                                            // cmhi v.16B/cms v.16B: per byte lane all-ones if Vn[i] > Vm[i] (cmhi,
                                                                                                                            // ge=false) or >= (cmhi, ge=true). Loads zero-extend to 32; cmp; cmova / cmovae.
                                                                                                                            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                            let cc = if ge { 0x43u8 } else { 0x47u8 }; // cmovae(>=) / cmova(>)
                                                                                                                            for i in 0..lanes {
                                                                                                                                let off = (i as i32);
                                                                                                                                                                                                buf.movzx_byte_mem(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                buf.movzx_byte_mem(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                buf.and_ri64(RAX, 0xff); // clear garbage high bits (movzx only clears r32)
                                                                                                                                                                                                buf.and_ri64(RCX, 0xff);
                                                                                                                                                                                                buf.cmp_rr64(RAX, RCX);
                                                                                                                                                                                                                                                                buf.mov_ri64(RDX, 0);
                                                                                                                                                                                                                                                                buf.mov_ri64(RCX, 0xffff_ffff_ffff_ffff);
                                                                                                                                                                                                                                                                buf.cmov_rr64(cc, RDX, RCX);
                                                                                                                                                                                                                                                                // store low byte of RDX (DL) — RAX/RDX have REX-free 8-bit forms
                                                                                                                                                                                                                                                                buf.mov_store8(RBX, slot(rd) + off, RDX);
                                                                                                                            }
                                                                                                                            Ok(())
                                                                                                                        }
                                                                                                                        Inst::SimdCmhs { rd, rn, rm, lanes } => {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                // cmhs Vd.4S/2S: per 32-bit lane all-ones if Vn[i] >= Vm[i]
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                // (unsigned "higher or same"); cmovae (cc 0x43) since equal passes.
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                for i in 0..lanes {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let off = (i as i32) * 4;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load32(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load32(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX); // unsigned: CF=0 if Vn>=Vm, CF=1 if Vn<Vm
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0); // default 0
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff); // ones
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.cmov_rr64(0x43, RDI, RDX); // cmovae: ones iff Vn>=Vm (equal passes)
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_store32(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                Ok(())
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            Inst::SimdCmhsD { rd, rn, rm } => {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                // cmhs Vd.2D (Q=1): per 64-bit lane ones if Vn[i] >= Vm[i].
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                for i in 0..2u32 {
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    let off = (i as i32) * 8;
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load64(RAX, RBX, slot(rn) + off);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load64(RCX, RBX, slot(rm) + off);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.cmp_rr64(RAX, RCX); // CF=0 if Vn>=Vm
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDI, 0xffff_ffff_ffff_ffff);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_ri64(RDX, 0);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.cmov_rr64(0x43, RDI, RDX); // cmovae
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_store64(RBX, slot(rd) + off, RDI);
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                Ok(())
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                                                        Inst::SimdCmgt { rd, rn, rm, lanes, esize, ge } => {
                                                                                                                                                                                                                                                                                                                                                                                    // cmgt/cmge Vd.T, Vn.T, Vm.T: per lane all-ones if Vn > Vm (cmgt) or
                                                                                                                                                                                                                                                                                                                                                                                    // Vn >= Vm (cmge), SIGNED. Loads SIGN-EXTEND (movsx) so the 64-bit cmp
                                                                                                                                                                                                                                                                                                                                                                                    // is a correct signed compare even for negative lanes.
                                                                                                                                                                                                                                                                                                                                                                                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                                                                                                                                                                                                                    let ones = 0xffff_ffff_ffff_ffffu64;
                                                                                                                                                                                                                                                                                                                                                                                    for i in 0..lanes {
                                                                                                                                                                                                                                                                                                                                                                                        let off = (i as i32) * esize as i32;
                                                                                                                                                                                                                                                                                                                                                                                        // sign-extend operand into RAX (Vn) / RCX (Vm)
                                                                                                                                                                                                                                                                                                                                                                                        match esize {
                                                                                                                                                                                                                                                                                                                                                                                            8 => { buf.mov_load64(RAX, RBX, slot(rn)+off); buf.mov_load64(RCX, RBX, slot(rm)+off); }
                                                                                                                                                                                                                                                                                                                                                                                            4 => { buf.mov_load32(RAX, RBX, slot(rn)+off); buf.movsxd_r64_r32(RAX, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                   buf.mov_load32(RCX, RBX, slot(rm)+off); buf.movsxd_r64_r32(RCX, RCX); }
                                                                                                                                                                                                                                                                                                                                                                                            2 => { buf.movsx_word_mem(RAX, RBX, slot(rn)+off); buf.movsx_word_mem(RCX, RBX, slot(rm)+off); }
                                                                                                                                                                                                                                                                                                                                                                                            _ => { buf.movsx_byte_mem(RAX, RBX, slot(rn)+off); buf.movsx_byte_mem(RCX, RBX, slot(rm)+off); }
                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                                        buf.cmp_rr64(RAX, RCX);
                                                                                                                                                                                                                                                                                                                                                                                        buf.mov_ri64(RDI, ones);
                                                                                                                                                                                                                                                                                                                                                                                        buf.mov_ri64(RDX, 0);
                                                                                                                                                                                                                                                                                                                                                                                        if ge {
                                                                                                                                                                                                                                                                                                                                                                                            // all-ones iff Vn >= Vm (clear when Vn < Vm): cmovl 0x4c
                                                                                                                                                                                                                                                                                                                                                                                            buf.cmov_rr64(0x4c, RDI, RDX);
                                                                                                                                                                                                                                                                                                                                                                                        } else {
                                                                                                                                                                                                                                                                                                                                                                                            // all-ones iff Vn > Vm (clear when Vn <= Vm): cmovle 0x4e
                                                                                                                                                                                                                                                                                                                                                                                            buf.cmov_rr64(0x4e, RDI, RDX);
                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                                        match esize {
                                                                                                                                                                                                                                                                                                                                                                                            8 => buf.mov_store64(RBX, slot(rd)+off, RDI),
                                                                                                                                                                                                                                                                                                                                                                                            4 => buf.mov_store32(RBX, slot(rd)+off, RDI),
                                                                                                                                                                                                                                                                                                                                                                                            2 => buf.mov_store16(RBX, slot(rd)+off, RDI),
                                                                                                                                                                                                                                                                                                                                                                                            _ => buf.mov_store8(RBX, slot(rd)+off, RDI),
                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                    Ok(())
                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                        Inst::SimdUz1 { rd, rn, rm, esize, q } => {
                                                                                                                    // uzp1 Vd.T, Vn.T, Vm.T: even-indexed elements of Vn then Vm.
                                                                                                                    // Vd[i]=Vn[2i] for i in 0..n/2; Vd[n/2+i]=Vm[2i]. n = 8 or 16 bytes.
                                                                                                                    // When rd aliases a source (gcc emits rd==rn ubiquitously), the
                                                                                                                    // writes must not corrupt bytes the loop still reads from that
                                                                                                                    // source — snapshot to scratch first.
                                                                                                                    let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
                                                                                                                    let rn_src = permute_source(buf, rd, rn, false); // read-from offset
                                                                                                                    let rm_src = permute_source(buf, rd, rm, true);
                                                                                                                    let n: i32 = if q { 16 } else { 8 };
                                                                                                                    let es = esize as i32;
                                                                                                                    for i in 0..(n / (2 * es)) {
                                                                                                                        let ei = 2 * i * es;
                                                                                                                        // Vd[i] = Vn[2i]
                                                                                                                        match esize {
                                                                                                                            8 => { buf.mov_load64(RAX, RBX, rn_src + ei); buf.mov_store64(RBX, slot(rd as i32) + i * es, RAX); }
                                                                                                                            4 => { buf.mov_load32(RAX, RBX, rn_src + ei); buf.mov_store32(RBX, slot(rd as i32) + i * es, RAX); }
                                                                                                                            2 => { buf.mov_load32(RAX, RBX, rn_src + ei); buf.mov_store16(RBX, slot(rd as i32) + i * es, RAX); }
                                                                                                                            _ => { buf.mov_load32(RAX, RBX, rn_src + ei); buf.mov_store8(RBX, slot(rd as i32) + i * es, RAX); }
                                                                                                                        }
                                                                                                                        // Vd[n/2 + i] = Vm[2i]
                                                                                                                        match esize {
                                                                                                                                            8 => { buf.mov_load64(RAX, RBX, rm_src + ei); buf.mov_store64(RBX, slot(rd as i32) + (n / (2*es) + i) * es, RAX); },
                                                                                                                                            4 => { buf.mov_load32(RAX, RBX, rm_src + ei); buf.mov_store32(RBX, slot(rd as i32) + (n / (2*es) + i) * es, RAX); },
                                                                                                                                            2 => { buf.mov_load32(RAX, RBX, rm_src + ei); buf.mov_store16(RBX, slot(rd as i32) + (n / (2*es) + i) * es, RAX); },
                                                                                                                                            _ => { buf.mov_load32(RAX, RBX, rm_src + ei); buf.mov_store8(RBX, slot(rd as i32) + (n / (2*es) + i) * es, RAX); },
                                                                                                                        }
                                                                                                                    }
                                                                                                                    Ok(())
                                                                                                        }
Inst::SimdUz2 { rd, rn, rm, esize, q } => {
            // uzp2 Vd.T, Vn.T, Vm.T: ODD-indexed elements of Vn then Vm.
            // Vd[i]=Vn[2i+1] for i in 0..n/2; Vd[n/2+i]=Vm[2i+1]. n = 8 or 16 bytes.
            // (gcc magic-division reducer gathers product-HIGH words with this.)
            // rd aliasing a source (gcc emits rd==rn) must not clobber reads.
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            for i in 0..(n / (2 * es)) {
                let oi = (2 * i + 1) * es;
                match esize {
                    8 => { buf.mov_load64(RAX, RBX, rn_src + oi); buf.mov_store64(RBX, slot(rd as i32) + i * es, RAX); }
                    4 => { buf.mov_load32(RAX, RBX, rn_src + oi); buf.mov_store32(RBX, slot(rd as i32) + i * es, RAX); }
                    2 => { buf.mov_load32(RAX, RBX, rn_src + oi); buf.mov_store16(RBX, slot(rd as i32) + i * es, RAX); }
                    _ => { buf.mov_load32(RAX, RBX, rn_src + oi); buf.mov_store8(RBX, slot(rd as i32) + i * es, RAX); }
                }
                let n2 = n / (2 * es);
                match esize {
                                    8 => { buf.mov_load64(RAX, RBX, rm_src + oi); buf.mov_store64(RBX, slot(rd as i32) + (n2 + i) * es, RAX); },
                                    4 => { buf.mov_load32(RAX, RBX, rm_src + oi); buf.mov_store32(RBX, slot(rd as i32) + (n2 + i) * es, RAX); },
                                    2 => { buf.mov_load32(RAX, RBX, rm_src + oi); buf.mov_store16(RBX, slot(rd as i32) + (n2 + i) * es, RAX); },
                                    _ => { buf.mov_load32(RAX, RBX, rm_src + oi); buf.mov_store8(RBX, slot(rd as i32) + (n2 + i) * es, RAX); },
                }
            }
            Ok(())
}
Inst::SimdZip1 { rd, rn, rm, esize, q } => {
            // zip1 Vd.T, Vn.T, Vm.T: interleave lower halves. Vd[2k]=Vn[k] and
            // Vd[2k+1]=Vm[k] for k in 0..(n/2), n = 8 (q=0) / 16 (q=1) bytes,
            // element size = es (1,2,4,8). Source element k at Vn[+]k*es and
            // Vm[k*es]; dest at Vd[2k*es] / Vd[(2k+1)*es].
            // rd aliasing a source must not clobber reads mid-permute.
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            for k in 0..(n / (2 * es)) {
                let so = k * es; // source element k byte offset (Vn & Vm)
                let d0 = 2 * k * es; // dest element 2k
                let d1 = (2 * k + 1) * es; // dest element 2k+1
                match esize {
                    8 => {
                        buf.mov_load64(RAX, RBX, rn_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load64(RAX, RBX, rm_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d1, RAX);
                    }
                    4 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d1, RAX);
                    }
                    2 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d1, RAX);
                    }
                    _ => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d1, RAX);
                    }
                }
            }
            Ok(())
}
Inst::SimdTrn1 { rd, rn, rm, esize, q } => {
            // trn1 Vd.T, Vn.T, Vm.T: transpose even lanes. For k in 0..(N/2):
            // Vd[2k]=Vn[2k] (even element of Vn), Vd[2k+1]=Vm[2k]. N = n/es
            // elements per vector. No half-length split (unlike zip1): source
            // index is 2k (not k). Guard rd aliasing a source.
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            for k in 0..(n / (2 * es)) {
                let so = 2 * k * es;      // even source element byte offset
                let d0 = 2 * k * es;
                let d1 = (2 * k + 1) * es;
                match esize {
                    8 => {
                        buf.mov_load64(RAX, RBX, rn_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load64(RAX, RBX, rm_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d1, RAX);
                    }
                    4 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d1, RAX);
                    }
                    2 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d1, RAX);
                    }
                    _ => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d1, RAX);
                    }
                }
            }
            Ok(())
}
Inst::SimdTrn2 { rd, rn, rm, esize, q } => {
            // trn2 Vd.T, Vn.T, Vm.T: transpose odd lanes. For k in 0..(N/2):
            // Vd[2k]=Vn[2k+1] (odd element), Vd[2k+1]=Vm[2k+1].
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            for k in 0..(n / (2 * es)) {
                let so = (2 * k + 1) * es; // odd source element byte offset
                let d0 = 2 * k * es;
                let d1 = (2 * k + 1) * es;
                match esize {
                    8 => {
                        buf.mov_load64(RAX, RBX, rn_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load64(RAX, RBX, rm_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d1, RAX);
                    }
                    4 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d1, RAX);
                    }
                    2 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d1, RAX);
                    }
                    _ => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d1, RAX);
                    }
                }
            }
            Ok(())
}
Inst::SimdZip2 { rd, rn, rm, esize, q } => {
            // zip2 Vd.T, Vn.T, Vm.T: interleave the UPPER halves.
            // Vd[2k]=Vn[n/2+k] and Vd[(2k+1)]=Vm[n/2+k] for k in 0..(n/2),
            // n = 8 (q=0) / 16 (q=1) bytes, element size = es.
            // rd aliasing a source must not clobber reads mid-permute.
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let slot = |r: i32| crate::jit::VECTOR_BASE + r * 16;
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            for k in 0..(n / (2 * es)) {
                let so = (n / 2) + k * es; // source element n/2+k byte offset
                let d0 = 2 * k * es;
                let d1 = (2 * k + 1) * es;
                match esize {
                    8 => {
                        buf.mov_load64(RAX, RBX, rn_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load64(RAX, RBX, rm_src + so);
                        buf.mov_store64(RBX, slot(rd as i32) + d1, RAX);
                    }
                    4 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store32(RBX, slot(rd as i32) + d1, RAX);
                    }
                    2 => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store16(RBX, slot(rd as i32) + d1, RAX);
                    }
                    _ => {
                        buf.mov_load32(RAX, RBX, rn_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d0, RAX);
                        buf.mov_load32(RAX, RBX, rm_src + so);
                        buf.mov_store8(RBX, slot(rd as i32) + d1, RAX);
                    }
                }
            }
            Ok(())
}
Inst::SimdAddD { rd, rn, rm, sub } => {
            // add/sub Vd.2D, Vn.2D, Vm.2D: two 64-bit lanes.
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            for i in 0..2i32 {
                let off = i * 8;
                buf.mov_load64(RAX, RBX, slot(rn) + off);
                buf.mov_load64(RCX, RBX, slot(rm) + off);
                if sub { buf.sub_rr64(RAX, RCX); } else { buf.add_rr64(RAX, RCX); }
                buf.mov_store64(RBX, slot(rd) + off, RAX);
            }
            Ok(())
}
Inst::SimdAddB { rd, rn, rm, sub, q } => {
            // add/sub Vd.16b, Vn.16b, Vm.16b (or 8b): byte-lane via paddb/psubb.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.movdqu_load(RAX, RBX, vslot(rn));
            buf.movdqu_load(RCX, RBX, vslot(rm));
            if sub { buf.psubb(RAX, RCX); } else { buf.paddb(RAX, RCX); }
            buf.movdqu_store(RBX, vslot(rd), RAX);
            Ok(())
}
Inst::SimdMaxMinP { rd, rn, rm, min, unsigned, esize, q } => {
            // smaxp/sminp/umaxp/uminp Vd.T,Vn.T,Vm.T: halves of Vd = pairwise
            // reduce Vn (low half), then Vm. esize bytes per lane; signed/unsigned
            // comparison; per-adjacent-pair max/min into dst lane.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let es = esize as i32;
            let lanes = if q { 16 / es } else { 8 / es };
            // snapshot sources in case rd aliases rn/rm (gcc emits smaxp v30,v31,v30)
            let src_rn = permute_source(buf, rd, rn, false);
            let src_rm = permute_source(buf, rd, rm, false);
            let mut d = 0;
            for base in [src_rn, src_rm] {
                for i in 0..lanes / 2 {
                    // load pair (2i, 2i+1), choose max/min
                    let lo = base + (2 * i) * es;
                    let hi = lo + es;
                    // load each into RAX/RCX extended by sign or zero
                    match es {
                        1 => {
                            if unsigned { buf.movzx_byte_mem(RAX, RBX, lo); buf.movzx_byte_mem(RCX, RBX, hi); }
                            else { buf.movsx_byte_mem(RAX, RBX, lo); buf.movsx_byte_mem(RCX, RBX, hi); }
                        }
                        2 => {
                            if unsigned { buf.movzx_word_mem(RAX, RBX, lo); buf.movzx_word_mem(RCX, RBX, hi); }
                            else { buf.movsx_word_mem(RAX, RBX, lo); buf.movsx_word_mem(RCX, RBX, hi); }
                        }
                        _ => {
                            buf.mov_load32(RAX, RBX, lo);
                            if !unsigned { buf.movsxd_r64_r32(RAX, RAX); }
                            buf.mov_load32(RCX, RBX, hi);
                            if !unsigned { buf.movsxd_r64_r32(RCX, RCX); }
                        }
                    }
                    buf.cmp_rr64(RAX, RCX);
                    // signed: L=0x4C (RAX<RCX), G=0x4F; unsigned: B=0x42, A=0x47
                    let (lt, gt) = if unsigned { (0x42u8, 0x47u8) } else { (0x4c, 0x4f) };
                    if min {
                        buf.cmov_rr64(gt, RAX, RCX); // RAX=RCX if RAX>RCX (lo>hi: hi is smaller)
                    } else {
                        buf.cmov_rr64(lt, RAX, RCX); // RAX=RCX if RAX<RCX (hi is larger)
                    }
                    let dst = vslot(rd) + d * es;
                    match es {
                        1 => buf.mov_store8(RBX, dst, RAX),
                        2 => buf.mov_store16(RBX, dst, RAX),
                        _ => buf.mov_store32(RBX, dst, RAX),
                    }
                    d += 1;
                }
            }
            Ok(())
}
Inst::SimdAddH { rd, rn, rm, sub, q } => {
            // add/sub Vd.8h, Vn.8h, Vm.8h (or 4h): 16-bit halfword lanes via paddw/psubw.
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            buf.movdqu_load(RAX, RBX, vslot(rn));
            buf.movdqu_load(RCX, RBX, vslot(rm));
            if sub { buf.psubw(RAX, RCX); } else { buf.paddw(RAX, RCX); }
            // for q=0 (.4h) only the low 8 bytes are legal; still write the full 128b
            // (the guest doesn't rely on the upper half of a .4h result).
            buf.movdqu_store(RBX, vslot(rd), RAX);
            Ok(())
}
Inst::SimdMovEl { rd, rn, esize, index, signed, is_x } => {
            // umov/smov Rd, Vn.bits[idx]: load esize-byte element at offset
            // index*esize, extend zero (umov) or sign (smov) into GPR rd.
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16 + (index as i32) * (esize as i32);
            match (esize, signed, is_x) {
                (8, _, _) => buf.mov_load64(RAX, RBX, src),
                (4, true, true) => { buf.mov_load32(RAX, RBX, src); buf.movsxd_r64_r32(RAX, RAX); }
                (4, _, _) => buf.mov_load32(RAX, RBX, src),
                (2, true, _) => buf.movsx_word_mem(RAX, RBX, src),
                (2, false, _) => buf.movzx_word_mem(RAX, RBX, src),
                (1, true, _) => buf.movsx_byte_mem(RAX, RBX, src),
                (1, false, _) => buf.movzx_byte_mem(RAX, RBX, src),
                    _ => unreachable!("smov/umov esize must be 1/2/4/8"),
                }
            if is_x { buf.mov_store64(RBX, slot(rd as u32), RAX); }
            else { buf.mov_store32(RBX, slot(rd as u32), RAX); }
            Ok(())
}
                                                                                                                                                                                Inst::Addv { rd, rn, size, q } => {
            // ADDV Dd,Vn.T : horizontal sum of sign-extended elements.
            let lanes: u32 = match (size, q) {
                (1, false) => 8,
                (1, true) => 16,
                (2, false) => 4,
                (2, true) => 8,
                (4, true) => 4,
                _ => return Err(format!("Addv unsupported size={} q={}", size, q)),
            };
            let vsrc = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            buf.xor_rr64(RDI, RDI); // accumulator
            for i in 0..lanes {
                let off = vsrc + (i as i32) * (size as i32);
                match size {
                    1 => buf.movsx_byte_mem(RAX, RBX, off),
                    2 => buf.movsx_word_mem(RAX, RBX, off),
                    4 => {
                        buf.mov_load32(RAX, RBX, off);
                        buf.movsxd_r64_r32(RAX, RAX);
                    }
                    _ => unreachable!(),
                }
                buf.add_rr64(RDI, RAX);
            }
            let vdst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            // ADDV <Bd/Hd/Sd>, <Vn>.T writes ONLY the low element of the
            // destination and CLEARS the remaining bits of the register (ARM
            // "add across vector, result to scalar" semantics). The caller may
            // then read the full register as an integer (gcc emits `addv b`;
            // `fmov xD, dN`), so every byte the scalar result does not write
            // MUST be zeroed — leaving the source lanes there fed a polluted
            // 64-bit value into the read (e.g. popcount + stale lane bytes).
            match size {
                1 => {
                    // x86 64-bit store of the 8-bit sum also zeroes bytes 1..7;
                    // mask to a byte first (lanes are signed, so the raw sum can
                    // be negative and RDI would otherwise sign-extend up top).
                    buf.and_ri64(RDI, 0xff);
                    buf.mov_store64(RBX, vdst, RDI);
                }
                2 => {
                    buf.and_ri64(RDI, 0xffff);
                    buf.mov_store64(RBX, vdst, RDI);
                }
                4 => {
                    buf.mov_store32(RBX, vdst, RDI); // 32-bit store zeroes upper 32
                }
                _ => unreachable!(),
            }
            Ok(())
        }
        Inst::SimdReduceMinMax { rd, rn, size, signed, is_min, q } => {
            // SMINV/SMAXV/UMINV/UMAXV Sd/Hd/Bd, Vn.T: reduce min/max over ALL
            // lanes to the bottom scalar. RAX = each lane (sign/zero-extended to
            // 64), RDX = running extrema (init to lane 0), CMOVcc per later lane.
            // cc: signed min < (JL 0x0C), signed max > (JG 0x0F);
            //     unsigned min < (JB 0x02), unsigned max > (JA 0x07).
            let lanes: u32 = match (size, q) {
                (1, false) => 8,
                (1, true) => 16,
                (2, false) => 4,
                (2, true) => 8,
                (4, true) => 4,
                _ => return Err(format!("SimdReduceMinMax unsupported size={size} q={q}")),
            };
            let cc = if is_min {
                if signed { 0x4F } else { 0x47 } // CMOVG / CMOVA (update when candidate < current)
            } else {
                if signed { 0x4C } else { 0x42 } // CMOVL / CMOVB (update when candidate > current)
            };
            let vsrc = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let load = |buf: &mut CodeBuf, off: i32| match size {
                1 => {
                    if signed { buf.movsx_byte_mem(RAX, RBX, off) } else { buf.movzx_byte_mem(RAX, RBX, off) }
                }
                2 => {
                    if signed { buf.movsx_word_mem(RAX, RBX, off) } else { buf.movzx_word_mem(RAX, RBX, off) }
                }
                4 => {
                    buf.mov_load32(RAX, RBX, off);
                    if signed { buf.movsxd_r64_r32(RAX, RAX); }
                }
                _ => unreachable!(),
            };
            load(buf, vsrc);
            buf.mov_rr64(RDX, RAX); // running extrema = lane 0
            for i in 1..lanes {
                let off = vsrc + (i as i32) * (size as i32);
                load(buf, off);
                // cmp RAX(new) vs RDX(cur): RAX < RDX for min / RAX > RDX for max.
                buf.cmp_rr64(RDX, RAX);
                buf.cmov_rr64(cc, RDX, RAX);
            }
            let vdst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            match size {
                1 => buf.mov_store8(RBX, vdst, RDX),   // low byte; upper bits past it
                2 => buf.mov_store16(RBX, vdst, RDX),
                _ => buf.mov_store32(RBX, vdst, RDX), // 32-bit store zeroes upper 32
            }
            Ok(())
        }
        Inst::SimdPairAddD { rd, rn, unsigned: _u } => {
            // ADDP Dd, Vn.2D : pairwise-add the two 64-bit lanes of Vn into the
            // low 64 bits of Vd. Plain 64-bit add (signed/unsigned same result).
            let vsrc = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let vdst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            buf.mov_load64(RAX, RBX, vsrc);        // lane 0
            buf.mov_load64(RCX, RBX, vsrc + 8);    // lane 1
            buf.add_rr64(RAX, RCX);
            buf.mov_store64(RBX, vdst, RAX);       // high 64 bits of Vd left as-is
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
                    Inst::SimdCmTest { rd, rn, rm, lanes, esize } => {
                        // cmtst Vd.T, Vn.T, Vm.T (wall 0x4e208c01): each element is all-ones
                        // iff (Vn[i] & Vm[i]) != 0, else 0. Load element, test Vn&Vm != 0, cmov.
                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                        for i in 0..lanes {
                            let off = (i as i32) * (esize as i32);
                            if esize == 8 {
                                buf.mov_load64(RAX, RBX, slot(rn) + off);
                                buf.mov_load64(RCX, RBX, slot(rm) + off);
                            } else {
                                buf.mov_load32(RAX, RBX, slot(rn) + off);
                                buf.mov_load32(RCX, RBX, slot(rm) + off);
                            }
                            buf.test_rr64(RAX, RCX); // ZF=1 if (Vn&Vm)==0
                            buf.mov_ri64(RDI, 0);
                            buf.mov_ri64(RDX, 0xffff_ffff_ffff_ffff);
                            buf.cmov_rr64(0x45, RDI, RDX); // 0x45=cmovne: ones if nonzero
                            match esize {
                                8 => buf.mov_store64(RBX, slot(rd) + off, RDI),
                                4 => buf.mov_store32(RBX, slot(rd) + off, RDI),
                                2 => buf.mov_store16(RBX, slot(rd) + off, RDI),
                                _ => buf.mov_store8(RBX, slot(rd) + off, RDI),
                            }
                        }
                        Ok(())
                    }
                    Inst::Tbl { rd, rn, rm, tbx, n_tables } => {
                    // tbl vd.16b, {vn..vn+nt-1}, vm: vd[i] = concat(vn..vn+nt)[vm[i]].
                    // The n_tables registers are stored CONTIGUOUSLY (16-byte stride) at
                    // VECTOR_BASE+rn*16 .. +16*n_tables, so concatenated byte `idx` lives at
                    // slot(rn)+idx. idx>=16*n_tables => 0 (tbl) or keep old (tbx).
                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let tbytes = 16 * (n_tables as i32);
                    for i in 0..16i32 {
                        buf.movzx_byte_mem(RAX, RBX, slot(rm) + i); // idx = vm[i]
                        // candidate = concat-table byte at offset idx
                        buf.mov_ri64(R10, slot(rn) as u64);
                        buf.add_rr64(R10, RBX);
                        buf.add_rr64(R10, RAX);
                        buf.movzx_byte_mem(RCX, R10, 0);
                        // default: 0 (tbl) or keep (tbx)
                        buf.mov_ri64(RDX, 0);
                        if tbx { buf.movzx_byte_mem(RDX, RBX, slot(rd) + i); }
                        // select table value only when idx < 16*n_tables (unsigned below)
                        buf.mov_ri64(RDI, tbytes as u64);
                        buf.cmp_rr64(RAX, RDI);
                        buf.cmov_rr64(0x42, RDX, RCX); // cmovb: RDX=RCX if idx<tbytes
                        buf.mov_store8(RBX, slot(rd) + i, RDX);
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
                Inst::SaturatNarrow { rd, rn, dst_esize, src_signed, dst_signed, q } => {
                    // sqxtn/uqxtn/sqxtun/uqxtun Vd.T, Vn.U: saturating narrow. Each src
                    // element (2*dst_esize) is sign/zero-extended to 64, clamped into the
                    // dst range, then the low dst_esize bytes stored into V[rd] lane.
                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let src_esize = 2 * (dst_esize as i32);
                    let lanes = if q { 16 / (dst_esize as i32) } else { 8 / (dst_esize as i32) };
                    let maxv: i64 = if dst_signed {
                        if dst_esize == 2 { 0x7fff } else { 0x7f }
                    } else if dst_esize == 2 { 0xffff } else { 0xff };
                    let minv: i64 = if dst_signed {
                        if dst_esize == 2 { -0x8000 } else { -0x80 }
                    } else { 0 };
                    for i in 0..lanes {
                        let src_off = (i as i32) * src_esize;
                        let dst_off = (i as i32) * (dst_esize as i32);
                        match src_esize {
                            2 => {
                                if src_signed { buf.movsx_word_mem(RAX, RBX, slot(rn)+src_off); }
                                else { buf.movzx_word_mem(RAX, RBX, slot(rn)+src_off); }
                            }
                            _ => {
                                buf.mov_load32(RAX, RBX, slot(rn)+src_off);
                                if src_signed { buf.shl_ri8(RAX, 32); buf.sar_ri8(RAX, 32); }
                            }
                        }
                        // clamp low: RAX = max(RAX, minv) using signed compare
                        buf.mov_ri64(RCX, minv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4c, RAX, RCX); // cmovl: RAX=RCX(minv) if RAX<RCX
                        // clamp high: RAX = min(RAX, maxv)
                        buf.mov_ri64(RCX, maxv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4f, RAX, RCX); // cmovg: RAX=RCX(maxv) if RAX>RCX
                        // store low dst_esize bytes
                        match dst_esize {
                            2 => buf.mov_store16(RBX, slot(rd)+dst_off, RAX),
                            _ => buf.mov_store8(RBX, slot(rd)+dst_off, RAX),
                        }
                    }
                    // Q=0 zero the high 64 bits of Vd
                    if !q {
                        buf.mov_ri64(RAX, 0);
                        buf.mov_store64(RBX, slot(rd) + 8, RAX);
                    }
                    Ok(())
                }
                Inst::SatNarrowShift { rd, rn, src_esize, dst_esize, shift, src_signed, dst_signed, q } => {
                    // sqshrn/uqshrn/sqshrun Vd.T, Vn.T, #imm: shift each src element
                    // (width src_esize) right by `shift` (arith if src_signed else
                    // logical), then saturate narrow to dst_esize (src_esize/2).
                    // SELF-ALIAS (gcc emits sqshrn v31,v31 in narrowing loops): the
                    // dest bytes overlap the source bytes, so snapshot the source
                    // to scratch when rd==rn (permute_source).
                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let src = permute_source(buf, rd, rn, false);
                    let se = src_esize as i32;
                    let de = dst_esize as i32;
                    let lanes = if q { 16 / de } else { 8 / de };
                    // clamp bounds for the DST (dst_esize bytes)
                    let maxv: i64 = if dst_signed {
                        if de == 2 { 0x7fff } else { 0x7f }
                    } else if de == 2 { 0xffff } else { 0xff };
                    let minv: i64 = if dst_signed {
                        if de == 2 { -0x8000 } else { -0x80 }
                    } else { 0 };
                    for i in 0..lanes {
                        let src_off = (i as i32) * se;
                        let dst_off = (i as i32) * de;
                        // load src element, sign/zero-extend to 64
                        match se {
                            2 => {
                                if src_signed { buf.movsx_word_mem(RAX, RBX, src+src_off); }
                                else { buf.movzx_word_mem(RAX, RBX, src+src_off); }
                            }
                            _ => {
                                buf.mov_load32(RAX, RBX, src+src_off);
                                if src_signed { buf.shl_ri8(RAX, 32); buf.sar_ri8(RAX, 32); }
                            }
                        }
                        // right-shift: arith for signed src, logical for unsigned
                        if src_signed { buf.sar_ri8(RAX, shift); } else { buf.shr_ri8(RAX, shift); }
                        // clamp low
                        buf.mov_ri64(RCX, minv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4c, RAX, RCX); // RAX=minv if RAX<minv
                        // clamp high
                        buf.mov_ri64(RCX, maxv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4f, RAX, RCX); // RAX=maxv if RAX>maxv
                        match de {
                            2 => buf.mov_store16(RBX, slot(rd)+dst_off, RAX),
                            _ => buf.mov_store8(RBX, slot(rd)+dst_off, RAX),
                        }
                    }
                    if !q {
                        buf.mov_ri64(RAX, 0);
                        buf.mov_store64(RBX, slot(rd) + 8, RAX);
                    }
                    Ok(())
                }
                Inst::Ld1V { rd, rn, bytes } => {
                    // ld1 {Vt.T}, [Xn], #imm: load `bytes` (16 or 8) contiguous bytes
                    // from guest address x[rn] into V[rd], then x[rn] += bytes.
                    let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                    ldg(buf, RDX, rn as u32); // RDX = host ptr to guest mem
                    if bytes == 16 {
                        buf.movdqu_load(0, RDX, 0);
                        buf.movdqu_store(RBX, slot, 0);
                    } else {
                        buf.movq_load(0, RDX, 0);
                        buf.movq_store(RBX, slot, 0);
                        buf.mov_ri64(RAX, 0);
                        buf.mov_store64(RBX, slot + 8, RAX); // zero high 64
                    }
                    ldg(buf, RAX, rn as u32);
                    buf.add_ri64(RAX, bytes as u32); // post-index: x[rn] += bytes
                    stg(buf, rn as u32, RAX);
                    Ok(())
                }
                Inst::St1V { rd, rn, bytes } => {
                    // st1 {Vt.T}, [Xn], #imm: store V[rd]'s `bytes` to guest
                    // address x[rn], then x[rn] += bytes.
                    let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                    ldg(buf, RDX, rn as u32); // RDX = host ptr
                    if bytes == 16 {
                        buf.movdqu_load(0, RBX, slot);
                        buf.movdqu_store(RDX, 0, 0);
                    } else {
                        buf.movq_load(0, RBX, slot);
                        buf.movq_store(RDX, 0, 0);
                    }
                    ldg(buf, RAX, rn as u32);
                    buf.add_ri64(RAX, bytes as u32); // post-index
                    stg(buf, rn as u32, RAX);
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
                Inst::Ld1L { rd, rn, esize, index } => {
                    // ld1 {Vt.T}[idx], [Xn]: load `esize` bytes from [x[rn]]
                    // into lane `index` of Vd (byte offset index*esize).
                    let slot = crate::jit::VECTOR_BASE + (rd as i32) * 16;
                    ldg(buf, RDX, rn as u32); // RDX = base address (host ptr)
                    match esize {
                        8 => buf.mov_load64(RAX, RDX, 0),
                        4 => buf.mov_load32(RAX, RDX, 0),
                        2 => buf.movzx_word_mem(RAX, RDX, 0),
                        _ => buf.movzx_byte_mem(RAX, RDX, 0),
                    }
                    let off = slot + (index as i32) * (esize as i32);
                    match esize {
                        8 => buf.mov_store64(RBX, off, RAX),
                        4 => buf.mov_store32(RBX, off, RAX),
                        2 => buf.mov_store16(RBX, off, RAX),
                        _ => buf.mov_store8(RBX, off, RAX),
                    }
                    Ok(())
                }
                Inst::SimdXtl { rd, rn, sign, esrc, upper } => {
                    // uxtl/sxtl Vd.<long>, Vn.<short>: widen each esrc-byte lane
                    // to a (esrc*2)-byte lane (zero/sign extend). Lanes = 8/esrc,
                    // the dest occupies the full 16-byte vector (Q=1 long form).
                    // IN-PLACE ALIASING: when rd==rn the widened write of lane i
                    // (2*esrc bytes at i*2*esrc) overlaps the narrow source bytes
                    // of later lanes (esrc bytes at (i+1)*esrc), so a naive
                    // read-then-write loop clobbers the still-needed source — e.g.
                    // gcc's `sxtl v30.2d, v30.2s` (rd==rn) dropped lane 1. Snapshot
                    // Vn to the permscratch slot first when they alias.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    // upper (sxtl2/uxtl2): the narrow src lanes live in the
                    // UPPER 64 bits of Vn (byte 8..15), like saddw2.
                    let n_half: i32 = if upper { 8 } else { 0 };
                    let srcbase = permute_source(buf, rd, rn, false);
                    let lanes = 8usize >> esrc.trailing_zeros() as usize;
                    for i in 0..lanes {
                        let src = srcbase + n_half + (i as i32) * (esrc as i32);
                        let dst = vslot(rd) + (i as i32) * (esrc as i32) * 2;
                        match (esrc, sign) {
                            (1, false) => buf.movzx_byte_mem(RAX, RBX, src),
                            (1, true) => buf.movsx_byte_mem(RAX, RBX, src),
                            (2, false) => buf.movzx_word_mem(RAX, RBX, src),
                            (2, true) => buf.movsx_word_mem(RAX, RBX, src),
                            (4, false) => buf.mov_load32(RAX, RBX, src),
                            (4, true) => buf.mov_load32(RAX, RBX, src),
                            _ => unreachable!(),
                        }
                        if esrc == 4 && sign {
                            buf.movsxd_r64_r32(RAX, RAX);
                        }
                        match (esrc as i32) * 2 {
                            4 => { buf.mov_store32(RBX, dst, RAX); }
                            8 => { buf.mov_store64(RBX, dst, RAX); }
                            _ => { buf.mov_store16(RBX, dst, RAX); }
                        }
                    }
                    Ok(())
                }
                Inst::SimdAddw { rd, rn, rm, sign, esrc, upper, sub } => {
                    // uaddw/saddw Vd.T, Vn.T, Vm.(T/2): Vd[i] = Vn[i] + extend(Vm_hi)
                    // narrow source element = esrc bytes, dest element = 2*esrc.
                    // `upper` (saddw2/uaddw2): the narrow src is the UPPER 64 bits
                    // of Vm (byte 8..15), not the lower — a second loop pass
                    // accumulates the other half. `sub` (ssubw/usubw, bit13):
                    // Vd[i] = Vn[i] - extend(Vm[i]) -- the add/sub-wide family.
                    // IN-PLACE ALIASING: when the narrow source rm aliases rd, the
                    // widened (2*esrc) write of lane i at i*2*esrc overwrites the
                    // narrow msrc bytes of lane i+1 (at (i+1)*esrc), so a naive
                    // read-then-write loop clobbers the still-needed source — e.g.
                    // gcc's `saddw v31.2d, v29.2d, v31.2s` (rd==rm) dropped the
                    // second msrc lane and summed [1,0] instead of [2]. Snapshot
                    // the source(s) that alias rd to permscratch first.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let der = (esrc as i32) * 2;      // dest element width
                    let lanes = 8usize >> esrc.trailing_zeros() as usize;
                    let m_half: i32 = if upper { 8 } else { 0 };
                    // Snapshot into scratch slot A (rn) / B (rm) when they alias rd.
                    let nbase = if rn == rd { permute_source(buf, rd, rn, false) } else { vslot(rn) };
                    let mbase = if rm == rd { permute_source(buf, rd, rm, true) } else { vslot(rm) };
                    for i in 0..lanes {
                        let nsrc = nbase + (i as i32) * der;   // Vn wide elem
                        let msrc = mbase + m_half + (i as i32) * (esrc as i32);
                        let dst = vslot(rd) + (i as i32) * der;
                        match der {
                            4 => buf.mov_load32(RAX, RBX, nsrc),
                            8 => buf.mov_load64(RAX, RBX, nsrc),
                            _ => buf.mov_load32(RAX, RBX, nsrc),
                        }
                        match (esrc, sign) {
                            (1, false) => buf.movzx_byte_mem(RCX, RBX, msrc),
                            (1, true) => buf.movsx_byte_mem(RCX, RBX, msrc),
                            (2, false) => buf.movzx_word_mem(RCX, RBX, msrc),
                            (2, true) => buf.movsx_word_mem(RCX, RBX, msrc),
                            (4, false) => buf.mov_load32(RCX, RBX, msrc),
                            (4, true) => buf.mov_load32(RCX, RBX, msrc),
                            _ => unreachable!(),
                        }
                        if esrc == 4 && sign {
                            buf.movsxd_r64_r32(RCX, RCX);
                        }
                        if sub {
                            buf.sub_rr64(RAX, RCX);
                        } else {
                            buf.add_rr64(RAX, RCX);
                        }
                        match der {
                            4 => buf.mov_store32(RBX, dst, RAX),
                            8 => buf.mov_store64(RBX, dst, RAX),
                            _ => buf.mov_store16(RBX, dst, RAX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdVLog { rd, rn, rm, op } => {
                    // and/orr/eor/bic Vd.128 = Vn.128 op Vm.128 (Q selects 8/16B,
                    // translate always on the full 16-byte slot).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    buf.movdqu_load(RAX, RBX, vslot(rn));
                    buf.movdqu_load(RCX, RBX, vslot(rm));
                    match op {
                        0 => buf.pand(RAX, RCX),       // AND
                        1 => buf.pxor_xmm(RAX, RCX),   // EOR
                        2 => buf.por(RAX, RCX),        // ORR
                        _ => { buf.pandn(RCX, RAX); buf.movdqu_store(RBX, vslot(rd), RCX); return Ok(()); } // BIC: xmm1 = ~v1 & v0
                    }
                    buf.movdqu_store(RBX, vslot(rd), RAX);
                    Ok(())
                }
                Inst::SimdNot { rd, rn } => {
                    // mvn Vd.16B/8B, Vn: bitwise NOT of the full 16-byte slot.
                    // RAX = Vn; RCX = all-ones; RAX = RAX ^ RCX.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    buf.movdqu_load(RAX, RBX, vslot(rn));
                    buf.movdqu_ones(RCX);
                    buf.pxor_xmm(RAX, RCX);
                    buf.movdqu_store(RBX, vslot(rd), RAX);
                    Ok(())
                }
                Inst::SimdSel { rd, rn, rm, op } => {
                    // bsl/bit/bif bitwise select between three 128-bit vectors.
                    // BSL: Vd = (Vn & Vd) | (~Vd & Vm).  (op stored as op).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    buf.movdqu_load(RAX, RBX, vslot(rn));
                    buf.movdqu_load(RCX, RBX, vslot(rm));
                    buf.movdqu_load(R10, RBX, vslot(rd));
                    // RAX = Rn & Rd ; R10 = ~Rd & Rm ; OR them
                    match op {
                        0 => {
                            buf.pand(RAX, R10);      // Rn & Rd
                            buf.pandn(R10, RCX);     // ~Rd & Rm
                            buf.por(RAX, R10);       // final
                        }
                        _ => {
                            buf.pandn(RAX, RCX);     // ~Vm & Rn
                            buf.pand(R10, RCX);      // Vd & Vm
                            buf.por(RAX, R10);
                        }
                    }
                    buf.movdqu_store(RBX, vslot(rd), RAX);
                    Ok(())
                }
                Inst::SimdHighNarrow { rd, rn, rm, dst_esize, sub, round, q } => {
                                        // addhn/subhn/raddhn Vd.T, Vn.W, Vm.W: dst[i] = high half of the
                                        // src-width (Vn[i] +/- Vm[i]), narrowed to dst_esize bytes. Q=1
                                        // (addhn2/raddhn2) uses the UPPER 64 bits of Vn/Vm and writes the
                                        // upper half of Vd; Q=0 uses the lower and writes the lower.
                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                        let src_esize = 2 * (dst_esize as i32);
                                        let lanes = if q { 8 / (dst_esize as i32) } else { 8 / (dst_esize as i32) };
                                        let dst_bits = (8 * (dst_esize as i32)) as u8;
                                        let src_base = 0; // source lanes always start at 0 (full 128-bit read regardless of Q)
                                        let dst_base = if q { 8 } else { 0 }; // Q=1 writes the upper half of Vd
                                        for i in 0..lanes {
                                            let so = src_base + (i as i32) * src_esize;
                                            let de = dst_base + (i as i32) * (dst_esize as i32);
                                            if src_esize >= 4 {
                                                buf.mov_load64(RAX, RBX, slot(rn)+so);
                                                buf.mov_load64(RCX, RBX, slot(rm)+so);
                                            } else {
                                                buf.mov_load32(RAX, RBX, slot(rn)+so);
                                                buf.mov_load32(RCX, RBX, slot(rm)+so);
                                            }
                                            if sub { buf.sub_rr64(RAX, RCX); } else { buf.add_rr64(RAX, RCX); }
                                            if round { let h = (1u64 << (dst_bits - 1)) as u64; buf.mov_ri64(R10, h); buf.add_rr64(RAX, R10); }
                                            buf.shr_ri8(RAX, dst_bits);
                                            match dst_esize {
                                                4 => buf.mov_store32(RBX, slot(rd)+de, RAX),
                                                2 => buf.mov_store16(RBX, slot(rd)+de, RAX),
                                                _ => buf.mov_store8(RBX, slot(rd)+de, RAX),
                                            }
                                        }
                                        // Q=0 writes only the low half of Vd; the upper 8 bytes stay 0.
                                        if !q {
                                            buf.mov_ri64(RAX, 0);
                                            buf.mov_store64(RBX, slot(rd)+8, RAX);
                                        }
                                        Ok(())
                                    }
                                        Inst::Ld2 { rd, rn, q, post, esize } => {
                    // ld2 {Vt, Vt1}, [Xn]: load 2 structure vectors, DEINTERLEAVED.
                    // Memory holds {V0.e0,V1.e0, V0.e1,V1.e1, ...}: element i of
                    // reg j (j in 0..2) is at byte offset i*(2*es) + j*es, es =
                    // element size. Vt[i]=mem[2*i*es], Vt1[i]=mem[2*i*es+es].
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32; // bytes per vector
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32); // RDX = base (host ptr)
                    for j in 0..2i32 {
                        for i in 0..nelems {
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RDX, i * 2 * es + j * es + b);
                                buf.mov_store8(RBX, vslot((rd as i32 + j) as u8) + i * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::St2 { rd, rn, q, post, esize } => {
                    // st2 {Vt, Vt1}, [Xn]: the inverse — store structure vectors
                    // Vt..Vt1 to memory in the deinterleaved {V0.e0,V1.e0,...}
                    // layout (element i of reg j at byte i*2*es + j*es).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32;
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32); // RDX = base (host ptr)
                    for j in 0..2i32 {
                        for i in 0..nelems {
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RBX, vslot((rd as i32 + j) as u8) + i * es + b);
                                buf.mov_store8(RDX, i * 2 * es + j * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                // ---- ld3/st3: structure DEINTERLEAVE (3 registers) ----
                // Memory holds {V0.e0,V1.e0,V2.e0, V0.e1,...}: element i of
                // reg j (j in 0..3) is at byte offset i*(3*es) + j*es.
                Inst::Ld3N { rd, rn, q, post, esize } => {
                    let v = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32;
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32);
                    for j in 0..3i32 {
                        for i in 0..nelems {
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RDX, i * 3 * es + j * es + b);
                                buf.mov_store8(RBX, v((rd as i32 + j) as u8) + i * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::St3N { rd, rn, q, post, esize } => {
                    let v = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32;
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32);
                    for j in 0..3i32 {
                        for i in 0..nelems {
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RBX, v((rd as i32 + j) as u8) + i * es + b);
                                buf.mov_store8(RDX, i * 3 * es + j * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                // ---- ld4/st4: structure DEINTERLEAVE (4 registers) ----
                // Memory holds {V0.e0,V1.e0,V2.e0,V3.e0, V0.e1,...}: element i
                // of reg j (j in 0..4) is at byte offset i*(4*es) + j*es, es =
                // element size in bytes.
                Inst::Ld4N { rd, rn, q, post, esize } => {
                    let v = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32; // bytes per vector
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32);
                    for j in 0..4i32 {
                        for i in 0..nelems {
                            // copy es bytes: mem[i*(4es)+j*es .. +es] -> v[rd+j]+i*es
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RDX, i * 4 * es + j * es + b);
                                buf.mov_store8(RBX, v((rd as i32 + j) as u8) + i * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::St4N { rd, rn, q, post, esize } => {
                    let v = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let total = if q { 16 } else { 8 } as i32;
                    let es = esize as i32;
                    let nelems = total / es;
                    ldg(buf, RDX, rn as u32);
                    for j in 0..4i32 {
                        for i in 0..nelems {
                            for b in 0..es {
                                buf.movzx_byte_mem(RAX, RBX, v((rd as i32 + j) as u8) + i * es + b);
                                buf.mov_store8(RDX, i * 4 * es + j * es + b, RAX);
                            }
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::Ld1N { rd, rn, nreg, q, post } => {
                    // ld1 {Vt, Vt2, ..}, [Xn]: load `nreg` CONSECUTIVE (q?16:8)-byte
                    // blocks of memory into V[rd], V[rd+1], .. (no deinterleave) —
                    // the compiler's array-literal / memcpy idiom.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let block = if q { 16 } else { 8 } as i32;
                    ldg(buf, RDX, rn as u32); // RDX = base host ptr
                    for k in 0..nreg as i32 {
                        let slot = vslot((rd as i32 + k) as u8);
                        if block == 16 {
                            buf.movdqu_load(0, RDX, k * 16);
                            buf.movdqu_store(RBX, slot, 0);
                        } else {
                            buf.movq_load(0, RDX, k * 8);
                            buf.movq_store(RBX, slot, 0);
                            buf.mov_ri64(RAX, 0);
                            buf.mov_store64(RBX, slot + 8, RAX); // zero high u64
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::St1N { rd, rn, nreg, q, post } => {
                    // st1 {Vt, Vt2, ..}, [Xn]: store `nreg` consecutive (q?16:8)-byte
                    // blocks from V[rd], V[rd+1], .. to memory (no deinterleave).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let block = if q { 16 } else { 8 } as i32;
                    ldg(buf, RDX, rn as u32);
                    for k in 0..nreg as i32 {
                        let slot = vslot((rd as i32 + k) as u8);
                        if block == 16 {
                            buf.movdqu_load(0, RBX, slot);
                            buf.movdqu_store(RDX, k * 16, 0);
                        } else {
                            buf.movq_load(0, RBX, slot);
                            buf.movq_store(RDX, k * 8, 0);
                        }
                    }
                    if post != 0 {
                        buf.mov_load64(RAX, RBX, slot(rn as u32));
                        buf.add_ri64(RAX, post as u32);
                        buf.mov_store64(RBX, slot(rn as u32), RAX);
                    }
                    Ok(())
                }
                Inst::SimdShl { rd, rn, esize, shift } => {
                    // shl Vd.T, Vn.T, #imm : left-shift each lane by shift.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let lanes = 16 / (esize as i32);  // 16/esize lanes
                    for i in 0..lanes {
                        let off = vslot(rn) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_load64(RAX, RBX, off),
                            4 => buf.mov_load32(RAX, RBX, off),
                            2 => buf.movzx_word_mem(RAX, RBX, off),
                            _ => buf.movzx_byte_mem(RAX, RBX, off),
                        }
                        buf.shl_ri8(RAX, shift);
                        let dst = vslot(rd) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_store64(RBX, dst, RAX),
                            4 => buf.mov_store32(RBX, dst, RAX),
                            2 => buf.mov_store16(RBX, dst, RAX),
                            _ => buf.mov_store8(RBX, dst, RAX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdSatShl { rd, rn, esize, shift, sat } => {
                    // sqshl/uqshl/sqshlu Vd.T, Vn.T, #imm: left-shift each lane then
                    // saturate. sat: 0=sqshl (signed src/dst), 1=uqshl (unsigned
                    // src/dst), 2=sqshlu (signed src, unsigned dst). Shift arithmetic
                    // on the 64-bit reg after sign/zero-extending the source, then
                    // clamp. Self-alias-safe (dest not re-read).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let lanes = 16 / (esize as i32);
                    let ebits = 8 * (esize as i32);
                    let (maxv, minv): (i64, i64) = match (sat, esize) {
                        (0, 8) => (i64::MAX, i64::MIN),
                        (0, 4) => (0x7fff_ffff, -0x8000_0000),
                        (0, 2) => (0x7fff, -0x8000),
                        (0, 1) => (0x7f, -0x80),
                        (1, 8) => (i64::MAX, 0),
                        (1, 4) => (0xffff_ffff, 0),
                        (1, 2) => (0xffff, 0),
                        (1, 1) => (0xff, 0),
                        (2, 8) => (i64::MAX, 0),
                        (2, 4) => (0xffff_ffff, 0),
                        (2, 2) => (0xffff, 0),
                        _ => (0xff, 0),
                    };
                    let src_signed = sat != 1;
                    for i in 0..lanes {
                        let off = vslot(rn) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_load64(RAX, RBX, off),
                            4 => {
                                buf.mov_load32(RAX, RBX, off);
                                if src_signed { buf.movsxd_r64_r32(RAX, RAX); }
                            }
                            2 => {
                                if src_signed { buf.movsx_word_mem(RAX, RBX, off); }
                                else { buf.movzx_word_mem(RAX, RBX, off); }
                            }
                            _ => {
                                if src_signed { buf.movsx_byte_mem(RAX, RBX, off); }
                                else { buf.movzx_byte_mem(RAX, RBX, off); }
                            }
                        }
                        // shl in the wide 64-bit reg. Because shift <= ebits-1 and the
                        // source was sign/zero-extended, value*2^shift fits i64 (magnitude
                        // <= 2^(2*ebits-1)), so clamping against the element range after
                        // the shift saturates correctly (a negative src that overflows the
                        // lane clamps to min, a positive one to max).
                        buf.shl_ri8(RAX, shift);
                        // clamp
                        buf.mov_ri64(RCX, minv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4c, RAX, RCX); // RAX=minv if RAX<minv
                        buf.mov_ri64(RCX, maxv as u64);
                        buf.cmp_rr64(RAX, RCX);
                        buf.cmov_rr64(0x4f, RAX, RCX); // RAX=maxv if RAX>maxv
                        let dst = vslot(rd) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_store64(RBX, dst, RAX),
                            4 => buf.mov_store32(RBX, dst, RAX),
                            2 => buf.mov_store16(RBX, dst, RAX),
                            _ => buf.mov_store8(RBX, dst, RAX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdShrAcc { rd, rn, esize, shift, unsigned } => {
                    // usra/ssra Vd.T, Vn.T, #imm : Vd_i += Vn_i >> imm (logical if
                    // unsigned/usra, arithmetic if signed/ssra). The source element is
                    // sign-extended to 64 bits for ssra (zero-extending a negative
                    // element made the arithmetic shift positive).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let lanes = 16 / (esize as i32);
                    for i in 0..lanes {
                        let off = vslot(rn) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_load64(RAX, RBX, off),
                            4 => {
                                buf.mov_load32(RAX, RBX, off);
                                if !unsigned { buf.movsxd_r64_r32(RAX, RAX); }
                            }
                            2 => {
                                if unsigned { buf.movzx_word_mem(RAX, RBX, off); }
                                else { buf.movsx_word_mem(RAX, RBX, off); }
                            }
                            _ => {
                                if unsigned { buf.movzx_byte_mem(RAX, RBX, off); }
                                else { buf.movsx_byte_mem(RAX, RBX, off); }
                            }
                        }
                        let esize_bits = (esize as i32) * 8;
                        if (shift as i32) >= esize_bits {
                            if unsigned {
                                buf.xor_rr64(RAX, RAX);
                            } else {
                                buf.sar_ri8(RAX, 63);
                            }
                        } else if unsigned {
                            buf.shr_ri8(RAX, shift);
                        } else {
                            buf.sar_ri8(RAX, shift);
                        }
                        let dst = vslot(rd) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_load64(RCX, RBX, dst),
                            4 => buf.mov_load32(RCX, RBX, dst),
                            2 => buf.movzx_word_mem(RCX, RBX, dst),
                            _ => buf.movzx_byte_mem(RCX, RBX, dst),
                        }
                        buf.add_rr64(RCX, RAX);
                        match esize {
                            8 => buf.mov_store64(RBX, dst, RCX),
                            4 => buf.mov_store32(RBX, dst, RCX),
                            2 => buf.mov_store16(RBX, dst, RCX),
                            _ => buf.mov_store8(RBX, dst, RCX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdShrn { rd, rn, esrc, shift, upper, round } => {
                    // shrn/shrn2/rshrn/rshrn2 Vd.T, Vn.U, #imm: shift each DOUBLE-width
                    // source element (esrc bytes) right by `shift`, truncate (shrn) or
                    // round (rshrn: add 2^(shift-1) before shifting) to the HALF-width
                    // dest element (esrc/2 bytes). shrn writes low/high dest lanes by
                    // `upper`; rshrn rounds. SELF-ALIAS (gcc emits shrn v31,v31) snapshots
                    // the source to scratch so the dest writes don't clobber later reads.
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let src = permute_source(buf, rd, rn, false);
                    let es = esrc as i32;         // source element bytes
                    let ds = (esrc as i32) / 2;   // dest element bytes
                    let src_lanes = 16 / es;      // source elements in 128-bit reg
                    let dst_off = if upper { 8 } else { 0 }; // shrn2 writes high half
                    for i in 0..src_lanes {
                        // load DOUBLE-width source lane, zero-extended
                        match esrc {
                            8 => buf.mov_load64(RAX, RBX, src + i * es),
                            4 => buf.mov_load32(RAX, RBX, src + i * es),
                            2 => buf.movzx_word_mem(RAX, RBX, src + i * es),
                            _ => buf.movzx_byte_mem(RAX, RBX, src + i * es),
                        }
                        if round && (shift as i32) >= 1 {
                            // rshrn: add 1 << (shift-1) to round-half-up
                            buf.add_ri64(RAX, 1u32 << (shift - 1));
                        }
                        if (shift as i32) >= es * 8 {
                            buf.xor_rr64(RAX, RAX);
                        } else {
                            buf.shr_ri8(RAX, shift);
                        }
                        let dst = vslot(rd) + dst_off + i * ds;
                        match ds {
                            4 => buf.mov_store32(RBX, dst, RAX),
                            2 => buf.mov_store16(RBX, dst, RAX),
                            _ => buf.mov_store8(RBX, dst, RAX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdShr { rd, rn, esize, shift, unsigned } => {
                    // ushr/sshr Vd.T, Vn.T, #imm : Vd_i = Vn_i >> shift (logical if
                    // unsigned/ushr, arithmetic if signed/sshr), no accumulate.
                    // For sshr the esize-bit element must be SIGN-extended to 64 bits
                    // before the arithmetic shift (zero-extending a negative element
                    // made it positive); guards shift >= esize*8 (0 for logical,
                    // all-ones sign-fill for arithmetic - a bare x86 imm clamps).
                    let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let lanes = 16 / (esize as i32);
                    let esize_bits = (esize as i32) * 8;
                    for i in 0..lanes {
                        let off = vslot(rn) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_load64(RAX, RBX, off),
                            4 => {
                                buf.mov_load32(RAX, RBX, off);
                                if !unsigned { buf.movsxd_r64_r32(RAX, RAX); }
                            }
                            2 => {
                                if unsigned { buf.movzx_word_mem(RAX, RBX, off); }
                                else { buf.movsx_word_mem(RAX, RBX, off); }
                            }
                            _ => {
                                if unsigned { buf.movzx_byte_mem(RAX, RBX, off); }
                                else { buf.movsx_byte_mem(RAX, RBX, off); }
                            }
                        }
                        if (shift as i32) >= esize_bits {
                            if unsigned {
                                buf.xor_rr64(RAX, RAX);
                            } else {
                                buf.sar_ri8(RAX, 63);
                            }
                        } else if unsigned {
                            buf.shr_ri8(RAX, shift);
                        } else {
                            buf.sar_ri8(RAX, shift);
                        }
                        let dst = vslot(rd) + (i as i32) * (esize as i32);
                        match esize {
                            8 => buf.mov_store64(RBX, dst, RAX),
                            4 => buf.mov_store32(RBX, dst, RAX),
                            2 => buf.mov_store16(RBX, dst, RAX),
                            _ => buf.mov_store8(RBX, dst, RAX),
                        }
                    }
                    Ok(())
                }
                Inst::SimdInsD { rd, rn, dst_idx, src_idx, esize } => {
                    // mov Vd.T[dst], Vn.T[src]: copy one element (esize bytes) lane
                    // between vectors (d 64-bit or s 32-bit lanes).
                    let es = esize as i32;
                    let src = crate::jit::VECTOR_BASE + (rn as i32)*16 + (src_idx as i32)*es;
                    let dst = crate::jit::VECTOR_BASE + (rd as i32)*16 + (dst_idx as i32)*es;
                    if esize == 8 {
                        buf.mov_load64(RAX, RBX, src);
                        buf.mov_store64(RBX, dst, RAX);
                    } else if esize == 4 {
                        buf.mov_load32(RAX, RBX, src);
                        buf.mov_store32(RBX, dst, RAX);
                    } else if esize == 2 {
                        buf.movzx_word_mem(RAX, RBX, src);
                        buf.mov_store16(RBX, dst, RAX);
                    } else {
                        buf.movzx_byte_mem(RAX, RBX, src);
                        buf.mov_store8(RBX, dst, RAX);
                    }
                    Ok(())
                }
                Inst::SimdFmulEl { rd, rn, rm, esize, index, q } => {
                    // fmul Vd.T, Vn.T, Vm.T[L]: each lane of Vd = Vn[lane] * Vm[L].
                    let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                    let es = esize as i32;
                    let n = if q { 16i32 } else { 8i32 };
                    let lanes = n / es;
                    // Broadcast the single element into xmm2 ONCE up front. Must
                    // NOT re-read the element memory each iteration: when rd==rm
                    // (e.g. `fmul v17.4s, v7.4s, v17.s[0]`, rd==rm==17) the first
                    // lane's write clobbers the element before later lanes read
                    // it, corrupting every lane after 0 (silent wrong vector).
                    let mel = f(rm) + (index as i32) * es; // address of Vm[L]
                    if esize == 8 {
                        buf.movq_load(2, RBX, mel);
                    } else {
                        buf.mov_load32(RAX, RBX, mel);
                        buf.movd_xmm_r32(2, RAX);
                    }
                    for l in 0..lanes {
                        // total lane byte offset: lanes may be 2S(8B),4S/2d(16B)
                        let to = l * es;
                        if esize == 8 {
                            buf.movq_load(0, RBX, f(rn) + to);
                            buf.mulsd(0, 2);
                            buf.movq_store(RBX, f(rd) + to, 0);
                        } else {
                            // single-precision lanes
                            buf.mov_load32(RAX, RBX, f(rn) + to);
                            buf.movd_xmm_r32(0, RAX);
                            buf.mulss(0, 2);
                            buf.movd_r32_xmm(RAX, 0);
                            buf.mov_store32(RBX, f(rd) + to, RAX);
                                    }
                                }
                                Ok(())
                            }
                            Inst::SimdRev { rd, rn, granule, q } => {
                                // rev64/rev32 Vd.T, Vn.T: byte-reverse within each
                                // granule (8B for rev64, 4B for rev32) via BSWAP.
                                let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                let base = f(rn);
                                let dbase = f(rd);
                                let total = if q { 16 } else { 8 };
                                let n = total / (granule as i32);
                                for g in 0..n {
                                    let o = g * (granule as i32);
                                    if granule == 8 {
                                        buf.mov_load64(RAX, RBX, base + o);
                                        buf.bswap_r64(RAX);
                                        buf.mov_store64(RBX, dbase + o, RAX);
                                    } else if granule == 4 {
                                        buf.mov_load32(RAX, RBX, base + o);
                                        buf.bswap_r32(RAX);
                                        buf.mov_store32(RBX, dbase + o, RAX);
                                    } else {
                                        // granule == 2 (rev16): byte-swap each
                                        // 16-bit halfword = rotate-left-by-8.
                                        buf.mov_load16(RAX, RBX, base + o);
                                        buf.rol16_ri8(RAX, 8);
                                        buf.mov_store16(RBX, dbase + o, RAX);
                                    }
                                }
                                Ok(())
                            }
                            Inst::SimdLaneS { rd, rn, esize, index } => {
                                // mov Sd/Dd, Vn.T[idx]: copy the element into the
                                // low bytes of the dest FP register slot.
                                let f = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                let src = f(rn) + (index as i32) * (esize as i32);
                                let dst = f(rd);
                                if esize == 8 {
                                    buf.mov_load64(RAX, RBX, src);
                                    buf.mov_store64(RBX, dst, RAX);
                                } else {
                                    buf.mov_load32(RAX, RBX, src);
                                    buf.mov_store32(RBX, dst, RAX);
                                }
                                Ok(())
                            }
                            Inst::Sha { mode, rd, rn, rm } => {
                                // Call the host SHA helper: passes the current
                                // guest-state pointer (RBX) and a packed[op|rd|rn|rm].
                                let packed = ((mode as u64) << 24) | ((rd as u64) << 16)
                                    | ((rn as u64) << 8) | (rm as u64);
                                let addr = crate::jit::guest_sha1stem as usize as u64;
                                buf.mov_rr64(RDI, RBX);   // arg0 = CpuState*
                                buf.mov_ri64(RSI, packed); // arg1 = packed op
                                buf.mov_ri64(RAX, addr);
                                // Align RSP ≡ 0 mod 16 at the host call site
                                // (block body runs at RSP ≡ 8; SysV host calls
                                // need ≡ 0) — same rationale as the Svc arm.
                                buf.sub_ri64(4, 8);
                                buf.call_r64(RAX);
                                buf.add_ri64(4, 8);
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
                buf.zero_ext_r32(RAX);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::Div { rd, rn, rm, signed, is_x } => {
            // udiv/sdiv Rd, Rn, Rm: RAX = Rn / Rm (div/rdiv in RAX/RDX:AX).
            ldg(buf, RAX, rn as u32);        // dividend
            ldg(buf, RCX, rm as u32);        // divisor
            buf.xor_rr64(RDX, RDX);          // clear hi-word for unsigned div
            if signed {
                buf.idiv_r64(RCX);           // signed: use I DIV
            } else if is_x {
                buf.div_r64(RCX);
            } else {
                buf.div_r32(RCX);
            }
            stg(buf, rd as u32, RAX);        // quotient -> Rd
            Ok(())
        }
        Inst::SimdVShift { rd, rn, rm, esize, signed_, q, rounding } => {
            // ushl/sshl Vd.T, Vn.T, Vm.T : per-lane variable shift.
            // Each count lane C is a SIGNED esize-bit value:
            //   C >= 0 -> result = V << C          (left)
            //   C <  0 -> result = V >> -C         (right; sshl = arithmetic, ushl = logical)
            //   |C| >= B (B = esize*8):
            //       left  shift by >= B -> 0
            //       right shift by >= B -> ushl: 0, sshl: sign-fill (= sign bit replicated)
            // The scalar count is masked by x86's `shl/shr/sar r64, cl` to low 6 bits,
            // so out-of-range shifts must be guarded explicitly (else shl by 64 wraps).
            let vslot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let lanes = if q { 16 / (esize as i32) } else { 8 / (esize as i32) };
            let bbits = (esize as i32) * 8;
            let wmask: u64 = if esize == 8 { u64::MAX } else { (1u64 << bbits) - 1 };
            for i in 0..lanes {
                let src = vslot(rn) + (i as i32) * (esize as i32);
                let cnt = vslot(rm) + (i as i32) * (esize as i32);
                let dst = vslot(rd) + (i as i32) * (esize as i32);
                // load value V -> RAX, count C -> RCX (zero-extended)
                match esize {
                    8 => buf.mov_load64(RAX, RBX, src),
                    4 => buf.mov_load32(RAX, RBX, src),
                    2 => buf.movzx_word_mem(RAX, RBX, src),
                    _ => buf.movzx_byte_mem(RAX, RBX, src),
                }
                match esize {
                    8 => buf.mov_load64(RCX, RBX, cnt),
                    4 => buf.mov_load32(RCX, RBX, cnt),
                    2 => buf.movzx_word_mem(RCX, RBX, cnt),
                    _ => buf.movzx_byte_mem(RCX, RBX, cnt),
                }
                // sign-extend the count lane to its element width
                match esize {
                    8 => {}
                    4 => buf.movsxd_r64_r32(RCX, RCX),
                    2 => {
                        buf.shl_ri8(RCX, 48);
                        buf.sar_ri8(RCX, 48);
                    }
                    _ => {
                        buf.shl_ri8(RCX, 56);
                        buf.sar_ri8(RCX, 56);
                    }
                }
                let mut done_jumps: Vec<usize> = Vec::new(); // jump offsets that target the final store
                let mut fixed: Vec<(usize, usize)> = Vec::new(); // (jump, fixed internal label)
                buf.test_rr64(RCX, RCX); // set SF (sign) from the sign-extended count
                let jneg = buf.jcc_rel32(0x88); // JS: count < 0 -> right path
                // ---- left path (C >= 0) ----
                if bbits < 64 {
                    buf.cmp_ri64(RCX, bbits as u32);
                    let jbig = buf.jcc_rel32(0x83); // JAE: C >= B -> result 0
                    buf.shl_cl64(RAX);
                    if wmask != u64::MAX {
                        buf.mov_ri64(RDX, wmask);
                        buf.and_rr64(RAX, RDX);
                    }
                    done_jumps.push(buf.jmp_rel32()); // jdone
                    let big_at = buf.len();
                    buf.mov_ri64(RAX, 0);
                    done_jumps.push(buf.jmp_rel32()); // jbigfall (big -> store)
                    fixed.push((jbig, big_at));
                } else {
                    // 64-bit, C in [0,63]: shl by CL is exact (x86 masks to low 6 bits)
                    buf.shl_cl64(RAX);
                    done_jumps.push(buf.jmp_rel32()); // jdone
                }
                // ---- right path (C < 0): -C = -(RCX) = k ----
                let right_at = buf.len();
                buf.neg_r64(RCX); // RCX = -C = magnitude k
                if signed_ {
                    // sshl/srshl: arithmetic right shift, sign-extend the element's sign first
                    match esize {
                        4 => buf.movsxd_r64_r32(RAX, RAX),
                        2 => {
                            buf.shl_ri8(RAX, 48);
                            buf.sar_ri8(RAX, 48);
                        }
                        1 => {
                            buf.shl_ri8(RAX, 56);
                            buf.sar_ri8(RAX, 56);
                        }
                        _ => {}
                    }
                }
                if bbits < 64 {
                    buf.cmp_ri64(RCX, bbits as u32);
                    let jbigr = buf.jcc_rel32(0x83); // JAE: magnitude >= B
                    // rounding (urshl/srshl): result = (V + (1 << (k-1))) >> k.
                    // Add the in-range bias only when k < B; the out-of-range branch
                    // below keeps RAX unbiased (ARM rounds only within-range right
                    // shifts — a full shift-out returns sign-fill/0 regardless).
                    if rounding {
                        // RDX = 1 << (k-1): bits k..0 set then shift right 1.
                        buf.mov_ri64(RDX, 1);
                        buf.shl_cl64(RDX);      // RDX = 1 << k
                        buf.shr_ri8(RDX, 1);    // RDX = 1 << (k-1)
                        buf.add_rr64(RAX, RDX); // V += bias
                    }
                    if signed_ {
                        buf.sar_cl64(RAX); // sshl/srshl: arithmetic right shift
                    } else {
                        buf.shr_cl64(RAX); // ushl/urshl: logical right shift
                    }
                    if wmask != u64::MAX {
                        buf.mov_ri64(RDX, wmask);
                        buf.and_rr64(RAX, RDX);
                    }
                    done_jumps.push(buf.jmp_rel32()); // jdoner
                    let sign_or_zero = buf.len();
                    // out-of-range right shift: ushl/urshl -> 0; sshl/srshl -> sign-fill
                    if signed_ {
                        // result = (V < 0) ? wmask : 0 ... but V already sign-extended.
                        buf.test_rr64(RAX, RAX);
                        let jnsz = buf.jcc_rel32(0x89); // JNS: V >= 0 -> 0
                        buf.mov_ri64(RAX, wmask);
                        done_jumps.push(buf.jmp_rel32()); // jz (wmask -> store)
                        let zero_at = buf.len();
                        buf.mov_ri64(RAX, 0);
                        fixed.push((jnsz, zero_at));
                    } else {
                        buf.mov_ri64(RAX, 0);
                    }
                    fixed.push((jbigr, sign_or_zero));
                } else {
                    // 64-bit (bbits==64): for 64-bit esize, rounding (srshl/q) is
                    // determined per-lane; here bbits<64 is false only for esize 8,
                    // and srshl .2d lives in the bbits<64==false path. Bias then sar.
                    if rounding {
                        buf.mov_ri64(RDX, 1);
                        buf.shl_cl64(RDX);
                        buf.shr_ri8(RDX, 1);
                        buf.add_rr64(RAX, RDX);
                    }
                    buf.sar_cl64(RAX); // 64-bit, C in [-63,-1] so sar is fine
                }
                // patch to final store
                let done = buf.len();
                for &at in &done_jumps {
                    let disp = (done as i64 - (at as i64 + 4)) as i32;
                    buf.bytes[at..at + 4].copy_from_slice(&disp.to_le_bytes());
                }
                for (at, tgt) in &fixed {
                    let disp = (*tgt as i64 - (*at as i64 + 4)) as i32;
                    buf.bytes[*at..*at + 4].copy_from_slice(&disp.to_le_bytes());
                }
                let disp_neg = (right_at as i64 - (jneg as i64 + 4)) as i32;
                buf.bytes[jneg..jneg + 4].copy_from_slice(&disp_neg.to_le_bytes());
                // store lane result to Vd[i]
                match esize {
                    8 => buf.mov_store64(RBX, dst, RAX),
                    4 => buf.mov_store32(RBX, dst, RAX),
                    2 => buf.mov_store16(RBX, dst, RAX),
                    _ => buf.mov_store8(RBX, dst, RAX),
                }
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
            let sign64: u64 = if esize == 8 { 0x8000_0000_0000_0000 }
                else if esize == 2 { 0x8000 } else { 0x8000_0000 };
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
                } else if esize == 2 {
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store16(RBX, dst + off, RAX); // fp16 fabs/fneg: 2 bytes
                } else {
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, dst + off, RAX);
                }
            }
            Ok(())
        }
        Inst::SimdFrint { rd, rn, mode, esize, q } => {
            // frint{n,m,p,z,a} Vd.T, Vn.T: per-lane FP rounding. esize 8 lanes
            // are doubles; esize 4 lanes are singles (promote to double, round,
            // demote). roundsd imm: 0b00=nearest-even(frintn), 0b01=floor(m),
            // 0b10=ceil(p), 0b11=toward-zero(z). frinta (mode 4) = ties-away:
            // sign*floor(|x|+0.5).
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            let emit_round = |buf: &mut crate::x86::CodeBuf| {
                match mode {
                    0 => buf.roundsd(0, 0, 0b00), // frintn: round to nearest-even
                    1 => buf.roundsd(0, 0, 0b01), // frintm: floor
                    2 => buf.roundsd(0, 0, 0b10), // frintp: ceil
                    3 => buf.roundsd(0, 0, 0b11), // frintz: toward zero
                    4 => {
                        // frinta: nearest, ties away = sign*floor(|x|+0.5)
                        buf.movq_r64_xmm(RAX, 0);
                        buf.mov_ri64(RCX, 0x8000_0000_0000_0000);
                        buf.and_rr64(RAX, RCX);               // sign bit
                        buf.movq_xmm_r64(2, RAX);             // xmm2 = sign mask
                        buf.movq_r64_xmm(RAX, 0);
                        buf.mov_ri64(RCX, 0x7fff_ffff_ffff_ffff);
                        buf.and_rr64(RAX, RCX);               // |x|
                        buf.movq_xmm_r64(0, RAX);
                        buf.mov_ri64(RAX, 0x3fe0_0000_0000_0000); // 0.5
                        buf.movq_xmm_r64(1, RAX);
                        buf.addsd(0, 1);                      // |x| + 0.5
                        buf.roundsd(0, 0, 0x01);              // floor
                        buf.pxor_xmm(0, 2);                   // reapply sign
                    }
                    _ => unreachable!("SimdFrint bad mode {mode}"),
                }
            };
            for l in 0..lanes {
                let off = l * m;
                if esize == 8 {
                    buf.movq_load(0, RBX, src + off); // 64-bit double lane
                    emit_round(&mut *buf);
                    buf.movq_store(RBX, dst + off, 0);
                } else {
                    buf.mov_load32(RAX, RBX, src + off); // 32-bit float lane
                    buf.movd_xmm_r32(0, RAX);
                    buf.cvtss2sd(0, 0); // promote to double
                    emit_round(&mut *buf);
                    buf.cvtsd2ss(0, 0); // demote back to single
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, dst + off, RAX);
                }
            }
            Ok(())
        }
        Inst::SimdArithUnary { rd, rn, esize, q, op } => {
            // neg/abs Vd.T, Vn.T: per-lane signed negate or absolute value.
            // neg: 0 - lane (two's complement wraps on overflow, matching ARM).
            // abs: |signed lane| via the identity (x ^ (x ar>> w-1)) - (x ar>> w-1)
            // after sign-extending the lane; safe read-modify-write when rd==rn.
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            for l in 0..lanes {
                match esize {
                    8 => buf.mov_load64(RAX, RBX, src + l * m),
                    4 => {
                        buf.mov_load32(RAX, RBX, src + l * m);
                        if op == 1 {
                            buf.movsxd_r64_r32(RAX, RAX); // sign-extend for |signed|
                        }
                    }
                    2 => {
                        if op == 1 {
                            buf.movsx_word_mem(RAX, RBX, src + l * m);
                        } else {
                            buf.movzx_word_mem(RAX, RBX, src + l * m);
                        }
                    }
                    _ => {
                        if op == 1 {
                            buf.movsx_byte_mem(RAX, RBX, src + l * m);
                        } else {
                            buf.movzx_byte_mem(RAX, RBX, src + l * m);
                        }
                    }
                }
                if op == 0 {
                    buf.neg_r64(RAX);
                } else {
                    let w = esize * 8;
                    buf.mov_rr64(RCX, RAX);
                    buf.sar_ri8(RCX, (w - 1) as u8);
                    buf.xor_rr64(RAX, RCX);
                    buf.sub_rr64(RAX, RCX);
                }
                match esize {
                    8 => buf.mov_store64(RBX, dst + l * m, RAX),
                    4 => buf.mov_store32(RBX, dst + l * m, RAX),
                    2 => buf.mov_store16(RBX, dst + l * m, RAX),
                    _ => buf.mov_store8(RBX, dst + l * m, RAX),
                }
            }
            Ok(())
        }
        Inst::SimdAddp { rd, rn, rm, esize, q } => {
            // ADDP Vd.T, Vn.T, Vm.T: pairwise-adjacent addition, no widening.
            // First half of the output lanes = sums of adjacent (2i, 2i+1) lane
            // pairs of Vn; second half = pair sums of Vm. Output lanes total
            // n/esize where n = 8 (q=0) or 16 (q=1) bytes. Each src element read
            // EXACTLY esize bytes (zero-extended) then added, so a lane's high
            // bytes never pollute the neighbour; store exactly esize bytes.
            // SELF-ALIAS (gcc emits addp v31,v31,v31 in reductions): the second
            // half of the dest overlaps the source bytes the first half just wrote,
            // so snapshot any source that aliases rd to scratch (permute_source).
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            let rn_src = permute_source(buf, rd, rn, false);
            let rm_src = permute_source(buf, rd, rm, true);
            let n: i32 = if q { 16 } else { 8 };
            let es = esize as i32;
            let out_lanes = n / es;
            let half = out_lanes / 2;
            for i in 0..out_lanes {
                let (base, pair) = if i < half {
                    (rn_src, i * 2)
                } else {
                    (rm_src, (i - half) * 2)
                };
                // load pair at (2i, 2i+1), zero-extend each to 64-bit, add
                let mut load = |tgt: u8, off: i32| match esize {
                    8 => buf.mov_load64(tgt, RBX, base + off * es),
                    4 => buf.mov_load32(tgt, RBX, base + off * es),
                    2 => buf.movzx_word_mem(tgt, RBX, base + off * es),
                    _ => buf.movzx_byte_mem(tgt, RBX, base + off * es),
                };
                load(RAX, pair);
                load(RCX, pair + 1);
                buf.add_rr64(RAX, RCX);
                match esize {
                    8 => buf.mov_store64(RBX, slot(rd) + i * es, RAX),
                    4 => buf.mov_store32(RBX, slot(rd) + i * es, RAX),
                    2 => buf.mov_store16(RBX, slot(rd) + i * es, RAX),
                    _ => buf.mov_store8(RBX, slot(rd) + i * es, RAX),
                }
            }
            Ok(())
        }
        Inst::FcvVec { rd, rn, signed, esize, q } => {
            // fcvtzu/fcvtzs Vd.T, Vn.T: convert each FP lane (esize bytes) to an
            // int, truncating toward zero; negative clamp for the unsigned form.
            // The 4-byte (S) lane form is NOT a double: it must load the 32-bit
            // float and promote (movq_load would read 8 bytes = lane + next lane),
            // and the result lane is 32-bit (mov_store32, not mov_store64 which
            // would clobber the neighbouring lane).
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            for l in 0..lanes {
                if esize == 8 {
                    buf.movq_load(0, RBX, src + l * m); // 64-bit double lane
                } else {
                    buf.mov_load32(RAX, RBX, src + l * m); // 32-bit float lane
                    buf.movd_xmm_r32(0, RAX);
                    buf.cvtss2sd(0, 0); // promote to double in xmm0
                }
                buf.cvttsd2si(RAX, 0);
                if !signed {
                    buf.xor_rr64(RCX, RCX);
                    buf.test_rr64(RAX, RAX);
                    buf.cmov_rr64(0x48, RAX, RCX); // negative d -> 0
                }
                if esize == 8 {
                    buf.mov_store64(RBX, dst + l * m, RAX); // 64-bit int lane
                } else {
                    buf.mov_store32(RBX, dst + l * m, RAX); // 32-bit int lane
                }
            }
            Ok(())
        }
                Inst::VecIntToFp { rd, rn, esize, signed, q } => {
            // scvtf/ucvtf Vd.T, Vn.T: convert each int lane (esize bytes) to FP.
            // The reverse of FcvVec. esize=4 -> s32/u32 -> f32 per lane;
            // esize=8 -> (scvtf v.2d) i64 -> f64 per lane.
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as i32 } else { 8 / esize as i32 };
            let m = esize as i32;
            for l in 0..lanes {
                if esize == 8 {
                    // scvtf v.2d: 64-bit signed int lane -> double.
                    buf.mov_load64(RAX, RBX, src + l * m);
                    buf.cvtsi2sd(0, true, RAX);
                    buf.movq_store(RBX, dst + l * m, 0);
                } else {
                    // 32-bit lane. mov_load32 zero-extends to RAX; sign-extend
                    // if signed so cvtsi2ss is exact for negatives.
                    buf.mov_load32(RAX, RBX, src + l * m);
                    if signed {
                        buf.movsxd_r64_r32(RAX, RAX);
                    }
                    buf.cvtsi2ss(0, true, RAX);
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, dst + l * m, RAX);
                }
            }
            Ok(())
        }
                Inst::VecFcvtl { rd, rn, upper, half } => {
            // fcvtl Vd.2D, Vn.2S (f32->f64) / fcvtl2 Vd.2D, Vn.4S — widen two f32
            // lanes of Vn to doubles; OR the fp16 form (half): fcvtl Vd.4s, Vn.4h /
            // fcvtl2 Vd.4s, Vn.8h — widen four f16 lanes to floats via F16C.
            // fcvtl reads Vn bytes 0..7 (f32) / 0..8 (fp16); fcvtl2 (upper) reads
            // Vn bytes 8..15 (f32) / 8..16 (fp16). Both write all of Vd.
            // In-place: fcvtl (lower) writing would clobber Vn's upper lanes when
            // rd==rn; snapshot Vn via permute_source.
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let src = permute_source(buf, rd, rn, false);
            if half {
                // fp16 -> f32 widen: 4 lanes, vcvtph2ps each promoted lane.
                let soff = if upper { 8i32 } else { 0i32 };
                for lane in 0..4 {
                    buf.mov_load16(RAX, RBX, src + soff + lane * 2); // f16 lane
                    buf.movd_xmm_r32(0, RAX);
                    buf.bytes.extend_from_slice(&[0xc4, 0xe2, 0x79, 0x13, 0xc0]); // vcvtph2ps xmm0,xmm0
                    buf.movd_r32_xmm(RAX, 0);
                    buf.mov_store32(RBX, dst + lane * 4, RAX); // f32 lane
                }
            } else {
                let soff = if upper { 8i32 } else { 0i32 };
                for lane in 0..2 {
                    buf.mov_load32(RAX, RBX, src + soff + lane * 4); // f32 lane
                    buf.movd_xmm_r32(0, RAX); // to xmm0 low 32
                    buf.cvtss2sd(0, 0); // widen f64
                    buf.movq_store(RBX, dst + lane * 8, 0); // store f64 lane
                }
            }
            Ok(())
        }
                Inst::VecFcvtn { rd, rn, upper } => {
            // fcvtn Vd.2S, Vn.2D / fcvtn2 Vd.4S, Vn.2D: narrow two double lanes
            // of Vn to floats in Vd. fcvtn writes Vd bytes 0..7, fcvtn2 (upper)
            // writes Vd bytes 8..15. Source is always the full 2 doubles.
            // In-place: fcvtn2 (upper) writing the f32 lane0 at Vd byte 8 would
            // clobber Vn's f64 lane1 at byte 8 before it's read when rd==rn.
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let src = permute_source(buf, rd, rn, false);
            let doff = if upper { 8i32 } else { 0i32 };
            for lane in 0..2 {
                buf.movq_load(0, RBX, src + lane * 8); // f64 lane -> xmm0
                buf.cvtsd2ss(0, 0); // narrow f32 in xmm0 low
                buf.movd_r32_xmm(RAX, 0); // low32 -> RAX
                buf.mov_store32(RBX, dst + doff + lane * 4, RAX); // store f32
            }
            Ok(())
        }
        Inst::SimdCmpZero { rd, rn, esize, q, cond } => {
            // cmeq/cmgt/cmge/cmlt/cmle Vd.T,Vn.T,#0: each lane -> all-ones if the
            // signed int compare against literal 0 holds, else 0. cond: 0=eq,
            // 1=gt, 2=ge, 3=lt, 4=le. Signed conds sign-extend the lane so the
            // x86 setg/setge/setl/setle on the 64-bit value compare correctly.
            let src = crate::jit::VECTOR_BASE + (rn as i32) * 16;
            let dst = crate::jit::VECTOR_BASE + (rd as i32) * 16;
            let lanes = if q { 16 / esize as u32 } else { 8 / esize as u32 } as u32;
            let e = esize as i32;
            let signed = cond != 0;
            for l in 0..lanes {
                let off = (l as i32) * e;
                if e == 8 {
                    buf.mov_load64(RAX, RBX, src + off);
                } else if signed {
                    match e {
                        4 => { buf.mov_load32(RAX, RBX, src + off); buf.movsxd_r64_r32(RAX, RAX); }
                        2 => buf.movsx_word_mem(RAX, RBX, src + off),
                        _ => buf.movsx_byte_mem(RAX, RBX, src + off),
                    }
                } else {
                    match e {
                        4 => buf.mov_load32(RAX, RBX, src + off),
                        2 => buf.movzx_word_mem(RAX, RBX, src + off),
                        _ => buf.movzx_byte_mem(RAX, RBX, src + off),
                    }
                }
                buf.test_rr64(RAX, RAX);
                let cc = match cond { 0 => 4, 1 => 0xf, 2 => 0xd, 3 => 0xc, _ => 0xe };
                buf.setcc_rm8(cc, RAX);       // AL = 0/1
                buf.movzx_r32_r8(RAX, RAX);   // RAX = 0/1
                buf.neg_r64(RAX);             // 0 or all-ones
                match e {
                    8 => buf.mov_store64(RBX, dst + off, RAX),
                    4 => buf.mov_store32(RBX, dst + off, RAX),
                    2 => buf.mov_store16(RBX, dst + off, RAX),
                    _ => buf.mov_store8(RBX, dst + off, RAX),
                }
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
                                                                                                                                                                                Inst::SimdBit { rd, rn, rm, bif } => {
            let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
            for off in [0i32, 8] {
                // BIT (bif=false): (Vn&Vm)|(Vd&~Vm); BIF (bif=true): (Vn&~Vm)|(Vd&Vm).
                // RAX = (sel & Vm), RDX = (keep & ~Vm), result = RAX|RDX, where
                // (sel,keep) = (Vn,Vd) for BIT and (Vd,Vn) for BIF.
                let (sel, keep) = if bif { (rd, rn) } else { (rn, rd) };
                buf.mov_load64(RAX, RBX, slot(sel) + off); // source taken where Vm=1
                buf.mov_load64(RCX, RBX, slot(rm) + off); // RCX = Vm
                buf.and_rr64(RAX, RCX); // RAX = sel & Vm
                buf.mov_load64(RDX, RBX, slot(keep) + off); // source taken where Vm=0
                buf.mov_ri64(RDI, 0xffff_ffff_ffff_ffff);
                buf.xor_rr64(RCX, RDI); // RCX = ~Vm
                buf.and_rr64(RDX, RCX); // RDX = keep & ~Vm
                buf.or_rr64(RAX, RDX); // (sel&Vm)|(keep&~Vm)
                buf.mov_store64(RBX, slot(rd) + off, RAX);
            }
            Ok(())
        }
        Inst::SimdExt { rd, rn, rm, imm, q } => {
                                                                                                                                                                                    // ext Vd, Vn, Vm, #imm: Vd = the 128(64)-bit window of the
                                                                                                                                                                                    // concatenation starting at byte `imm`, where **Vn occupies the
                                                                                                                                                                                    // low-address bytes (0..15) and Vm the high (16..31)** — verified
                                                                                                                                                                                    // against qemu: ext(Vn=0x99..,Vm=0x02..,#8) -> lo=Vn[8..15],
                                                                                                                                                                                    // hi=Vm[0..7]. (The OLD order had Vm low and Vn high, which
                                                                                                                                                                                    // inverted every non-symmetric `ext`; gcc's horizontal
                                                                                                                                                                                    // xor-reduce emitted `ext v0,v30,v0,#8` and returned v30.hi^
                                                                                                                                                                                    // v30.lo wrong by a full xor of one 64-bit lane.)
                                                                                                                                                                                    // concat words W[0..3] = Vn.lo, Vn.hi, Vm.lo, Vm.hi (byte
                                                                                                                                                                                    // addresses 0..31). result.lo = bytes imm..imm+7 of concat,
                                                                                                                                                                                    // result.hi = bytes imm+8..imm+15 (16B form). For 8B (Q=0)
                                                                                                                                                                                    // only the low 64 bits are produced and the high lane is 0.
                                                                                                                                                                                    let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                                                                                                    // Emit 64-bit field of concat starting at byte `start` into `dst`.
                                                                                                                                                                                    let emit_bytes64 = |buf: &mut crate::x86::CodeBuf, dst: u8, start: usize| {
                                                                                                                                                                                                                                            let wi = start / 8;
                                                                                                                                                                                                                                            // sh = bit offset within the concat word (start is in BYTES)
                                                                                                                                                                                                                                            let sh = ((start % 8) * 8) as u8;
                                                                                                                                                                                                                                            if sh == 0 {
                                                                                                                                                                                            // aligned: 8 bytes directly from one concat word
                                                                                                                                                                                            match wi {
                                                                                                                                                                                                0 => buf.mov_load64(dst, RBX, slot(rn)),
                                                                                                                                                                                                1 => buf.mov_load64(dst, RBX, slot(rn) + 8),
                                                                                                                                                                                                2 => buf.mov_load64(dst, RBX, slot(rm)),
                                                                                                                                                                                                _ => buf.mov_load64(dst, RBX, slot(rm) + 8),
                                                                                                                                                                                            }
                                                                                                                                                                                        } else {
                                                                                                                                                                                            // unaligned: (W[wi] >> sh) | (W[wi+1] << (64-sh))
                                                                                                                                                                                            let (ra0, a_off, rb0, b_off) = match wi {
                                                                                                                                                                                                0 => (rn, 0, rn, 8),
                                                                                                                                                                                                1 => (rn, 8, rm, 0),
                                                                                                                                                                                                _ => (rm, 0, rm, 8),
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
                                                                                                                                                                                                                                                                                                                                                                                                            match esize {
                                                                                                                                                                                                                                                                                                                                                                                                                8 => {
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load64(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                4 => {
                                                                                                                                                                                                                                                                                                                                                                                                                    // esize == 4 (.s): load 32-bit, zero- or sign-extend.
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.mov_load32(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    if sign {
                                                                                                                                                                                                                                                                                                                                                                                                                        buf.movsxd_r64_r32(RAX, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                2 if sign => {
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.movsx_word_mem(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                2 => {
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.movzx_word_mem(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                1 if sign => {
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.movsx_byte_mem(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                                _ => {
                                                                                                                                                                                                                                                                                                                                                                                                                    // esize == 1, unsigned
                                                                                                                                                                                                                                                                                                                                                                                                                    buf.movzx_byte_mem(RAX, RBX, vbase);
                                                                                                                                                                                                                                                                                                                                                                                                                    stg(buf, rd as u32, RAX);
                                                                                                                                                                                                                                                                                                                                                                                                                }
                                                                                                                                                                                                                                                                                                                                                                                                            }
                                                                                                                                                                                                                                                                                                                                                                                                            Ok(())
                                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                    Inst::InsGp { rd, rn, esize, index } => {
                                                                                                        // ins/mov Vd.T[index], Rn: copy esize bytes of GPR rn into
                                                                                                        // the vector slot Vd at byte offset index*esize (the reverse
                                                                                                        // of SimdLaneGp). Only the low esize bytes of Rn participate.
                                                                                                        let slot = |r: u8| crate::jit::VECTOR_BASE + (r as i32) * 16;
                                                                                                        let off = (index as i32) * (esize as i32);
                                                                                                        ldg(buf, RAX, rn as u32);
                                                                                                        match esize {
                                                                                                            1 => buf.mov_store8(RBX, slot(rd) + off, RAX),
                                                                                                            2 => buf.mov_store16(RBX, slot(rd) + off, RAX),
                                                                                                            4 => buf.mov_store32(RBX, slot(rd) + off, RAX),
                                                                                                            _ => buf.mov_store64(RBX, slot(rd) + off, RAX),
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
            fp_s,
            sext,
        } => {
            let esize = if q128 {
                16i32
            } else if fp_d {
                8i32
            } else if fp_s {
                4i32
            } else if size_64 {
                8i32
            } else {
                4i32
            };
            let imm32 = imm as i32;
            // eff base: three addressing modes.
            //  pre-index `[rn, #imm]!`  -> access at rn+imm, then rn += imm
            //  post-index `[rn], #imm`  -> access at rn,     then rn += imm
            //  offset     `[rn, #imm]`  -> access at rn+imm, no writeback
            // (ARM bit23=indexed, bit24=pre within indexed; offset form has
            //  bit23=0 => writeback=false. The offset immediate must still apply
            //  to the access address.)
            ldg(buf, RDX, rn as u32); // RDX = rn
            let (access_off, wb_off) = if preidx {
                (imm32, imm32)
            } else if writeback {
                (0i32, imm32) // post-index: access at rn, then rn += imm
            } else {
                (imm32, 0i32) // offset: access at rn+imm, no writeback
            };
            if fp_d {
                // 64-bit FP/vector d-pair: each reg is the LOW 8 bytes of its
                // 16-byte guest vector slot, at VECTOR_BASE + rt*16. BUGFIX
                // (Session 99): the stride was rt*8, so `ldp d29,d28` wrote to
                // 0x1f8/0x1f0 instead of 0x2e0/0x2d0 and the follow-on fmadd read
                // stale vector slots (structfield.elf -O2 returned 128 vs 52).
                let v0 = crate::jit::VECTOR_BASE + (rt as i32) * 16;
                let v1 = crate::jit::VECTOR_BASE + (rt2 as i32) * 16;
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
            if fp_s {
                // 32-bit FP/vector s-pair: each reg is the LOW 4 bytes of its
                // 16-byte vector slot, at VECTOR_BASE + rt*16. 4-byte transfers.
                let v0 = crate::jit::VECTOR_BASE + (rt as i32) * 16;
                let v1 = crate::jit::VECTOR_BASE + (rt2 as i32) * 16;
                for (reg_off, mem_off) in [(v0, access_off), (v1, access_off + esize)] {
                    if ld {
                        buf.mov_load32(RAX, RDX, mem_off); // eax <- [addr]
                        buf.mov_store32(RBX, reg_off, RAX); // v low4 <- eax
                    } else {
                        buf.mov_load32(RAX, RBX, reg_off); // eax <- v low4
                        buf.mov_store32(RDX, mem_off, RAX); // [addr] <- eax
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
                    stg_if_writable(buf, rt as u32);
                    buf.mov_load64(RAX, RDX, access_off + esize);
                    stg_if_writable(buf, rt2 as u32);
                } else {
                    if sext {
                        // ldpsw: load 32-bit, sign-extend to 64-bit X reg.
                        buf.mov_load32(RAX, RDX, access_off);
                        buf.movsxd_r64_r32(RAX, RAX);
                        stg_if_writable(buf, rt as u32);
                        buf.mov_load32(RAX, RDX, access_off + esize);
                        buf.movsxd_r64_r32(RAX, RAX);
                        stg_if_writable(buf, rt2 as u32);
                    } else {
                        buf.mov_load32(RAX, RDX, access_off);
                        stg_if_writable(buf, rt as u32);
                        buf.mov_load32(RAX, RDX, access_off + esize);
                        stg_if_writable(buf, rt2 as u32);
                    }
                }
            } else {
                // store rt at [eff], rt2 at [eff+esize]
                if size_64 {
                    ldg_src(buf, rt as u32);
                    buf.mov_store64(RDX, access_off, RAX);
                    ldg_src(buf, rt2 as u32);
                    buf.mov_store64(RDX, access_off + esize, RAX);
                } else {
                    ldg_src(buf, rt as u32);
                    buf.mov_store32(RDX, access_off, RAX);
                    ldg_src(buf, rt2 as u32);
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
            sext,
            index_ext,
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
            // index_ext (option bits[14:13]): 2 = UXTW (index is rm's low 32
            // bits, zero-extended to 64 before the shift), 1 = UXTB (low byte).
            // Real book code stores a bit-32 "sentinel" in the X register and
            // relies on `[xN, wM, uxtw#S]` dropping it — using the full 64-bit
            // rm here indexes OOB and SIGSEGVs (`ldr w8,[x8,w0,uxtw#2]` at 0x2173218
            // with x0=0x100000665). 3 (LSL) keeps the full 64-bit rm index.
            match index_ext {
                2 => buf.zero_ext_r32(RCX),
                1 => buf.and_ri64(RCX, 0xff),
                _ => {} // 3 (LSL/UXTX) or 0 (reserved): full-width index
            }
            if shift_amt != 0 {
                // Currently only constant <=3 via the (unused) sar_cl; emit shift left.
                // x86 has no shl-by-imm op in this emitter; use add-based *2 for 1..3.
                for _ in 0..shift_amt {
                    buf.add_rr64(RCX, RCX); // RCX += RCX (shift left by 1)
                }
            }
            buf.add_rr64(RAX, RCX); // RAX = effective address
            if ld && sext {
                // Sign-extending register-offset load (ldrsw/ldrsh/ldrsb).
                // Load `size` bytes from [RAX] and sign-extend into RAX; the
                // address is no longer needed, so RAX replaces the base.
                match size {
                    4 => {
                        buf.mov_load32(RAX, RAX, 0);
                        buf.movsxd_r64_r32(RAX, RAX);
                        stg_if_writable(buf, rt as u32);
                    }
                    2 => {
                        buf.movzx_word_mem(RAX, RAX, 0);
                        buf.shl_ri8(RAX, 48);
                        buf.sar_ri8(RAX, 48);
                        stg_if_writable(buf, rt as u32);
                    }
                    1 => {
                        buf.movzx_byte_mem(RAX, RAX, 0);
                        buf.shl_ri8(RAX, 56);
                        buf.sar_ri8(RAX, 56);
                        stg_if_writable(buf, rt as u32);
                    }
                    s => return Err(format!("LdStrReg sign-extend size {} not implemented", s)),
                }
                return Ok(());
            }
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
            if std::env::var_os("JIT_TRACE").is_some() && ld {
                // print addr + current 16 bytes at translate time (target addr
                // is RAX-modifiable only at runtime, so approximate via x0 slot).
                eprintln!("VECLD[DBG] ld vt={vt} rn={rn} imm={imm} off={off}");
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
        Inst::VecLdStrReg { vt, rn, rm, ld } => {
            // addr = x[rn] + x[rm] ; transfer 16 bytes via XMM0. Vector-file
            // register-offset ld/st; the GPR base/index are unaffected (this
            // must NOT write back into either — the old mis-decode stored a
            // byte INTO x[rn]'s register and corrupted the caller's state).
            ldg(buf, RAX, rn as u32); // base
            ldg(buf, RCX, rm as u32); // index
            buf.add_rr64(RAX, RCX); // RAX = addr
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            if ld {
                buf.movdqu_load(0, RAX, 0);
                buf.movdqu_store(RBX, vslot, 0);
            } else {
                buf.movdqu_load(0, RBX, vslot);
                buf.movdqu_store(RAX, 0, 0);
            }
            Ok(())
        }
        Inst::VecLdStImmUnscaled { vt, rn, imm9, ld } => {
            // addr = x[rn] + imm9 (signed) ; transfer 16 bytes via XMM0.
            ldg(buf, RAX, rn as u32);
            if imm9 != 0 {
                buf.lea64(RAX, RAX, imm9);
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            if ld {
                buf.movdqu_load(0, RAX, 0);
                buf.movdqu_store(RBX, vslot, 0);
            } else {
                buf.movdqu_load(0, RBX, vslot);
                buf.movdqu_store(RAX, 0, 0);
            }
            Ok(())
        }
        Inst::VecLdStIndexed { vt, rn, imm9, ld, pre } => {
            // pre:  addr = x[rn]+imm9, then x[rn] += imm9
            // post: addr = x[rn],     then x[rn] += imm9
            // 16-byte transfer via XMM0, then write back the advanced pointer.
            ldg(buf, RAX, rn as u32); // x[rn]
            if pre && imm9 != 0 {
                buf.lea64(RAX, RAX, imm9); // addr = x[rn]+imm9 (pre)
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            if ld {
                buf.movdqu_load(0, RAX, 0);
                buf.movdqu_store(RBX, vslot, 0);
            } else {
                buf.movdqu_load(0, RBX, vslot);
                buf.movdqu_store(RAX, 0, 0);
            }
            // Xn += imm9 (pre and post both advance the base register).
            ldg(buf, RCX, rn as u32);
            if imm9 != 0 {
                buf.lea64(RCX, RCX, imm9);
            }
            stg(buf, rn as u32, RCX);
            Ok(())
        }
        Inst::FpLdStImmUnscaled { vt, rn, imm9, size, ld } => {
            // Scalar ldur/stur: addr = x[rn] + imm9 (signed), no writeback.
            ldg(buf, RDX, rn as u32);
            if imm9 != 0 {
                buf.lea64(RDX, RDX, imm9);
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            fp_scalar_xfer(buf, RDX, vslot, size, ld)?;
            Ok(())
        }
        Inst::FpLdStImmWb { vt, rn, imm9, size, ld, pre } => {
            // Scalar pre/post-index ldr/str with base writeback.
            //   pre:  addr = x[rn] + imm9, then Xn += imm9
            //   post: addr = x[rn],        then Xn += imm9
            // NOTE: address must live in RDX (NOT RAX) — fp_scalar_xfer uses RAX
            // as its value scratch, so a store with addr==RAX would clobber the
            // base with the value being stored and write to [value] (a latent
            // pre/post-index scalar STORE miscompile, e.g. `str s30,[x4],#4`
            // faulting at 0x41480000 = the float bits). Mirrors FpLdStImmUnscaled.
            ldg(buf, RDX, rn as u32);
            if pre && imm9 != 0 {
                buf.lea64(RDX, RDX, imm9); // pre-add the offset into the address
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            fp_scalar_xfer(buf, RDX, vslot, size, ld)?;
            // base register advances by imm9 for both pre and post index.
            ldg(buf, RCX, rn as u32);
            if imm9 != 0 {
                buf.lea64(RCX, RCX, imm9);
            }
            stg(buf, rn as u32, RCX);
            Ok(())
        }
        Inst::FpLdStrReg { vt, rn, rm, size, ld, shift, index_ext } => {
            // addr = x[rn] + (x[rm] << log2(size)) in RDX ; transfer `size`
            // bytes between [addr] and the low bytes of guest vector slot v[vt].
            // Scalar register-offset (B/H/S/D); bit26=1 vector file.
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
            ldg(buf, RDX, rn as u32); // base address
            ldg(buf, RAX, rm as u32); // index
            match index_ext {
                2 => buf.zero_ext_r32(RAX), // UXTW: low-32 index (drop sentinel high)
                1 => buf.and_ri64(RAX, 0xff), // UXTB
                _ => {}                     // LSL (full 64-bit)
            }
            for _ in 0..shift_amt {
                buf.add_rr64(RAX, RAX);
            }
            buf.add_rr64(RDX, RAX); // RDX = addr
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            match (size, ld) {
                (1, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    buf.mov_store8(RBX, vslot, RAX);
                }
                (1, false) => {
                    buf.movzx_byte_mem(RAX, RBX, vslot);
                    buf.mov_store8(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    buf.mov_store16(RBX, vslot, RAX);
                }
                (2, false) => {
                    buf.movzx_word_mem(RAX, RBX, vslot);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (4, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    buf.mov_store32(RBX, vslot, RAX);
                }
                (4, false) => {
                    buf.mov_load32(RAX, RBX, vslot);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (8, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    buf.mov_store64(RBX, vslot, RAX);
                }
                (8, false) => {
                    buf.mov_load64(RAX, RBX, vslot);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("FpLdStrReg size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::FpLdStImm { vt, rn, imm, size, ld } => {
            // Transfer `size` bytes (B/H/S/D) between the low bytes of the guest
            // vector slot v[vt] and [rn + imm*size]. Upper lanes of the 16-byte
            // slot are untouched (ARM `ldr d0` preserves the high 64 bits, and a
            // scalar `str s0/d0` only stores the low 32/64).
            ldg(buf, RDX, rn as u32); // base address
            let off = (imm as i32).wrapping_mul(size as i32);
            if off != 0 {
                buf.lea64(RDX, RDX, off);
            }
            let vslot = crate::jit::VECTOR_BASE + (vt as i32) * 16;
            match (size, ld) {
                (8, true) => {
                    buf.mov_load64(RAX, RDX, 0);
                    buf.mov_store64(RBX, vslot, RAX);
                }
                (8, false) => {
                    buf.mov_load64(RAX, RBX, vslot);
                    buf.mov_store64(RDX, 0, RAX);
                }
                (4, true) => {
                    buf.mov_load32(RAX, RDX, 0);
                    buf.mov_store32(RBX, vslot, RAX);
                }
                (4, false) => {
                    buf.mov_load32(RAX, RBX, vslot);
                    buf.mov_store32(RDX, 0, RAX);
                }
                (2, true) => {
                    buf.movzx_word_mem(RAX, RDX, 0);
                    buf.mov_store16(RBX, vslot, RAX);
                }
                (2, false) => {
                    buf.movzx_word_mem(RAX, RBX, vslot);
                    buf.mov_store16(RDX, 0, RAX);
                }
                (1, true) => {
                    buf.movzx_byte_mem(RAX, RDX, 0);
                    buf.mov_store8(RBX, vslot, RAX);
                }
                (1, false) => {
                    buf.movzx_byte_mem(RAX, RBX, vslot);
                    buf.mov_store8(RDX, 0, RAX);
                }
                (s, _) => return Err(format!("FpLdStImm size {} not implemented", s)),
            }
            Ok(())
        }
        Inst::VecMovi { vd, lo, hi, kind } => {
            // Write a full 128-bit vector immediate into the guest v-slot
            // (CpuState.v, 16 bytes at VECTOR_BASE + 16*vd). The two u64 halves
            // are hoisted as immediates. kind: 0 = write (movi/mvni),
            // 1 = AND-in-place (bic, Vd &= lo/hi), 2 = OR-in-place (orr).
            // Use a scratch reg for the (possibly >32-bit) mask.
            let vslot = crate::jit::VECTOR_BASE + (vd as i32) * 16;
            match kind {
                1 | 2 => {
                    for (off, mask) in [(vslot, lo), (vslot + 8, hi)] {
                        buf.mov_load64(RAX, RBX, off);
                        buf.mov_ri64(RCX, mask);
                        if kind == 1 {
                            buf.and_rr64(RAX, RCX);
                        } else {
                            buf.or_rr64(RAX, RCX);
                        }
                        buf.mov_store64(RBX, off, RAX);
                    }
                }
                _ => {
                    buf.mov_ri64(RAX, lo);
                    buf.mov_store64(RBX, vslot, RAX);
                    buf.mov_ri64(RAX, hi);
                    buf.mov_store64(RBX, vslot + 8, RAX);
                }
            }
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
        Inst::SmeNoop => {
            // str za / smstart za / smstop za — no-ops in this SME-off guest
            // (the ZA tile is never live and streaming mode is never entered;
            // glibc compiles them on its __libc_arm_za_disable path).
            Ok(())
        }
        Inst::AddVectorLen { rd, rn, imm_bytes } => {
            // addvl/addsvl: Rd = Rn + imm*VL. VL=16 bytes in this no-SVE model.
            // Fetch Rn, add the scaled byte count, store Rd.
            ldg_src(buf, rn as u32);
            if imm_bytes != 0 {
                buf.add_ri64(RAX, imm_bytes as u32);
            }
            if rd != 31 {
                stg(buf, rd as u32, RAX);
            }
            Ok(())
        }
        Inst::SveCntd { rd } => {
            // cntd Xd = VL_d/64 = 2 at VL=16 bytes.
            if rd != 31 {
                buf.mov_ri64(RAX, 2);
                stg(buf, rd as u32, RAX);
            }
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
            ldg_src(buf, rt as u32); // test rt
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
            ldg_src(buf, rt as u32); // load rt
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
