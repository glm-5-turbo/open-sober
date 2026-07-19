### 📈 Remaining Blockers (July 19 session 2)

1. **🔴 QEMU + libroblox.so code execution bug** — libroblox.so (101MB, NDK r28c) has a ~96MB text segment. QEMU user-mode 10.2.1 (both system and custom-built) cannot execute code from JNI_OnLoad's original text page (offset ~0x1f64000). Other text pages (e.g., offset 0x1d77000) execute fine. The exact same instructions work when copied to a fresh `mmap(PROT_EXEC)` page.
   - `__builtin___clear_cache` does NOT fix it
   - `mprotect` to r+x does NOT fix it  
   - Suspected QEMU TCG bug with the specific huge page mapping
   - **Workaround #1:** Copy JNI_OnLoad to exec page, patch `adrp` PC-relative offsets to point to real GOT. Partially tested — adrp fix works but encoding was off (got SIGILL instead of SIGSEGV, so progress).
   - **Workaround #2:** Try a different/older Roblox APK

2. **Canary GOT patching works** — Verified that `mprotect` + GOT write successfully patches the `__stack_chk_guard` GOT entry.

3. **Bridge libraries work** — libc.so, libm.so, libdl.so with LIBC_* version tags compile and load.

4. **JNI function table is complete** — All 256 slots filled.

### 🟢 What I'd Do Next (Prioritized)

1. **Fix JNI_OnLoad execution** — Either implement the exec-page copy with adrp fixup in `jni_shim.c`, OR try an older APK
2. **Debug through JNI_OnLoad** once code executes
3. **EGL/GLES + graphics** — Mesa+zink stubs
4. **Window + input** — SDL2-based

**Custom QEMU at:** `/tmp/qemu-10.2.1/build/qemu-aarch64` (41MB static, VDSO disabled)