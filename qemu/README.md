# Open Sober QEMU (SMC-patched)

The runtime runs the Roblox ARM64 Android binary under QEMU user-mode
emulation.  Roblox self-modifies code pages at load time (canary patching,
JIT regions, our own in-process code patches in `jni_shim.c`).  Stock QEMU
(`qemu-aarch64` 10.2.1) chained translation blocks with direct native
`goto_tb` jumps; when a page was rewritten after being translated once, the
stale chain target caused **host SIGSEGV**.  This was the interminable
self-modifying-code (SMC) crash documented across sessions 5–11.

This directory contains a reproducible build of QEMU 10.2.1 with two small
TCG changes that eliminate the SMC hazard.

## Patches

| Patch | File | Effect |
|---|---|---|
| `0001-...-CF_NO_GOTO_TB.patch` | `accel/tcg/cpu-exec-common.c` | Force `CF_NO_GOTO_TB` on **every** TB in `curr_cflags()`. The translator then never emits `goto_tb` chained jumps; each block exit goes through the TB hash, so rewritten code is always re-translated from current guest bytes. |
| `0002-...-tb_set_jmp_target.patch` | `accel/tcg/cpu-exec.c` | Make `tb_set_jmp_target()` a no-op as belt-and-suspenders, so even a stray link attempt never patches a native jump into the translated text page. |

Together they guarantee deterministic SMC: a self-rewriting guest page can
never be executed through a stale chained jump.

## Build

```bash
./build.sh                 # downloads QEMU 10.2.1, patches, builds, installs
./build.sh /path/qemu-10.2.1.tar.xz   # offline, using a local tarball
```

Output: `./out/qemu-aarch64` (user-mode emulator, aarch64 target only).

## Install into the runtime cache

The `sober-core` launcher (`crates/sober-core/src/qemu.rs`) invokes the
emulator via `SoConfig.qemu_path`, defaulting to the patched binary at
`~/.cache/open-sober/qemu-patched`:

```bash
cp qemu/out/qemu-aarch64 ~/.cache/open-sober/qemu-patched
```

## Notes

- This build only produces the `aarch64-linux-user` target to keep the
  build lean (~1-2 GB build tree, 39 MB final binary).
- QEMU 10.2.1 source is 141 MB; the build needs a few hundred MB working
  space plus the ninja build.