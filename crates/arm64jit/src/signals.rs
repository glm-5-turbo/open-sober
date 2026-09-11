// SPDX-License-Identifier: MIT
//
// Guest signal handling for the arm64jit runtime. Serves the Linux signal
// contract to translated AArch64 guest code:
//
//   - `rt_sigaction` (134) records SIG_DFL / SIG_IGN / a guest handler fn.
//   - `kill` (129) / `tgkill` (131) deliver to a guest thread: a SAME-thread
//     (self) signal runs the handler synchronously; a DIFFERENT guest thread
//     gets a cooperative `pending_signal` that its own dispatcher loop picks up.
//   - A dispatched handler is entered as a guest function with the aarch64
//     signal ABI (x0 = signo, x1 = siginfo*, x2 = ucontext*), the interrupted
//     context is saved, and on the handler's `ret` (x30 == SIGRET sentinel) or
//     an explicit `rt_sigreturn` (139) the context is restored and the
//     interrupted guest resumes right after the original `svc`.
//   - Un-handled signals fall back to the POSIX default disposition (ignore /
//     terminate the process), so a guest that sends itself SIGTERM or writes to
//     a closed pipe behaves like a real Linux process instead of carrying on.

use crate::jit::CpuState;

/// Magic guest pointer a dispatched signal handler's `ret` jumps to (`x30`).
/// The dispatcher loop recognizes this pc and restores the saved context. It is
/// a deliberately invalid instruction address (nowhere near guest code or the
/// host-thunk region), so it can never be a real branch target.
pub const SIGRET: u64 = 0x5151_5253_5455_5657; // "SIGRET"

/// A guest-visible per-signal action, in the kernel's aarch64 `struct sigaction`
/// semantics: `handler` is 0 = SIG_DFL, 1 = SIG_IGN, else a guest function addr.
#[derive(Clone, Copy)]
pub struct GuestSigAction {
    pub handler: u64,
    pub flags: u64,
    pub mask: [u8; 8],
}

/// Process-wide signal action table, indexed by signal number (1..=64).
static SIG_ACTIONS: std::sync::Mutex<[GuestSigAction; 65]> =
    std::sync::Mutex::new([GuestSigAction { handler: 0, flags: 0, mask: [0; 8] }; 65]);

/// Saved interrupted guest context for an in-flight signal handler, per guest
/// thread (a process-local stack so nested signals unwind LIFO). The guest
/// siginfo/ucontext bytes live in the frame itself: since guest == host
/// addresses here, the handler's x1/x2 args can point straight at these
/// buffers, which stay valid until `sigreturn` pops the frame.
struct SigFrame {
    x: [u64; 32],
    v: [u64; 64],
    sp: u64,
    tpidr: u64,
    nzcv: u32,
    /// Guest address to resume at after the handler returns: the instruction
    /// right after the interrupted `svc` (the post-syscall continuation).
    pc: u64,
    si: [u8; 128],   // guest siginfo buffer (handler x1)
    uc: [u8; 1024],  // guest ucontext buffer (handler x2)
}

thread_local! {
    static FRAMES: std::cell::RefCell<Vec<Box<SigFrame>>> = std::cell::RefCell::new(Vec::new());
}

/// Test/app hook: clear the process-wide action table. silences nothing — just
/// returns every signal to its default disposition.
pub fn reset_actions() {
    *SIG_ACTIONS.lock().unwrap() =
        [GuestSigAction { handler: 0, flags: 0, mask: [0; 8] }; 65];
}

/// POSIX default disposition for an un-handled signal. Returns `true` when the
/// default action terminates the process (everything not ignore/stop/continue).
/// There is no job control in the single-process runtime, so SIGTSTP/SIGTTIN/
/// SIGTTOU/SIGSTOP/SIGCONT collapse to "ignore", as do SIGCHLD/SIGURG/SIGWINCH.
fn default_terminates(sig: u64) -> bool {
    !matches!(sig, 17 | 18 | 19 | 20 | 21 | 22 | 23 | 28)
}

/// Install or query a signal action (`rt_sigaction`, syscall 134).
///
/// Guest aarch64 kernel `struct sigaction` (all sizes in bytes):
///   sa_handler : u64 @ 0        (0 = SIG_DFL, 1 = SIG_IGN, else fn addr)
///   sa_flags   : u64 @ 8
///   sa_restorer: u64 @ 16
///   sa_mask    : 8  bytes @ 24  (kernel sigset_t, `sigsetsize` = 8)
pub fn rt_sigaction(sig: u64, act: u64, oact: u64) -> i64 {
    if sig < 1 || sig > 64 {
        return (-libc::EINVAL) as i64;
    }
    let mut table = SIG_ACTIONS.lock().unwrap();
    if oact != 0 {
        let cur = table[sig as usize];
        // SAFETY: the guest passed a writable oact of at least the 32-byte
        // (max(24+8)) aarch64 sigaction struct.
        unsafe {
            let p = oact as *mut u8;
            (p as *mut u64).write_unaligned(cur.handler);
            (p.add(8) as *mut u64).write_unaligned(cur.flags);
            (p.add(16) as *mut u64).write_unaligned(0); // we install no restorer
            std::ptr::copy_nonoverlapping(&cur.mask as *const u8, p.add(24), 8);
        }
    }
    if act != 0 {
        // SAFETY: the guest passed a readable act of >= 32 bytes.
        unsafe {
            let p = act as *const u8;
            let handler = (p as *const u64).read_unaligned();
            let flags = (p.add(8) as *const u64).read_unaligned();
            let mut mask = [0u8; 8];
            std::ptr::copy_nonoverlapping(p.add(24), &mut mask as *mut u8, 8);
            table[sig as usize] = GuestSigAction { handler, flags, mask };
        }
    }
    drop(table);
    0
}

