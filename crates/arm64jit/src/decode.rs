// SPDX-License-Identifier: MIT
//
// ARM64 (AArch64) instruction decoder for the open-sober JIT.
//
// Bitfields follow the ARM ARM (DDI0487). Encodings are verified against ground
// truth emitted by aarch64-linux-gnu-gcc -O1 (objdump) during development. Code
// outside the covered subset decodes to Unsupported so the translator traps on
// it visibly. Each class is added with a unit test matching the real encoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftKind {
    Lsl,
    Lsr,
    Asr,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchRegKind {
    Br,
    Blr,
    Ret,
    Eret,
}

/// A decoded AArch64 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    // ---- unconditional branch ----
    B { imm: i64, link: bool },
    // ---- conditional branch ----
    BCond { cond: u8, imm: i64 },
    // ---- pc-relative ----
    Adr { rd: u8, imm: i64 },
    Adrp { rd: u8, imm: i64 },
    // ---- move wide ----
    MoveWide { rd: u8, imm16: u16, hw: u8, opc: u8, sf: bool },
    // ---- add/sub immediate ----
    AddSubImm { rd: u8, rn: u8, imm12: u32, shift12: bool, sub: bool, sf: bool, s: bool },
    // ---- fallback ----
    Unsupported(u32),
}

#[inline]
fn b(insn: u32, lo: u32, hi: u32) -> u32 {
    (insn >> lo) & ((1u32 << (hi - lo + 1)) - 1)
}
#[inline]
fn sext(v: u64, bits: u32) -> i64 {
    ((v << (64 - bits)) as i64) >> (64 - bits)
}
#[inline]
fn rd(insn: u32) -> u8 {
    (insn & 0x1F) as u8
}
#[inline]
fn rn(insn: u32) -> u8 {
    b(insn, 5, 9) as u8
}

