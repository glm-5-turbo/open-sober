# Open Sober

Open-source reimplementation of the VinegarHQ Sober Roblox Linux runtime:
run the **Android ARM64 Roblox APK natively on x86-64 Linux** via binary
translation, bypassing Hyperion (Roblox's anticheat, not present in the
Android builds). The original Sober is closed-source — this is a
clean-room reimplementation.

> **Status (honest):** the runtime is a working, heavily-tested ARM64→x86-64
> JIT + ELF loader + Android syscall/GLES machinery, but it does **not yet
> boot a real Roblox session**. The go/no-go gate (binary loads → JNI inits →
> main loop runs → first frame, captured as a run log on a GPU host) is
> **unmet**, because it requires a real `libroblox.so`/APK and a GPU-capable
> host — neither is present on the development box. Everything below is
> implemented to that point and verified headlessly.

## Architecture

| Crate | Status | Description |
|---|---|---|
| **arm64jit** | ✅ core | AArch64→x86-64 JIT. Decodes/compiles guest ARM64 by executing x86-64 directly (no QEMU/TCG). JIT-ed binary loader, dynamic-linking (PLT/GLOB_DAT), syscall (`svc`) emulation, TLS (all models incl. cross-module), JNI native-method bridge, guest threads (`clone`), signal handling. |
| **libbadcpu** | ✅ | x86-64 CPU-feature emulator (SIGILL handler) for instructions x86 CPUs may lack: POPCNT, MOVBE, LZCNT, TZCNT, BMI1 (ANDN/BLSI/BLSMSK/BLSR), BMI2 (MULX, RORX, ADCX, ADOX, BEXTR, BZHI, SHRX/SARX/SHLX). |
| **libloader** | ✅ | Android-oriented ELF loader + process spawn/sandbox: chroot isolation, ELF loading, `DT_NEEDED` dependency chains, Android **packed relocations (APS2)**, **DT_RELR**, RELATIVE/GLOB_DAT/ABS64 relocs. |
| **sober-core** | ✅ | Main orchestrator binary `open-sober` (`open-sober play --apk roblox.apk`). Hosts the JIT path, guest boot/auxv, Android env/layout. |
| **sober-services** | ✅ | Roblox OAuth login handler (webview auth, token/user_id via IPC). |
| **egl-wrapper** | ✅ | cdylib exposing the guest's `libEGL.so` soname; dlopen's Mesa's real `libEGL.so.1` and forwards EGL entry points. |
| **glesv2-wrapper** | ✅ | cdylib `libGLESv2.so`; dlopens Mesa and forwards GLES entry points, intercepting `glCompressedTexImage2D` to decode Android texture formats to RGBA. |
| **input-wrapper** | ✅ | Android touch/key event model; raw-X11 (x11rb) pointer/keyboard → Android `AKEYCODE` / touch translation. |
| **texture-codec** | ✅ | Pure-Rust decoders for ETC1/ETC2/EAC/ASTC/ATC → RGBA8, shared by the GLES wrapper and the JIT resolver. |

## How It Works

1. **sober-services** handles Roblox OAuth; auth passes to **sober-core**.
2. **libloader** loads the ARM64 APK's `libroblox.so` and its `DT_NEEDED`
   dependency chain (contiguous mapped image).
3. **arm64jit** translates each guest ARM64 basic block on demand and executes
   x86-64, resolving imports through the PLT/GOT and emulating `svc` syscalls,
   TLS, threading, and JNI calls. `libbadcpu` catches any host CPU gaps.
4. Graphics calls (EGL/`gl*`) are bound to Mesa through the **egl/glesv2
   wrappers** (optionally `MESA_LOADER_DRIVER_OVERRIDE=zink` for GLES→Vulkan);
   Android compressed textures are decoded via **texture-codec**.

## arm64jit — verified ISA surface

Guarded by ~360 workspace tests and a **differential fuzzer** (`fuzz_jit.py`)
that runs the same C through the JIT *and* native x86-64 / `qemu-aarch64`
oracles and requires exact equality:

- Scalar + vector FP/SIMD (NEON), int/FP conversions, bitfield/logic-immediate,
  wide multiply, saturating ops, reductions, TLS (local-exec, initial-exec,
  global-dynamic `__tls_get_addr`, cross-module TPREL64/TLSDESC).
- Loader: multi-module `DT_NEEDED`, `RELATIVE`/`GLOB_DAT`/`ABS64`, APS2 and
  DT_RELR relocations, cross-module PLT/GLES binding.
- Syscall `svc` emulation (sockets, epoll, fstat/stat, TLS `clone`-threads,
  signal blocking) and a JNI native-method registry.

## Build & test

```bash
cargo build --workspace     # debug
cargo test --workspace      # ~360+ tests, all green (climbs every cycle)
```

Run a small statically-linked ARM64 example with no QEMU:

```bash
cargo run -p arm64jit --example elfjit -- <program.elf> <entry_addr>
```

Graphics plumbing is verified headlessly (Mesa **llvmpipe** software renderer +
`surfaceless` EGL / Xvfb) — a GPU is only needed for a real frame.

## License

MIT