/// Apply the effective action for `sig` to the CURRENT guest thread (`st`):
///   - SIG_IGN / default-ignore -> nothing (signal consumed).
///   - default-terminate        -> process terminates (128 + sig), like Linux.
///   - installed guest handler   -> save context, prepare the handler (set
///                                  `st.redirect_request = handler` so the
///                                  dispatcher loop runs it next, and set
///                                  `x30 = SIGRET` so the handler's `ret` hands
///                                  back to the dispatcher's sigreturn), then
///                                  restore the context and resume at `resume`.
/// `resume` is the guest PC of the interrupted context: for a signal taken
/// during a self-delivering syscall this is `st.svc_next` (the instruction after
/// the `svc`); for a cross-thread (cooperative-pending) pickup it is the
/// thread's current `st.pc`. Runs on the owning thread of `st`.
pub fn dispatch_current_thread(st: &mut CpuState, sig: u32, resume: u64) {
    let act = SIG_ACTIONS.lock().unwrap()[sig as usize];
    if act.handler == 1 {
        return; // SIG_IGN
    }
    if act.handler == 0 {
        // Default disposition.
        if default_terminates(sig as u64) {
            // SAFETY: a SIG_DFL-terminating signal ends the whole process, as
            // on Linux (no guest settable handlers get to run for it).
            unsafe {
                libc::_exit(128 + sig as libc::c_int);
            }
        }
        return; // default-ignore
    }
    begin_handler(st, sig, act.handler, resume);
}

/// Save `st`'s context and enter a guest signal handler (`handler`) with the
/// aarch64 signal ABI. On the handler's `ret` (x30 = SIGRET) the dispatcher
/// calls `sigreturn` to restore. `resume` is the interrupted guest PC to resume
/// at after the handler returns.
fn begin_handler(st: &mut CpuState, sig: u32, handler: u64, resume: u64) {
    let mut f = Box::new(SigFrame {
        x: st.x,
        v: st.v,
        sp: st.x[31],
        tpidr: st.tpidr,
        nzcv: st.nzcv,
        pc: resume,
        si: [0u8; 128],
        uc: [0u8; 1024],
    });
    // The interrupted syscall (kill/tgkill) returns 0 to the guest after the
    // handler completes, matching Linux. The kernel would leave the syscall
    // return in x0; we mirror that by overriding the saved x0.
    f.x[0] = 0;
    // siginfo_t header: si_signo, si_errno=0, si_code=SI_USER(0).
    f.si[0..4].copy_from_slice(&(sig as i32).to_le_bytes());
    f.si[8..12].copy_from_slice(&0i32.to_le_bytes());
    let si_addr = f.si.as_ptr() as u64;
    let uc_addr = f.uc.as_ptr() as u64;
    FRAMES.with(|fr| fr.borrow_mut().push(f));
    // Configure the register file to run the handler per the aarch64 signal
    // convention: x0=signo, x1=siginfo*, x2=ucontext*, x30=SIGRET (so the
    // handler's `ret` hands back to the dispatcher's sigreturn). sp stays the
    // interrupted sp; the handler pushes its own frames beneath it. The
    // redirect_request tells the Svc arm (self-delivery) / dispatcher loop
    // (pending pickup) to run the handler instead of the interrupted code.
    st.redirect_request = handler;
    st.x[0] = sig as u64;
    st.x[1] = si_addr;
    st.x[2] = uc_addr;
    st.x[30] = SIGRET;
}

/// Restore the interrupted context (`rt_sigreturn`, syscall 139, or the SIGRET
/// handler-`ret` path). Returns false if there was no saved frame for this
/// thread — a stray sigreturn, which the guest should never produce.
pub fn sigreturn(st: &mut CpuState) -> bool {
    let popped = FRAMES.with(|fr| fr.borrow_mut().pop());
    match popped {
        Some(f) => {
            st.x = f.x;
            st.v = f.v;
            st.x[31] = f.sp;
            st.tpidr = f.tpidr;
            st.nzcv = f.nzcv;
            st.pc = f.pc;
            true
        }
        None => false,
    }
}