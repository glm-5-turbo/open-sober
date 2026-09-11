//! Guest process-startup (initial-stack) bootstrap for the JIT.
//!
//! The Linux kernel starts an aarch64 user program with a specific initial
//! stack image: working down from the top of the stack are the null-padded
//! argument/environment strings, then a word image
//! `[argc][argv, NULL-term][envp, NULL-term][auxv, AT_NULL-term]`, and `sp`
//! points at `argc`. A static (no-interpreter) binary's glibc `_start`
//! walks this stack for argc/argv/envp/**auxv**.
//!
//! If that stack is garbage — as it was before this module existed — glibc
//! reads bogus `AT_HWCAP`/`AT_HWCAP2` and can IFUNC-dispatch into code the
//! JIT hasn't decoded. Real repro: `modmain.elf` (a full statically-linked
//! glibc `main(){return 300%16;}`) reached `__libc_arm_za_disable` and its
//! `str za` ZA-store loop because garbage `_dl_hwcap2` had the HWCAP2_SME bit
//! (23) set; qemu returns 12, the JIT died on an unsupported SME opcode.
//! Laying out a real initial stack — with SME/SVE/MTE/LSE feature bits
//! deliberately cleared so libc picks scalar paths the JIT fully decodes —
//! lets a full glibc boot complete.

use libloader::elf::LoadedElf;

// Auxiliary vector entry types (linux/auxvec.h, aarch64 Linux).
pub const AT_NULL: u64 = 0;
pub const AT_PHDR: u64 = 3;
pub const AT_PHENT: u64 = 4;
pub const AT_PHNUM: u64 = 5;
pub const AT_PAGESZ: u64 = 6;
pub const AT_BASE: u64 = 7;
pub const AT_FLAGS: u64 = 8;
pub const AT_ENTRY: u64 = 9;
pub const AT_HWCAP: u64 = 16;
pub const AT_CLKTCK: u64 = 17;
pub const AT_RANDOM: u64 = 25;
pub const AT_HWCAP2: u64 = 26;
pub const AT_EXECFN: u64 = 31;

/// AT_HWCAP bits we advertise: the v8.0 baseline (FP + ASIMD) only.
///
/// Deliberately NOT advertised: SVE (22), LSE/atomics (7 arg/23... LSE is
/// bit 23 of AT_HWCAP actually — see below), MTE, crypto. Advertising a
/// feature makes glibc's IFUNCs and the compiler-generated code take an
/// accelerated path; if the JIT hasn't decoded the exact instructions that
/// path uses it stops on `Unsupported`. Scalar/base-SIMD libc code is fully
/// decoded, so a minimal HWCAP is the robust choice. (The exclusive-atomic
/// LDXR/STXR the JIT emulates as plain loads/stores are a *base* instruction;
/// LSE — `casal`/`swpal` — would only be used if HWCAP_ATOMICS bit 8 is set,
/// which it is not here.)
pub const HWCAP_FP: u64 = 1 << 0;
pub const HWCAP_ASIMD: u64 = 1 << 1;

pub const AT_PHDR_TYPE: u32 = 6; // PT_PHDR

/// Build the standard auxv vector for a loaded aarch64 ELF.
///
/// `hwcap2` defaults to 0 (no SME/SVE/MTE). `AT_RANDOM` is left as a `0`
/// value; [`layout_initial_stack`] fills the 16-on-stack random bytes and
/// patches that slot so the stack canary is seeded without a syscall.
#[allow(unused)]
pub fn standard_auxv(el: &LoadedElf, hwcap: u64, hwcap2: u64) -> Vec<(u64, u64)> {
    // AT_PHDR: the program-header table's runtime address. Prefer the PT_PHDR
    // program header; else (most static binaries) it lives 64 bytes into the
    // first PT_LOAD (ELF64 Ehdr is 64 bytes).
    let phdr = el
        .info
        .phdrs
        .iter()
        .find(|p| p.p_type == AT_PHDR_TYPE)
        .map(|p| el.guest_of(p.p_vaddr))
        .or_else(|| {
            el.info
                .phdrs
                .iter()
                .find(|p| p.p_type == libc::PT_LOAD)
                .map(|p| el.guest_of(p.p_vaddr + 64))
        })
        .unwrap_or(0);

    vec![
        (AT_PHDR, phdr),
        (AT_PHENT, 56), // sizeof(Elf64_Phdr)
        (AT_PHNUM, el.info.phdrs.len() as u64),
        (AT_PAGESZ, 4096),
        (AT_FLAGS, 0),
        (AT_ENTRY, el.guest_of(el.info.entry)),
        (AT_BASE, 0), // no interpreter for a static/self-loaded image
        (AT_HWCAP, hwcap),
        (AT_HWCAP2, hwcap2),
        (AT_CLKTCK, 100),
        (AT_RANDOM, 0), // filled by layout_initial_stack
    ]
}

