// SPDX-License-Identifier: MIT
//
// libbadcpu — x86-64 CPU feature emulator for Open Sober
//
// This library intercepts SIGILL signals raised when the Roblox Android
// binary executes instructions not supported by the host CPU, decodes
// the faulting instruction, and emulates it in software.
//
// Based on clean-room reverse engineering of Sober's libbadcpu.so.
// Architecture and semantics derived from public RE documentation,
// implementation is entirely fresh.

mod decoder;
mod cpuid;
mod emulator;

use std::sync::atomic::{AtomicBool, Ordering};

static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Install the SIGILL signal handler. Must be called once at process start.
/// Returns false if the CPU doesn't meet minimum requirements (SSE4.1).
pub fn install() -> bool {
    if HANDLER_INSTALLED.load(Ordering::SeqCst) {
        return true;
    }

    let features = cpuid::detect_features();

    if !features.has_sse41 {
        eprintln!("FATAL: CPU does not support SSE4.1 (x86-64-v2 minimum required)");
        return false;
    }

    cpuid::print_info(&features);

    // SAFETY: We set up a proper SIGILL handler with SA_SIGINFO.
    // The signal handler is async-signal-safe.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigill_handler as *const () as libc::sighandler_t;
        sa.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(libc::SIGILL, &sa, std::ptr::null_mut()) != 0 {
            eprintln!("Error: Failed to install SIGILL handler");
            return false;
        }
    }

    HANDLER_INSTALLED.store(true, Ordering::SeqCst);
    eprintln!("[libbadcpu] SIGILL handler installed");
    true
}

/// Remove the SIGILL signal handler and restore default behavior.
pub fn remove() {
    if HANDLER_INSTALLED.load(Ordering::SeqCst) {
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = libc::SIG_DFL;
            libc::sigaction(libc::SIGILL, &sa, std::ptr::null_mut());
        }
        HANDLER_INSTALLED.store(false, Ordering::SeqCst);
    }
}

/// Check if the handler is currently installed.
pub fn is_installed() -> bool {
    HANDLER_INSTALLED.load(Ordering::SeqCst)
}

extern "C" fn sigill_handler(_sig: i32, _info: *mut libc::siginfo_t, ucontext: *mut libc::c_void) {
    let ctx = ucontext as *mut libc::ucontext_t;

    // SAFETY: This runs in signal handler context. We only touch ucontext
    // and do async-signal-safe operations (no heap allocation).
    unsafe {
        let rip = (*ctx).uc_mcontext.gregs[libc::REG_RIP as usize] as *const u8;

        let features = cpuid::CpuFeatures {
            has_popcnt: true,
            has_sse41: true,
            brand_string: [0u8; 48],
            has_sse3: false,
            has_ssse3: false,
            has_sse42: false,
            has_avx: false,
            has_fma: false,
            has_avx2: false,
            has_bmi1: false,
            has_bmi2: false,
            vendor: cpuid::CpuVendor::Unknown,
        };

        let inst = decoder::decode_instruction(rip);
        let result = emulator::emulate(&inst, &features, ctx);

        match result {
            emulator::EmulationResult::Success => {}
            emulator::EmulationResult::UnrecognizedInstruction => {
                eprintln!("---------------------------------------------------------");
                eprintln!("FATAL: Unrecognized Instruction");
                eprint!("Instruction Bytes:");
                for i in 0..16usize {
                    eprint!(" {:02x}", *rip.add(i));
                }
                eprintln!();
                eprintln!("We attempted to emulate missing features, but encountered");
                eprintln!("an instruction that could not be handled.");
                std::process::exit(128 + libc::SIGILL);
            }
            emulator::EmulationResult::UnsupportedCpu => {
                eprintln!("This program requires x86-64-v2 features (SSE4.1 minimum).");
                std::process::exit(1);
            }
        }
    }
}

// Auto-initialize on library load for LD_PRELOAD-style injection
unsafe extern "C" fn init() {
    install();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_remove() {
        remove();
        assert!(!is_installed());
    }
}