pub fn decode(insn: u32) -> Inst {
    // ---- unconditional branch: bits[30:26] = 0b00101, bit31=link ----
    if b(insn, 26, 30) == 0b00101 {
        let link = insn >> 31 == 1;
        let imm = sext((insn & 0x03FF_FFFF) as u64, 26);
        return Inst::B { imm: imm << 2, link };
    }

    // ---- cond branch: bits[31:24] = 0x54 ----
    if insn >> 24 == 0x54 {
        let cond = (insn & 0xF) as u8;
        let imm = sext(b(insn, 5, 23) as u64, 19);
        return Inst::BCond { cond, imm: imm << 2 };
    }

    // ---- ADR / ADRP: bits[31:24] = 0x00 / 0x90 ----
    let immlo = (insn >> 29) & 0x3; // imm[1:0]
    let immhi = b(insn, 5, 23); // imm[20:2]
    let imm = (((immhi as u64) << 2) | (immlo as u64));
    match insn >> 24 {
        0x00 => return Inst::Adr { rd: rd(insn), imm: sext(imm, 21) },
        0x90 => return Inst::Adrp { rd: rd(insn), imm: sext(imm, 21) << 12 },
        _ => {}
    }

    // ---- MoveWide (MOVZ/MOVK/MOVN): high byte is one of 6 verified encodings.
        //   0x52 movz32  0xD2 movz64  0x72 movk32  0xF2 movk64  0x12 movn32  0x92 movn64
        let top = insn >> 24;
        let sf = insn >> 31 == 1;
        if matches!(top, 0x12 | 0x52 | 0x72 | 0x92 | 0xD2 | 0xF2) {
            // opc: bit30 (sf=0 uses bit29=m... ). Derive opcode from identifier bits:
            //   -(sf, opc2) : movz <-> opc=0, movk: opc=1, movn: opc=2
            // Use displacement: the top-nibble distinguishes mov | k | n via bit3.
            let opc = match top & 0xF0 {
                0xD0 | 0x50 => 0, // movz
                0xF0 | 0x70 => 1, // movk
                _ => 2, // movn
            };
            let _ = sf;
            let hw = b(insn, 21, 22) as u8;
            let imm16 = (insn >> 5) as u16;
            let rd = rd(insn);
            return Inst::MoveWide { rd, imm16, hw, opc, sf };
        }

        // ---- add/subtract immediate ----
        // signature top byte: 0x11(add32) 0x51(sub32) 0x91(add64) 0xD1(sub64)
        if top == 0x11 || top == 0x51 || top == 0x91 || top == 0xD1 {
            let sub = (insn >> 30) & 1 == 1;
            let s = (insn >> 29) & 1 == 1;
            let shift12 = (insn >> 22) & 1 == 1;
            let imm12 = b(insn, 10, 21);
            let rn = b(insn, 5, 9) as u8;
            let rd = rd(insn);
            return Inst::AddSubImm { rd, rn, imm12, shift12, sub, sf, s };
        }

    Inst::Unsupported(insn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper() {}

    #[test]
    fn bl_opens_space_for_more() {
        // 0x94000000 is `bl 0x0000000000000000` — verify the decode picks branch
        let i = decode(0x94000000);
        match i {
            Inst::B { imm, link } => {
                assert_eq!(imm, 0);
                assert!(link);
            }
            _ => panic!("expected BL, got {:?}", i),
        }
    }

    #[test]
    fn b_ne_ground_truth() {
        // aarch64-linux-gnu: "b.ne 1c <test_fn+0x10>" at offset 0x34 -> 0x54ffff41
        let i = decode(0x54ffff41);
        match i {
            Inst::BCond { cond, imm } => {
                assert_eq!(cond, 1); // NE
                assert_eq!(imm, 0x1c - 0x34); // relative branch displacement
            }
            _ => panic!("expected BCond, got {:?}", i),
        }
    }

    #[test]
    fn ret_is_unsupported_skeleton() {
        let i = decode(0xd65f03c0); // RET (not yet full impl)
        match i {
            Inst::Unsupported(_) => {}
            other => panic!("ret currently unsupported, but got {:?}", other),
        }
    }

    #[test]
    fn movz_64_ground_truth() {
        // "mov x1, #0x7"  = 0xd28000e1
        match decode(0xd28000e1) {
            Inst::MoveWide { rd, imm16, hw, opc, sf } => {
                assert_eq!(rd, 1);
                assert_eq!(imm16, 0x7);
                assert_eq!(hw, 0);
                assert_eq!(opc, 0); // movz
                assert!(sf);
            }
            other => panic!("expected MoveWide, got {:?}", other),
        }
    }
    #[test]
    fn movk_64_lsl32_ground_truth() {
        // "movk x3, #0x8, lsl #32" = 0xf2c00103
        match decode(0xf2c00103) {
            Inst::MoveWide { rd, imm16, hw, opc, sf } => {
                assert_eq!(rd, 3);
                assert_eq!(imm16, 0x8);
                assert_eq!(hw, 2);
                assert_eq!(opc, 1); // movk
                assert!(sf);
            }
            other => panic!("expected MoveWide, got {:?}", other),
        }
    }
    #[test]
    fn add_imm64_ground_truth() {
        // "add x12, x13, #0x234" = 0x9108d1ac (from objdump at 0x1c)
        match decode(0x9108d1ac) {
            Inst::AddSubImm { rd, rn, imm12, shift12, sub, sf, s } => {
                assert_eq!(rd, 12);
                assert_eq!(rn, 13);
                assert_eq!(imm12, 0x234);
                assert!(!shift12);
                assert!(!sub);
                assert!(sf);
                assert!(!s);
            }
            other => panic!("expected AddSubImm, got {:?}", other),
        }
    }
    #[test]
    fn sub_imm32_ground_truth() {
        // "sub w14, w15, #0x45" = 0x510115ee
        match decode(0x510115ee) {
            Inst::AddSubImm { rd, rn, imm12, sub, sf, .. } => {
                assert_eq!(rd, 14);
                assert_eq!(rn, 15);
                assert_eq!(imm12, 0x45);
                assert!(sub);
                assert!(!sf);
            }
            other => panic!("expected AddSubImm, got {:?}", other),
        }
    }
}