/// Lay out the kernel's initial-stack image at the top of the guest stack
/// region and return the guest SP (== host pointer when guest==host) to place
/// in x31.
///
/// Layout from the returned `sp` upward:
/// ```text
/// [sp+0x00] argc
/// [        ] argv[]  NULL-terminated
/// [        ] envp[]  NULL-terminated
/// [        ] auxv[]  AT_NULL-terminated
/// ```
/// The argv0/env strings and the AT_RANDOM 16-byte block live just above the
/// word image. If any auxv entry has type [`AT_RANDOM`] with value `0`, that
/// slot is patched to point at a freshly filled 16-byte block so the glibc
/// stack canary has real entropy without a getrandom syscall.
#[allow(unused)]
pub fn layout_initial_stack(
    stack: *mut u8,
    stack_size: usize,
    argv0: Option<&[u8]>,
    env: &[&[u8]],
    auxv: &mut [(u64, u64)],
) -> u64 {
    assert!(!stack.is_null());
    let top: u64 = stack as u64 + stack_size as u64;
    let argc: u64 = if argv0.is_some() { 1 } else { 0 };
    let has_random = auxv.iter().any(|&(t, val)| t == AT_RANDOM && val == 0);
    let mut str_bytes: usize =
        argv0.map_or(0, |s| s.len() + 1) + env.iter().map(|e| e.len() + 1).sum::<usize>();
    if has_random {
        str_bytes += 16; // AT_RANDOM block
    }
    let n_words = 1 + argc as usize + 1 + env.len() + 1 + 2 * auxv.len() + 2;
    let word_bytes = n_words * 8;
    let sp = (top - (word_bytes + str_bytes) as u64) & !15u64;
    let mut cur = sp + word_bytes as u64;

    // AT_RANDOM 16-byte block (entropy for the glibc stack canary).
    let random_ptr = if has_random {
        let p = cur;
        cur += 16;
        p
    } else {
        0
    };

    // Place argv0 + env strings.
    let argv0_ptr = if let Some(s) = argv0 {
        let p = cur;
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), cur as *mut u8, s.len()) };
        cur += s.len() as u64;
        unsafe { *(cur as *mut u8) = 0 };
        cur += 1;
        p
    } else {
        0
    };
    let env_ptrs: Vec<u64> = env
        .iter()
        .map(|e| {
            let p = cur;
            unsafe { std::ptr::copy_nonoverlapping(e.as_ptr(), cur as *mut u8, e.len()) };
            cur += e.len() as u64;
            unsafe { *(cur as *mut u8) = 0 };
            cur += 1;
            p
        })
        .collect();

    // Patch AT_RANDOM in place.
    if has_random {
        // Fill 16 bytes with a cheap, reproducible source of entropy.
        let seed = top ^ 0x9e3779b97f4a7c15u64;
        let mut b = seed;
        for i in 0..16usize {
            // xorshift64
            b ^= b << 13;
            b ^= b >> 7;
            b ^= b << 17;
            unsafe { *(random_ptr as *mut u8).add(i) = b as u8 };
        }
        for slot in auxv.iter_mut() {
            if slot.0 == AT_RANDOM && slot.1 == 0 {
                slot.1 = random_ptr;
            }
        }
    }

    // Write the word image from sp upward.
    let mut wp = sp;
    let mut put = |v: u64, wp: &mut u64| {
        unsafe { *(*wp as *mut u64) = v };
        *wp += 8;
    };
    put(argc, &mut wp);
    put(argv0_ptr, &mut wp);
    put(0, &mut wp); // argv NULL terminator
    for p in env_ptrs {
        put(p, &mut wp);
    }
    put(0, &mut wp); // envp NULL terminator
    for &(t, v) in auxv.iter() {
        put(t, &mut wp);
        put(v, &mut wp);
    }
    put(AT_NULL, &mut wp);
    put(0, &mut wp);

    sp
}