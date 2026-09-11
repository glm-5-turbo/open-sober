# Fuzz repro artifacts

## fp_edge_9000_58.c / fp_edge_9000_21.c — RESOLVED (fcsel swallowed as fcvt)
Found by `gen_fp_edge` (FP-edge differential generator). Both oracles
(native x86 gcc AND qemu-aarch64) agree: result 2039651. JIT returned
2474795 (diff = +435144 = exactly a[5]*1e6).

ROOT CAUSE (NOT a register-clobber as first hypothesized): a **decode
collision**. gcc -O3 schedules a scalar `fcsel Dd,Dn,Dm,<cond>` into the
d-reg min/max chain whose rm/cond fields give it a top-16 (`0x1e65`) that
also matches the fcvt-to-int decode gate (`fcvtau`/`fcvtas`). e.g.
`fcsel d26,d28,d5,mi` = `0x1e654f9a` decoded as an `FcvtToInt`, which
writes integer X{rd} instead of vector D{rd}. So the min-accumulator
register (d26) was never updated and kept its stale `a[i]*1e6` double;
the final total carried that element (+435144).

FIX (decode.rs): the FcvtToInt gate now also requires `bits[11:10]==00`.
Every real fcvt-to-int clears bits[11:10] (!!verified: fcvtau 0x1e650062
has them 00); fcsel requires 0b11 (its cond lives in bits[15:12]). With
the guard, `0x1e654f9a` falls through to Inst::FcsSel and d26 updates
correctly. Sensitivity-proven: reverting only the `0x0c00` guard re-fails
the canary with the exact original jit 2474795 vs oracle 2039651.

Landed:
- decode regression in fp_2d_op_decode_collisions_with_int_add_bsl_and_fcvt
  (fcsel 0x1e654f9a/0x1e65ef9a -> FcsSel; fcvtau 0x1e650062 still fcvt).
- e2e canary diff_fp_edge_fcsel_swallowed_as_fcvt (fcsel_vs_fcvt).
Run: cargo build -p arm64jit --example elfjit; elfjit <(compile this at -O3
-aarch64 -static -nostdlib -Wl,-e,entry) <entry> ; diff vs native+gcc / qemu.
Now jit == oracle == 2039651.