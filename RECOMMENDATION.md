# Open Sober - Binary Translation Approach Recommendation

## Executive Summary

The Open Sober project needs to run the ARM64 Android Roblox APK on x86-64 Linux. This requires solving three fundamentally distinct problems:

1. **CPU instruction translation**: ARM64 machine code -> x86-64 machine code (at runtime)
2. **System library translation**: Android Bionic libc -> Linux glibc
3. **Android framework translation**: JNI, asset manager, looper, input, EGL/GLES

**Recommended approach: Hybrid - QEMU user-mode for CPU translation + android2gnulinux for Bionic/glibc compatibility + custom Android framework wrappers.**

This is the most practical path to a working prototype. A fully custom JIT (like the oaknut+dyncall approach Sober uses) is a longer-term optimization that can replace QEMU once the system works end-to-end.

---

## 1. Problem Analysis

### What the Roblox APK Contains

The Android Roblox APK is primarily:
- ARM64 native libraries (.so files) compiled for `aarch64-linux-android`
- Linked against **Bionic libc** (Android's C library, not glibc)
- Using the **Android linker** (`linker64`) which is incompatible with ld-linux
- Making JNI calls to the Android framework (Java classes)
- Using Android-specific APIs: `AAssetManager`, `ALooper`, `AInputQueue`, `EGL/GLES`

### Critical Insight

The Roblox APK's native code is **already Linux user-space code** - it makes Linux system calls, uses pthreads, allocates memory, etc. The differences are:
- **Binary format**: ARM64 instructions vs x86-64 instructions
- **C library**: Bionic symbols vs glibc symbols (largely compatible at the syscall level)
- **Linker format**: Android's `linker64` uses different TLS, auxiliary vector handling etc.
- **Android framework APIs**: JNI calls to Java/Android runtime

---

## 2. Approach Evaluation

### Approach A: QEMU User Mode

| Aspect | Assessment |
|--------|-----------|
| **Direction** | QEMU user-mode runs ARM64 binaries on x86-64 -- exactly what we need |
| **Maturity** | Very mature (20+ years), used in production everywhere |
| **Performance** | ~5-10x slowdown for compute, moderate for syscall-heavy workloads |
| **Embeddability** | QEMU is NOT designed as a library. No libqemu API exists |
| **Licensing** | GPL v2 -- viral, must be a separate process |
| **Reuse** | Drop-in via `qemu-aarch64` subprocess or `binfmt_misc` |

**How it would work:**
```
x86-64 process
  |
  +-> fork/exec qemu-aarch64 ./libroblox.so
  |       |
  |       +-> ARM64 JIT translation (TCG)
  |       +-> Syscall translation (ARM64 Linux -> x86-64 Linux)
  |       +-> Signal translation
  |
  +-> Host graphics wrapper (EGL/GLES pass-through)
  +-> Bionic compatibility layer (LD_PRELOAD or linker trick)
```

**Key considerations:**
- QEMU user-mode has no stable library API. The project explicitly builds standalone executables. To embed QEMU, you would need to fork QEMU's source, strip it down to a library, and maintain it yourself -- a massive undertaking.
- Using `qemu-aarch64` as a **subprocess** is straightforward. You set up the ARM64 sysroot, set `QEMU_LD_PREFIX`, and run.
- Communication between the host process and the QEMU subprocess requires some IPC mechanism (shared memory, sockets, etc.)
- Recent versions of QEMU user-mode (9.0+) have significantly improved performance with the `TCG` backend and block chaining optimizations.
- **binfmt_misc** support means the Linux kernel can auto-invoke QEMU for ARM64 binaries, making execution transparent.

**Verdict:** Best for initial prototype. Use as subprocess, not embedded library. Acceptable performance for a game where GPU is the bottleneck.

### Approach B: FEX-Emu

| Aspect | Assessment |
|--------|-----------|
| **Direction** | x86(-64) -> ARM64 **only**. Not reversible |
| **Maturity** | Very mature (7700+ stars, active development) |
| **Reverse direction** | FEX has no ARM64 -> x86-64 mode. Building one would be building a new emulator |
| **Embeddability** | Not designed as library, is a standalone launcher |
| **Reuse potential** | The IR design and thunking architecture are excellent references |

**Why FEX doesn't work in reverse:**
- FEX's frontend parses x86 instructions and emits its own IR (which "loosely models ARM64")
- FEX's backend (called "Splatter") emits ARM64 code from the IR
- Neither component has any ARM64 decoding or x86 emission capability
- Reversing FEX would mean building a completely new frontend (ARM64 decoder -> IR) and a completely new backend (IR -> x86-64 emitter). This is essentially building a new emulator from scratch, reusing only the IR definitions and optimization passes.

**What we CAN learn from FEX:**
- The **thunking architecture** (Guest.cpp/Host.cpp pairs) for wrapping native libraries is excellent
- The **IR design** (SSA form, loosely modeled on ARM64) is a good reference
- The **code cache** and translation block management patterns
- The **Splatter backend** concept (concatenating configurable macros instead of instruction selection) is interesting for performance

**Verdict:** Not directly usable. The reverse direction would require a complete rewrite. Use as a design reference only.

### Approach C: Static Recompilation (Ghidra/Sleigh)

| Aspect | Assessment |
|--------|-----------|
| **Feasibility** | Extremely difficult for a real-time application |
| **Sleigh** | Ghidra's Sleigh is a processor specification language, not a runtime JIT engine |
| **Sleigh license** | Apache 2.0 -- could be used |
| **Box86/Box64 model** | Uses Dynarec (dynamic recompilation), NOT static recompilation |

**Why static recompilation is impractical:**
- Roblox is a dynamic application with massive code: over 100MB of native libraries, JIT-compiled Lua bytecode, dynamically loaded modules
- AOT recompilation cannot handle: dynamic loading (dlopen), self-modifying code, computed jumps, virtual dispatch, reflection
- Even with perfect static analysis, you'd need a dynamic fallback for anything unpredictable
- Ghidra's Sleigh is an **interpreter** for processor specifications, not a code generation engine. It's designed for disassembly, not JIT compilation.

**What Box86/Box64 does:**
- Uses **Dynarec** (dynamic recompilation) - translates blocks of x86 to ARM at runtime
- **Wraps native libraries** - when guest code calls libc/fopen(), it routes to the host ARM version of libc
- This is the same model we need, just in reverse (ARM64 guest -> x86-64 host)
- Box86's approach of wrapping system libraries rather than emulating them is the **performance key**

**Verdict:** Static recompilation is not viable. Dynamic recompilation (JIT) is the only practical approach. Box86's architecture is worth studying for our own Dynarec.

### Approach D: oaknut + dyncall Custom JIT

| Aspect | Assessment |
|--------|-----------|
| **oaknut** | AArch64 **assembler** (emits ARM64 code), MIT license. **Cannot emit x86-64 code** |
| **dyncall** | Cross-platform FFI for dynamic function calls. Can bridge calling conventions |
| **Missing: disassembler** | oaknut is an assembler only. We need an ARM64 **disassembler** (like Capstone) |
| **Missing: x86-64 assembler** | We need a library to emit x86-64 code at runtime |
| **Work required** | Building a full ARM64->x86-64 JIT is a massive project |

**What Sober actually uses:**
- Sober uses oaknut to **generate ARM64 code** because Sober itself runs on ARM64 Linux
- Sober uses dyncall for the **calling convention bridge** between ARM64 and the host
- Sober does NOT need CPU instruction translation because it already runs on ARM64 hardware
- Sober's real work is: Bionic->glibc translation + Android framework emulation

**The misunderstanding corrected:**
Sober is a **native ARM64 binary** that runs the Android ARM64 Roblox APK directly on ARM64 Linux. It does binary translation of the **library layer** (Bionic -> glibc) using symbol interposition and dyncall for ABI compatibility. It does NOT do any instruction set translation.

**For Open Sober (x86-64 host):**
We need to build:

1. **ARM64 disassembler** - Capstone (BSD license), LLVM's MC layer, or a custom decoder
2. **ARM64 semantics/emulation** - understanding what each ARM64 instruction does
3. **x86-64 assembler** - a library like **asmjit** (x86-64 JIT assembler, open source) or **Xbyak** to emit x86-64 code
4. **Register mapping** - ARM64 has 31 GPRs (x0-x30) + V0-V31 (SIMD/FP) vs x86-64 has 16 GPRs + XMM0-XMM15. This requires register allocation.
5. **Condition code translation** - ARM64 uses NZCV flags heavily. x86-64 uses EFLAGS. Different semantics.
6. **Memory model** - ARM64 has weak memory ordering. x86-64 has strong ordering. Need memory barriers.
7. **JIT engine** - translation block cache, chaining, invalidation, self-modifying code support

This is a **multi-person-year** effort to build anything usable.

**Tool options for x86-64 code generation:**
- **asmjit** (github.com/asmjit/asmjit): x86-64/x86 JIT assembler, permissive license. Most mature option.
- **Xbyak** (github.com/herumi/xbyak): C++ JIT assembler for x86-64, used by many emulators
- **Zydis** (github.com/zyantific/zydis): fast x86-64 decoder/encoder (for generating x86-64 binary data)

**Verdict:** This is the long-term goal but far too complex for an initial implementation. Use QEMU first, then migrate to a custom JIT once the system is proven.

### Approach E: android2gnulinux + apkenv

| Aspect | Assessment |
|--------|-----------|
| **android2gnulinux** | Bionic->glibc compatibility layer. Custom linker. C/C++, 84 stars |
| **apkenv** | Abandoned project, original domain dead, minimal remaining info |
| **Bionic translation (GitLab)** | Unclear status, 121 commits, no license visible |
| **libhybris** | Loads Android bionic .so's in glibc processes. ARM64 Linux only, not x86 |
| **Reuse for ARM64->x86-64** | android2gnulinux handles ABI layer, but CPU translation is separate problem |

**What android2gnulinux provides:**
1. **Custom linker** - loads bionic-linked ELF binaries and resolves Bionic symbols
2. **Runtime libraries** - implements bionic functions (bionic_open, etc.) mapping to glibc equivalents
3. **JNI/JVM shim** - `libjvm-*.so` that intercepts JNI calls and routes to native C implementations
4. **Porting support** - same libraries work for porting (recompiling Android code) and runtime (running bionic-linked binaries)

**Important limitation:** android2gnulinux assumes the guest binary is **native** (same architecture as host). It handles the OS/ABI layer, not the CPU instruction layer. For Open Sober, we would need to combine android2gnulinux with a CPU translator (like QEMU user-mode).

**apkenv status:** Effectively dead. Can provide design inspiration but not reusable code.

**Verdict:** Highly reusable for the Bionic/glibc layer. Combine with QEMU for CPU translation. This is the best path forward.

---

## 3. Recommended Architecture

### Phase 1: Proof of Concept (Estimated: 2-4 weeks)

```
+------------------------------------------+
|            Host Process (x86-64)          |
|                                           |
|  +------------------+  +----------------+ |
|  | android2gnulinux |  |  Graphics      | |
|  | runtime libs     |  |  Wrapper       | |
|  | (bionic->glibc)  |  |  (EGL/GLES)    | |
|  +--------+---------+  +-------+--------+ |
|           |                      |        |
|  +--------v----------------------v------+ |
|  |      QEMU User Mode (qemu-aarch64)   | |
|  |       (ARM64 JIT + Syscall Bridge)   | |
|  |                                       | |
|  |  +----------------------------------+ | |
|  |  | Roblox ARM64 Native Libraries    | | |
|  |  | (libroblox.so, etc.)             | | |
|  |  +----------------------------------+ | |
|  +---------------------------------------+ |
+------------------------------------------+
```

**Step-by-step implementation:**

1. **Set up ARM64 QEMU user-mode environment**
   ```bash
   # Install QEMU user-mode
   sudo apt install qemu-user qemu-user-binfmt
   
   # Create ARM64 sysroot with android2gnulinux runtime
   mkdir -p ~/arm64-sysroot/lib
   # Copy android2gnulinux runtime libs into sysroot
   cp runtime/*.so ~/arm64-sysroot/lib/
   
   # Extract Roblox APK native libs
   unzip Roblox.apk -d ./roblox-apk
   
   # Run via QEMU
   qemu-aarch64 -L ~/arm64-sysroot ./roblox-apk/lib/arm64-v8a/libroblox.so
   ```

2. **Integrate android2gnulinux runtime libraries**
   - Clone and build `github.com/Cloudef/android2gnulinux`
   - Use its `runtime/` directory as the ARM64 sysroot
   - The custom linker (`andre`) loads bionic-linked .so files
   - Runtime libs provide `bionic_*` symbol implementations that call glibc

3. **Graphics pass-through**
   - Roblox uses OpenGL ES 3.x via EGL
   - QEMU user-mode forwards ioctl calls, but GPU calls need actual hardware
   - Set up EGL/GLES wrapper that connects host GPU drivers
   - Use the host's native OpenGL driver but translate EGL/GLES calls

4. **Android framework shims**
   - Implement `AAssetManager` over host filesystem
   - Implement `ALooper` over epoll
   - Implement `AInputQueue` over X11/Wayland input
   - Provide `libandroid.so`, `libEGL.so`, `libGLESv2.so` stubs

### Phase 2: Optimization (Estimated: 1-2 months)

**Replace QEMU subprocess with tighter integration possibilities:**

Option 1: **QEMU as a library (forked)**
- Fork QEMU source, extract TCG and user-mode emulation
- Expose a C API: `init_emulator()`, `translate_block()`, `execute()`
- Complexity: High (QEMU internals are complex, GPL license concerns)
- Benefit: Eliminates IPC overhead, tighter integration

Option 2: **Custom ARM64 interpreter first, JIT later**
- Build a simple ARM64 interpreter using Capstone for decoding
- Start with common instructions: ADD, SUB, MOV, LDR, STR, B, BL, RET
- Expand coverage as needed
- Complexity: Medium (Capstone does the heavy lifting)
- Performance: Poor initially (interpretation is ~50-100x slower)

Option 3: **Direct call optimization**
- Identify "hot" functions via profiling
- Pre-translate known functions (like Box86's wrapped libraries)
- For library calls: use dyncall to bridge ARM64 calling convention to x86-64

### Phase 3: Custom JIT (Long-term goal: 3-6 months)

Build a specialized ARM64->x86-64 JIT:

```
ARM64 Code Block
      |
      v
Capstone (disassembler)
      |
      v
[Translation Logic]
      |
      +---> Register mapping (31 ARM regs -> 16 x86 regs + stack spill)
      +---> Instruction mapping (ARM64 op -> x86-64 op sequence)
      +---> Condition code mapping (NZCV -> EFLAGS)
      +---> Memory ordering (barriers for weak->strong model)
      |
      v
asmjit / Xbyak (x86-64 emitter)
      |
      v
x86-64 Code Block (executable)
      |
      v
Execute natively on CPU
```

**Key components needed:**

1. **ARM64 Decoder**: Capstone (BSD license, battle-tested)
2. **x86-64 Encoder**: asmjit (permissive, actively maintained, C++)
3. **Register Allocator**: Simple linear scan for ARM64->x86-64 mapping (x86 has 16 GPRs vs 31 on ARM - need stack spill for ~15)
4. **Instruction Translator**: Hand-written translation rules for the ~200 most common ARM64 instructions
5. **Block Linker**: Chain translated blocks together like QEMU's goto_tb mechanism
6. **Code Cache**: Manage translated blocks with LRU eviction

**What we DON'T need to build from scratch:**
- ARM64 decoding -> Capstone
- x86-64 encoding -> asmjit or Xbyak
- Executable memory management -> oaknut's CodeBlock/DualCodeBlock utilities
- Calling convention bridging -> dyncall

**What we MUST build from scratch:**
- ARM64 semantic analysis (what each instruction does)
- ARM64->x86-64 instruction mapping tables
- Register allocation (31->16 with spilling)
- Condition code and flags translation
- Memory ordering barrier insertion
- Block chaining and code cache management
- Syscall emulation layer

**Estimated complexity:**
- ARM64 instruction set: ~500-800 instructions total, ~200-300 commonly used
- Each instruction needs 1-10 x86-64 instructions to implement
- Total: ~1000-3000 lines of translation rules per instruction category
- Overall JIT: ~15,000-30,000 lines of C++

---

## 4. Reusable Open-Source Components

| Component | Project | License | What it provides |
|-----------|---------|---------|------------------|
| **ARM64 JIT execution** | QEMU user-mode | GPL v2 | Complete ARM64->x86-64 translation, syscall bridge |
| **Bionic->glibc translation** | android2gnulinux | (check) | Custom linker, bionic runtime libs, JNI shim |
| **ARM64 disassembler** | Capstone | BSD | Reliable ARM64 instruction decoding |
| **x86-64 assembler** | asmjit | Zlib | Runtime x86-64 code generation |
| **x86-64 assembler** | Xbyak | BSD | Alternative x86-64 JIT assembler |
| **ARM64 assembler** | oaknut | MIT | ARM64 code generation (for future ARM64 JIT) |
| **Calling convention FFI** | dyncall | (check) | Bridge ARM64/x86-64 calling conventions |
| **ARM spec reference** | LLVM TableGen | Apache 2 | ARM64 instruction definitions/semantics |
| **Graphics bridge** | FEX ThunkLibs | MIT | EGL/GLES guest-host thunk pattern |
| **TLS/bionic fix** | libhybris | LGPL | Bionic TLS slot handling |
| **Signal handling** | QEMU signals | GPL v2 | ARM64 signal frame conversion to x86-64 |

---

## 5. Build Plan: Step by Step

### Step 1: Environment Setup (Week 1)

```bash
# Dependencies
sudo apt install qemu-user qemu-user-binfmt aarch64-linux-gnu-gcc

# Get android2gnulinux
git clone https://github.com/Cloudef/android2gnulinux.git
cd android2gnulinux
make runtime  # Build the runtime libraries

# Get Capstone for ARM64 decoding
git clone https://github.com/capstone-engine/capstone.git

# Get asmjit for x86-64 generation
git clone https://github.com/asmjit/asmjit.git

# Get dyncall
hg clone http://hg.dyncall.org/pub/dyncall/dyncall
```

### Step 2: QEMU + android2gnulinux Integration (Week 2-3)

1. Build android2gnulinux runtime libraries for ARM64
2. Create ARM64 sysroot directory with these libraries
3. Test running a simple ARM64 Android native binary via QEMU
4. Ensure android2gnulinux's custom linker works with QEMU
5. Test with a non-graphics Android binary

### Step 3: Graphics and Input (Weeks 3-4)

1. Implement EGL stub library that connects to host GPU
2. Implement GLESv2 wrapper (pass-through to host GL)
3. Implement AAssetManager over host filesystem
4. Implement ALooper/AInputQueue over X11/Wayland/Wayland
5. Get Roblox to boot (even with graphical artifacts)

### Step 4: Custom JIT Skeleton (Weeks 5-8)

1. Build Capstone-based ARM64 decoder
2. Implement register mapping (ARM64->x86-64 with spilling)
3. Implement 50 most common instructions (data processing, loads/stores, branches)
4. Build code cache (translation block storage + lookup)
5. Implement block chaining (direct branch between translated blocks)
6. Test with simple ARM64 programs

### Step 5: Full JIT (Months 3-6)

1. Expand instruction coverage (add SIMD/FP, atomics, crypto)
2. Add system call emulation
3. Add signal handling
4. Performance optimization: inline cache, hot-block optimization
5. Replace QEMU backend with custom JIT

---

## 6. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| **QEMU performance too low for Roblox** | Medium | GPU-bound games spend most time in GPU calls, not CPU. QEMU is acceptable for CPU tasks. |
| **android2gnulinux unstable** | Medium | Can contribute fixes upstream or fork. The Bionic/glibc API surface is well-defined. |
| **Graphics wrapper too complex** | High | Use Waydroid's approach: real Android GLES stack in container with host GPU pass-through via ANativeWindow |
| **JIT complexity underestimated** | High | Start with QEMU, treat custom JIT as optional optimization |
| **License incompatibility** | Low | GPL (QEMU) used as separate process is fine. LGPL/BSD/MIT for all other components. |
| **Anti-cheat / integrity checks** | Medium | Roblox's byfron/hyperion may detect emulation. QEMU's translation is transparent but timing differences exist. |
| **Android linker TLS** | Medium | android2gnulinux's custom linker handles this. May need libhybris patches for TLS slot numbering. |

---

## 7. Final Recommendation

### Phase 1 (MVP): Use QEMU user-mode + android2gnulinux

```
qemu-aarch64 -L ./arm64-sysroot ./roblox-libs/libroblox.so
```

- **What works**: All CPU translation, syscall translation, signal handling
- **What we build**: android2gnulinux integration, graphics wrapper, Android framework shims
- **Timeline**: 3-4 weeks to a booting Roblox
- **Performance**: Acceptable (GPU is bottleneck for games)

### Phase 2 (Optimization): Custom JIT with Capstone + asmjit

- **What we build**: ARM64 decoder, instruction translator, x86-64 emitter, code cache
- **Reuse**: Capstone (decode), asmjit (emit), dyncall (ABI bridge), oaknut (memory management)
- **Timeline**: 3-6 months for full JIT
- **Performance**: Potentially 2-3x faster than QEMU for CPU-bound code

### Why Not the Other Approaches

- **FEX-Emu**: Wrong direction (x86->ARM only). Would need complete rewrite.
- **Static recompilation**: Impractical for dynamic game runtime.
- **Pure oaknut+dyncall JIT**: Correct eventual approach but too much to build initially. Better to iterate from something working.

---

## 8. References

| Project | URL | Key Takeaway |
|---------|-----|-------------|
| QEMU User Mode | qemu.org | Most mature ARM64->x86-64 JIT available |
| android2gnulinux | github.com/Cloudef/android2gnulinux | Best Bionic->glibc compatibility layer |
| Capstone | capstone-engine.org | Battle-tested ARM64 disassembler (BSD) |
| asmjit | github.com/asmjit/asmjit | Production-ready x86-64 JIT assembler |
| oaknut | github.com/merryhime/oaknut | AArch64 assembler (for reference, MIT) |
| dyncall | dyncall.org | Multi-architecture calling convention bridge |
| FEX-Emu | github.com/FEX-Emu/FEX | Design reference for thunking and IR (MIT) |
| Box86/Box64 | box86.org | Design reference for Dynarec + library wrapping |
| libhybris | github.com/libhybris/libhybris | Bionic TLS/stub patterns (LGPL) |
| Waydroid | waydro.id | Container-based Android runtime (design reference) |

---

## Appendix: Key Architectural Differences

### ARM64 AArch64 vs x86-64 System V Calling Convention

| Aspect | ARM64 | x86-64 |
|--------|-------|--------|
| Integer arg registers | x0-x7 (8) | rdi, rsi, rdx, rcx, r8, r9 (6) |
| Float arg registers | v0-v7 (8) | xmm0-xmm7 (8) |
| Return address | x30 (LR) register | Return address on stack |
| Callee-saved regs | x19-x28 (10) | rbx, rbp, r12-r15 (6) |
| Stack alignment | 16-byte at call | 16-byte at call |
| Frame pointer | x29 (FP) explicitly | rbp (optional) |
| Condition codes | NZCV in dedicated register | EFLAGS (implicit) |

### Key Translation Challenges

1. **Register count**: ARM64 has 31 GPRs vs 16 in x86-64. Need ~15 spill slots on stack.
2. **Link register**: ARM64's `x30` (LR) means leaf functions don't touch the stack. x86-64's return address is on the stack. Every BL (branch-and-link) needs a stack push/pop in x86-64.
3. **Condition codes**: ARM64's NZCV flags are set optionally (via `S` suffix on instructions). x86-64 EFLAGS are set implicitly by most ALU ops. Must carefully track when to preserve/restore flags.
4. **SIMD/FP**: ARM64 NEON has 32x128-bit registers. x86-64 SSE has 16x128-bit. AVX-512 adds more but alignment and semantics differ.
5. **Memory ordering**: ARM64 is weakly ordered (loads can be reordered with stores). x86-64 is strongly ordered (stores are ordered). ARM64 code expects acquire/release semantics via `LDAR`/`STLR`. These need memory barrier instructions (`mfence`, etc.) on x86-64.

---

*This document represents the best approach based on current research. The key insight is that separating the CPU translation problem (solved by QEMU) from the OS/ABI translation problem (solved by android2gnulinux) allows incremental progress toward a full custom JIT solution.*
