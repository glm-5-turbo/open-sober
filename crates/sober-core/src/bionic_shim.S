/* Auto-generated Bionic symbol shim with lazy dispatch. DO NOT EDIT.
 * 392 trampolines, 13 data objs.
 */

.section .data.bionic_tramp, "aw"
.balign 8
.globl __bf_tramp_table
.type __bf_tramp_table, %object
.size __bf_tramp_table, 3136
__bf_tramp_table:
.zero 3136

.section .text.bionic_resolver, "ax"
.balign 4
.globl __bf_resolve_and_call
.type __bf_resolve_and_call, %function
__bf_resolve_and_call:
    /* x0 = trampoline index, x1 = address of glibc func */
    /* This is called from each lazy trampoline for first-use resolution */
    /* We use a simple C function pointer table populated via dlsym */
    /* Arguments: x0 = entry index */
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    /* We call __bf_c_resolve(index) which does dlsym and fills table */
    bl __bf_c_resolve
    /* Restore and jump to the resolved function */
    ldp x29, x30, [sp], #16
    /* x0 now holds the function pointer, jump to it */
    br x0

.section .text.bionic_tramp, "ax"

.balign 4
.globl __bf___cxa_finalize
.type __bf___cxa_finalize, %function
__bf___cxa_finalize:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #0]
    cbz x17, 1f
    br x17
1:  mov x0, #0
    bl __bf_resolve_and_call
    br x0
.symver __bf___cxa_finalize, __cxa_finalize@@LIBC

.balign 4
.globl __bf___cxa_atexit
.type __bf___cxa_atexit, %function
__bf___cxa_atexit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #8]
    cbz x17, 1f
    br x17
1:  mov x0, #1
    bl __bf_resolve_and_call
    br x0
.symver __bf___cxa_atexit, __cxa_atexit@@LIBC

.balign 4
.globl __bf___register_atfork
.type __bf___register_atfork, %function
__bf___register_atfork:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #16]
    cbz x17, 1f
    br x17
1:  mov x0, #2
    bl __bf_resolve_and_call
    br x0
.symver __bf___register_atfork, __register_atfork@@LIBC

.balign 4
.globl __bf_strlen
.type __bf_strlen, %function
__bf_strlen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #24]
    cbz x17, 1f
    br x17
1:  mov x0, #3
    bl __bf_resolve_and_call
    br x0
.symver __bf_strlen, strlen@@LIBC

.balign 4
.globl __bf_memcmp
.type __bf_memcmp, %function
__bf_memcmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #32]
    cbz x17, 1f
    br x17
1:  mov x0, #4
    bl __bf_resolve_and_call
    br x0
.symver __bf_memcmp, memcmp@@LIBC

.balign 4
.globl __bf_pthread_mutex_init
.type __bf_pthread_mutex_init, %function
__bf_pthread_mutex_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #40]
    cbz x17, 1f
    br x17
1:  mov x0, #5
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_init, pthread_mutex_init@@LIBC

.balign 4
.globl __bf_pthread_mutex_destroy
.type __bf_pthread_mutex_destroy, %function
__bf_pthread_mutex_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #48]
    cbz x17, 1f
    br x17
1:  mov x0, #6
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_destroy, pthread_mutex_destroy@@LIBC

.balign 4
.globl __bf_pthread_once
.type __bf_pthread_once, %function
__bf_pthread_once:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #56]
    cbz x17, 1f
    br x17
1:  mov x0, #7
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_once, pthread_once@@LIBC

.balign 4
.globl __bf___memset_chk
.type __bf___memset_chk, %function
__bf___memset_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #64]
    cbz x17, 1f
    br x17
1:  mov x0, #8
    bl __bf_resolve_and_call
    br x0
.symver __bf___memset_chk, __memset_chk@@LIBC

.balign 4
.globl __bf___memcpy_chk
.type __bf___memcpy_chk, %function
__bf___memcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #72]
    cbz x17, 1f
    br x17
1:  mov x0, #9
    bl __bf_resolve_and_call
    br x0
.symver __bf___memcpy_chk, __memcpy_chk@@LIBC

.balign 4
.globl __bf___strlen_chk
.type __bf___strlen_chk, %function
__bf___strlen_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #80]
    cbz x17, 1f
    br x17
1:  mov x0, #10
    bl __bf_resolve_and_call
    br x0
.symver __bf___strlen_chk, __strlen_chk@@LIBC

.balign 4
.globl __bf_memchr
.type __bf_memchr, %function
__bf_memchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #88]
    cbz x17, 1f
    br x17
1:  mov x0, #11
    bl __bf_resolve_and_call
    br x0
.symver __bf_memchr, memchr@@LIBC

.balign 4
.globl __bf_dladdr
.type __bf_dladdr, %function
__bf_dladdr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #96]
    cbz x17, 1f
    br x17
1:  mov x0, #12
    bl __bf_resolve_and_call
    br x0
.symver __bf_dladdr, dladdr@@LIBC

.balign 4
.globl __bf_dlerror
.type __bf_dlerror, %function
__bf_dlerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #104]
    cbz x17, 1f
    br x17
1:  mov x0, #13
    bl __bf_resolve_and_call
    br x0
.symver __bf_dlerror, dlerror@@LIBC

.balign 4
.globl __bf_dlopen
.type __bf_dlopen, %function
__bf_dlopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #112]
    cbz x17, 1f
    br x17
1:  mov x0, #14
    bl __bf_resolve_and_call
    br x0
.symver __bf_dlopen, dlopen@@LIBC

.balign 4
.globl __bf_dlsym
.type __bf_dlsym, %function
__bf_dlsym:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #120]
    cbz x17, 1f
    br x17
1:  mov x0, #15
    bl __bf_resolve_and_call
    br x0
.symver __bf_dlsym, dlsym@@LIBC

.balign 4
.globl __bf_strncmp
.type __bf_strncmp, %function
__bf_strncmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #128]
    cbz x17, 1f
    br x17
1:  mov x0, #16
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncmp, strncmp@@LIBC

.balign 4
.globl __bf_strcmp
.type __bf_strcmp, %function
__bf_strcmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #136]
    cbz x17, 1f
    br x17
1:  mov x0, #17
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcmp, strcmp@@LIBC

.balign 4
.globl __bf_getauxval
.type __bf_getauxval, %function
__bf_getauxval:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #144]
    cbz x17, 1f
    br x17
1:  mov x0, #18
    bl __bf_resolve_and_call
    br x0
.symver __bf_getauxval, getauxval@@LIBC

.balign 4
.globl __bf___errno
.type __bf___errno, %function
__bf___errno:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #152]
    cbz x17, 1f
    br x17
1:  mov x0, #19
    bl __bf_resolve_and_call
    br x0
.symver __bf___errno, __errno@@LIBC

.balign 4
.globl __bf_close
.type __bf_close, %function
__bf_close:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #160]
    cbz x17, 1f
    br x17
1:  mov x0, #20
    bl __bf_resolve_and_call
    br x0
.symver __bf_close, close@@LIBC

.balign 4
.globl __bf___open_2
.type __bf___open_2, %function
__bf___open_2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #168]
    cbz x17, 1f
    br x17
1:  mov x0, #21
    bl __bf_resolve_and_call
    br x0
.symver __bf___open_2, __open_2@@LIBC

.balign 4
.globl __bf___read_chk
.type __bf___read_chk, %function
__bf___read_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #176]
    cbz x17, 1f
    br x17
1:  mov x0, #22
    bl __bf_resolve_and_call
    br x0
.symver __bf___read_chk, __read_chk@@LIBC

.balign 4
.globl __bf_read
.type __bf_read, %function
__bf_read:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #184]
    cbz x17, 1f
    br x17
1:  mov x0, #23
    bl __bf_resolve_and_call
    br x0
.symver __bf_read, read@@LIBC

.balign 4
.globl __bf_clock_gettime
.type __bf_clock_gettime, %function
__bf_clock_gettime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #192]
    cbz x17, 1f
    br x17
1:  mov x0, #24
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_gettime, clock_gettime@@LIBC

.balign 4
.globl __bf_syscall
.type __bf_syscall, %function
__bf_syscall:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #200]
    cbz x17, 1f
    br x17
1:  mov x0, #25
    bl __bf_resolve_and_call
    br x0
.symver __bf_syscall, syscall@@LIBC

.balign 4
.globl __bf_sched_getcpu
.type __bf_sched_getcpu, %function
__bf_sched_getcpu:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #208]
    cbz x17, 1f
    br x17
1:  mov x0, #26
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getcpu, sched_getcpu@@LIBC

.balign 4
.globl __bf_sysconf
.type __bf_sysconf, %function
__bf_sysconf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #216]
    cbz x17, 1f
    br x17
1:  mov x0, #27
    bl __bf_resolve_and_call
    br x0
.symver __bf_sysconf, sysconf@@LIBC

.balign 4
.globl __bf_mmap
.type __bf_mmap, %function
__bf_mmap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #224]
    cbz x17, 1f
    br x17
1:  mov x0, #28
    bl __bf_resolve_and_call
    br x0
.symver __bf_mmap, mmap@@LIBC

.balign 4
.globl __bf_mprotect
.type __bf_mprotect, %function
__bf_mprotect:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #232]
    cbz x17, 1f
    br x17
1:  mov x0, #29
    bl __bf_resolve_and_call
    br x0
.symver __bf_mprotect, mprotect@@LIBC

.balign 4
.globl __bf_munmap
.type __bf_munmap, %function
__bf_munmap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #240]
    cbz x17, 1f
    br x17
1:  mov x0, #30
    bl __bf_resolve_and_call
    br x0
.symver __bf_munmap, munmap@@LIBC

.balign 4
.globl __bf_pthread_attr_init
.type __bf_pthread_attr_init, %function
__bf_pthread_attr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #248]
    cbz x17, 1f
    br x17
1:  mov x0, #31
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_init, pthread_attr_init@@LIBC

.balign 4
.globl __bf_pthread_attr_setstacksize
.type __bf_pthread_attr_setstacksize, %function
__bf_pthread_attr_setstacksize:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #256]
    cbz x17, 1f
    br x17
1:  mov x0, #32
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setstacksize, pthread_attr_setstacksize@@LIBC

.balign 4
.globl __bf_pthread_create
.type __bf_pthread_create, %function
__bf_pthread_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #264]
    cbz x17, 1f
    br x17
1:  mov x0, #33
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_create, pthread_create@@LIBC

.balign 4
.globl __bf_pthread_attr_destroy
.type __bf_pthread_attr_destroy, %function
__bf_pthread_attr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #272]
    cbz x17, 1f
    br x17
1:  mov x0, #34
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_destroy, pthread_attr_destroy@@LIBC

.balign 4
.globl __bf_pthread_join
.type __bf_pthread_join, %function
__bf_pthread_join:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #280]
    cbz x17, 1f
    br x17
1:  mov x0, #35
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_join, pthread_join@@LIBC

.balign 4
.globl __bf_pthread_self
.type __bf_pthread_self, %function
__bf_pthread_self:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #288]
    cbz x17, 1f
    br x17
1:  mov x0, #36
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_self, pthread_self@@LIBC

.balign 4
.globl __bf_memset
.type __bf_memset, %function
__bf_memset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #296]
    cbz x17, 1f
    br x17
1:  mov x0, #37
    bl __bf_resolve_and_call
    br x0
.symver __bf_memset, memset@@LIBC

.balign 4
.globl __bf_pthread_setspecific
.type __bf_pthread_setspecific, %function
__bf_pthread_setspecific:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #304]
    cbz x17, 1f
    br x17
1:  mov x0, #38
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setspecific, pthread_setspecific@@LIBC

.balign 4
.globl __bf___strncpy_chk
.type __bf___strncpy_chk, %function
__bf___strncpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #312]
    cbz x17, 1f
    br x17
1:  mov x0, #39
    bl __bf_resolve_and_call
    br x0
.symver __bf___strncpy_chk, __strncpy_chk@@LIBC

.balign 4
.globl __bf_sched_get_priority_max
.type __bf_sched_get_priority_max, %function
__bf_sched_get_priority_max:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #320]
    cbz x17, 1f
    br x17
1:  mov x0, #40
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_get_priority_max, sched_get_priority_max@@LIBC

.balign 4
.globl __bf_sched_setscheduler
.type __bf_sched_setscheduler, %function
__bf_sched_setscheduler:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #328]
    cbz x17, 1f
    br x17
1:  mov x0, #41
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_setscheduler, sched_setscheduler@@LIBC

.balign 4
.globl __bf_pthread_mutex_lock
.type __bf_pthread_mutex_lock, %function
__bf_pthread_mutex_lock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #336]
    cbz x17, 1f
    br x17
1:  mov x0, #42
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_lock, pthread_mutex_lock@@LIBC

.balign 4
.globl __bf_pthread_cond_wait
.type __bf_pthread_cond_wait, %function
__bf_pthread_cond_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #344]
    cbz x17, 1f
    br x17
1:  mov x0, #43
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_wait, pthread_cond_wait@@LIBC

.balign 4
.globl __bf_pthread_mutex_unlock
.type __bf_pthread_mutex_unlock, %function
__bf_pthread_mutex_unlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #352]
    cbz x17, 1f
    br x17
1:  mov x0, #44
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_unlock, pthread_mutex_unlock@@LIBC

.balign 4
.globl __bf_pthread_cond_signal
.type __bf_pthread_cond_signal, %function
__bf_pthread_cond_signal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #360]
    cbz x17, 1f
    br x17
1:  mov x0, #45
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_signal, pthread_cond_signal@@LIBC

.balign 4
.globl __bf_pthread_cond_init
.type __bf_pthread_cond_init, %function
__bf_pthread_cond_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #368]
    cbz x17, 1f
    br x17
1:  mov x0, #46
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_init, pthread_cond_init@@LIBC

.balign 4
.globl __bf_gettid
.type __bf_gettid, %function
__bf_gettid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #376]
    cbz x17, 1f
    br x17
1:  mov x0, #47
    bl __bf_resolve_and_call
    br x0
.symver __bf_gettid, gettid@@LIBC

.balign 4
.globl __bf___vsprintf_chk
.type __bf___vsprintf_chk, %function
__bf___vsprintf_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #384]
    cbz x17, 1f
    br x17
1:  mov x0, #48
    bl __bf_resolve_and_call
    br x0
.symver __bf___vsprintf_chk, __vsprintf_chk@@LIBC

.balign 4
.globl __bf_atan2f
.type __bf_atan2f, %function
__bf_atan2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #392]
    cbz x17, 1f
    br x17
1:  mov x0, #49
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan2f, atan2f@@LIBC

.balign 4
.globl __bf_vsnprintf
.type __bf_vsnprintf, %function
__bf_vsnprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #400]
    cbz x17, 1f
    br x17
1:  mov x0, #50
    bl __bf_resolve_and_call
    br x0
.symver __bf_vsnprintf, vsnprintf@@LIBC

.balign 4
.globl __bf_fprintf
.type __bf_fprintf, %function
__bf_fprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #408]
    cbz x17, 1f
    br x17
1:  mov x0, #51
    bl __bf_resolve_and_call
    br x0
.symver __bf_fprintf, fprintf@@LIBC

.balign 4
.globl __bf_rand
.type __bf_rand, %function
__bf_rand:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #416]
    cbz x17, 1f
    br x17
1:  mov x0, #52
    bl __bf_resolve_and_call
    br x0
.symver __bf_rand, rand@@LIBC

.balign 4
.globl __bf_strtoll
.type __bf_strtoll, %function
__bf_strtoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #424]
    cbz x17, 1f
    br x17
1:  mov x0, #53
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoll, strtoll@@LIBC

.balign 4
.globl __bf_time
.type __bf_time, %function
__bf_time:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #432]
    cbz x17, 1f
    br x17
1:  mov x0, #54
    bl __bf_resolve_and_call
    br x0
.symver __bf_time, time@@LIBC

.balign 4
.globl __bf_pthread_cond_broadcast
.type __bf_pthread_cond_broadcast, %function
__bf_pthread_cond_broadcast:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #440]
    cbz x17, 1f
    br x17
1:  mov x0, #55
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_broadcast, pthread_cond_broadcast@@LIBC

.balign 4
.globl __bf_pthread_cond_destroy
.type __bf_pthread_cond_destroy, %function
__bf_pthread_cond_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #448]
    cbz x17, 1f
    br x17
1:  mov x0, #56
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_destroy, pthread_cond_destroy@@LIBC

.balign 4
.globl __bf_stat
.type __bf_stat, %function
__bf_stat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #456]
    cbz x17, 1f
    br x17
1:  mov x0, #57
    bl __bf_resolve_and_call
    br x0
.symver __bf_stat, stat@@LIBC

.balign 4
.globl __bf_opendir
.type __bf_opendir, %function
__bf_opendir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #464]
    cbz x17, 1f
    br x17
1:  mov x0, #58
    bl __bf_resolve_and_call
    br x0
.symver __bf_opendir, opendir@@LIBC

.balign 4
.globl __bf_readdir
.type __bf_readdir, %function
__bf_readdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #472]
    cbz x17, 1f
    br x17
1:  mov x0, #59
    bl __bf_resolve_and_call
    br x0
.symver __bf_readdir, readdir@@LIBC

.balign 4
.globl __bf_closedir
.type __bf_closedir, %function
__bf_closedir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #480]
    cbz x17, 1f
    br x17
1:  mov x0, #60
    bl __bf_resolve_and_call
    br x0
.symver __bf_closedir, closedir@@LIBC

.balign 4
.globl __bf_posix_fallocate
.type __bf_posix_fallocate, %function
__bf_posix_fallocate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #488]
    cbz x17, 1f
    br x17
1:  mov x0, #61
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_fallocate, posix_fallocate@@LIBC

.balign 4
.globl __bf_open
.type __bf_open, %function
__bf_open:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #496]
    cbz x17, 1f
    br x17
1:  mov x0, #62
    bl __bf_resolve_and_call
    br x0
.symver __bf_open, open@@LIBC

.balign 4
.globl __bf_pthread_equal
.type __bf_pthread_equal, %function
__bf_pthread_equal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #504]
    cbz x17, 1f
    br x17
1:  mov x0, #63
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_equal, pthread_equal@@LIBC

.balign 4
.globl __bf_atoll
.type __bf_atoll, %function
__bf_atoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #512]
    cbz x17, 1f
    br x17
1:  mov x0, #64
    bl __bf_resolve_and_call
    br x0
.symver __bf_atoll, atoll@@LIBC

.balign 4
.globl __bf_srand
.type __bf_srand, %function
__bf_srand:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #520]
    cbz x17, 1f
    br x17
1:  mov x0, #65
    bl __bf_resolve_and_call
    br x0
.symver __bf_srand, srand@@LIBC

.balign 4
.globl __bf_strtod
.type __bf_strtod, %function
__bf_strtod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #528]
    cbz x17, 1f
    br x17
1:  mov x0, #66
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtod, strtod@@LIBC

.balign 4
.globl __bf_localtime
.type __bf_localtime, %function
__bf_localtime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #536]
    cbz x17, 1f
    br x17
1:  mov x0, #67
    bl __bf_resolve_and_call
    br x0
.symver __bf_localtime, localtime@@LIBC

.balign 4
.globl __bf_fseek
.type __bf_fseek, %function
__bf_fseek:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #544]
    cbz x17, 1f
    br x17
1:  mov x0, #68
    bl __bf_resolve_and_call
    br x0
.symver __bf_fseek, fseek@@LIBC

.balign 4
.globl __bf_ftell
.type __bf_ftell, %function
__bf_ftell:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #552]
    cbz x17, 1f
    br x17
1:  mov x0, #69
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftell, ftell@@LIBC

.balign 4
.globl __bf_fclose
.type __bf_fclose, %function
__bf_fclose:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #560]
    cbz x17, 1f
    br x17
1:  mov x0, #70
    bl __bf_resolve_and_call
    br x0
.symver __bf_fclose, fclose@@LIBC

.balign 4
.globl __bf_atan
.type __bf_atan, %function
__bf_atan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #568]
    cbz x17, 1f
    br x17
1:  mov x0, #71
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan, atan@@LIBC

.balign 4
.globl __bf_mkdir
.type __bf_mkdir, %function
__bf_mkdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #576]
    cbz x17, 1f
    br x17
1:  mov x0, #72
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkdir, mkdir@@LIBC

.balign 4
.globl __bf___fread_chk
.type __bf___fread_chk, %function
__bf___fread_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #584]
    cbz x17, 1f
    br x17
1:  mov x0, #73
    bl __bf_resolve_and_call
    br x0
.symver __bf___fread_chk, __fread_chk@@LIBC_N

.balign 4
.globl __bf_fread
.type __bf_fread, %function
__bf_fread:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #592]
    cbz x17, 1f
    br x17
1:  mov x0, #74
    bl __bf_resolve_and_call
    br x0
.symver __bf_fread, fread@@LIBC

.balign 4
.globl __bf_asinf
.type __bf_asinf, %function
__bf_asinf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #600]
    cbz x17, 1f
    br x17
1:  mov x0, #75
    bl __bf_resolve_and_call
    br x0
.symver __bf_asinf, asinf@@LIBC

.balign 4
.globl __bf_dlclose
.type __bf_dlclose, %function
__bf_dlclose:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #608]
    cbz x17, 1f
    br x17
1:  mov x0, #76
    bl __bf_resolve_and_call
    br x0
.symver __bf_dlclose, dlclose@@LIBC

.balign 4
.globl __bf_access
.type __bf_access, %function
__bf_access:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #616]
    cbz x17, 1f
    br x17
1:  mov x0, #77
    bl __bf_resolve_and_call
    br x0
.symver __bf_access, access@@LIBC

.balign 4
.globl __bf_getenv
.type __bf_getenv, %function
__bf_getenv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #624]
    cbz x17, 1f
    br x17
1:  mov x0, #78
    bl __bf_resolve_and_call
    br x0
.symver __bf_getenv, getenv@@LIBC

.balign 4
.globl __bf_strncpy
.type __bf_strncpy, %function
__bf_strncpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #632]
    cbz x17, 1f
    br x17
1:  mov x0, #79
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncpy, strncpy@@LIBC

.balign 4
.globl __bf_feof
.type __bf_feof, %function
__bf_feof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #640]
    cbz x17, 1f
    br x17
1:  mov x0, #80
    bl __bf_resolve_and_call
    br x0
.symver __bf_feof, feof@@LIBC

.balign 4
.globl __bf_fopen
.type __bf_fopen, %function
__bf_fopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #648]
    cbz x17, 1f
    br x17
1:  mov x0, #81
    bl __bf_resolve_and_call
    br x0
.symver __bf_fopen, fopen@@LIBC

.balign 4
.globl __bf_fseeko
.type __bf_fseeko, %function
__bf_fseeko:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #656]
    cbz x17, 1f
    br x17
1:  mov x0, #82
    bl __bf_resolve_and_call
    br x0
.symver __bf_fseeko, fseeko@@LIBC

.balign 4
.globl __bf_ftello
.type __bf_ftello, %function
__bf_ftello:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #664]
    cbz x17, 1f
    br x17
1:  mov x0, #83
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftello, ftello@@LIBC

.balign 4
.globl __bf_fwrite
.type __bf_fwrite, %function
__bf_fwrite:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #672]
    cbz x17, 1f
    br x17
1:  mov x0, #84
    bl __bf_resolve_and_call
    br x0
.symver __bf_fwrite, fwrite@@LIBC

.balign 4
.globl __bf_fflush
.type __bf_fflush, %function
__bf_fflush:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #680]
    cbz x17, 1f
    br x17
1:  mov x0, #85
    bl __bf_resolve_and_call
    br x0
.symver __bf_fflush, fflush@@LIBC

.balign 4
.globl __bf_gmtime
.type __bf_gmtime, %function
__bf_gmtime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #688]
    cbz x17, 1f
    br x17
1:  mov x0, #86
    bl __bf_resolve_and_call
    br x0
.symver __bf_gmtime, gmtime@@LIBC

.balign 4
.globl __bf_mktime
.type __bf_mktime, %function
__bf_mktime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #696]
    cbz x17, 1f
    br x17
1:  mov x0, #87
    bl __bf_resolve_and_call
    br x0
.symver __bf_mktime, mktime@@LIBC

.balign 4
.globl __bf_pipe
.type __bf_pipe, %function
__bf_pipe:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #704]
    cbz x17, 1f
    br x17
1:  mov x0, #88
    bl __bf_resolve_and_call
    br x0
.symver __bf_pipe, pipe@@LIBC

.balign 4
.globl __bf_write
.type __bf_write, %function
__bf_write:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #712]
    cbz x17, 1f
    br x17
1:  mov x0, #89
    bl __bf_resolve_and_call
    br x0
.symver __bf_write, write@@LIBC

.balign 4
.globl __bf_pthread_key_create
.type __bf_pthread_key_create, %function
__bf_pthread_key_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #720]
    cbz x17, 1f
    br x17
1:  mov x0, #90
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_key_create, pthread_key_create@@LIBC

.balign 4
.globl __bf_abort
.type __bf_abort, %function
__bf_abort:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #728]
    cbz x17, 1f
    br x17
1:  mov x0, #91
    bl __bf_resolve_and_call
    br x0
.symver __bf_abort, abort@@LIBC

.balign 4
.globl __bf___assert
.type __bf___assert, %function
__bf___assert:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #736]
    cbz x17, 1f
    br x17
1:  mov x0, #92
    bl __bf_resolve_and_call
    br x0
.symver __bf___assert, __assert@@LIBC

.balign 4
.globl __bf_strrchr
.type __bf_strrchr, %function
__bf_strrchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #744]
    cbz x17, 1f
    br x17
1:  mov x0, #93
    bl __bf_resolve_and_call
    br x0
.symver __bf_strrchr, strrchr@@LIBC

.balign 4
.globl __bf___stack_chk_fail
.type __bf___stack_chk_fail, %function
__bf___stack_chk_fail:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #752]
    cbz x17, 1f
    br x17
1:  mov x0, #94
    bl __bf_resolve_and_call
    br x0
.symver __bf___stack_chk_fail, __stack_chk_fail@@LIBC

.balign 4
.globl __bf_atoi
.type __bf_atoi, %function
__bf_atoi:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #760]
    cbz x17, 1f
    br x17
1:  mov x0, #95
    bl __bf_resolve_and_call
    br x0
.symver __bf_atoi, atoi@@LIBC

.balign 4
.globl __bf_fcntl
.type __bf_fcntl, %function
__bf_fcntl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #768]
    cbz x17, 1f
    br x17
1:  mov x0, #96
    bl __bf_resolve_and_call
    br x0
.symver __bf_fcntl, fcntl@@LIBC

.balign 4
.globl __bf_memcpy
.type __bf_memcpy, %function
__bf_memcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #776]
    cbz x17, 1f
    br x17
1:  mov x0, #97
    bl __bf_resolve_and_call
    br x0
.symver __bf_memcpy, memcpy@@LIBC

.balign 4
.globl __bf_strcpy
.type __bf_strcpy, %function
__bf_strcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #784]
    cbz x17, 1f
    br x17
1:  mov x0, #98
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcpy, strcpy@@LIBC

.balign 4
.globl __bf_strerror
.type __bf_strerror, %function
__bf_strerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #792]
    cbz x17, 1f
    br x17
1:  mov x0, #99
    bl __bf_resolve_and_call
    br x0
.symver __bf_strerror, strerror@@LIBC

.balign 4
.globl __bf_pthread_attr_setdetachstate
.type __bf_pthread_attr_setdetachstate, %function
__bf_pthread_attr_setdetachstate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #800]
    cbz x17, 1f
    br x17
1:  mov x0, #100
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setdetachstate, pthread_attr_setdetachstate@@LIBC

.balign 4
.globl __bf_pthread_cond_timedwait
.type __bf_pthread_cond_timedwait, %function
__bf_pthread_cond_timedwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #808]
    cbz x17, 1f
    br x17
1:  mov x0, #101
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_timedwait, pthread_cond_timedwait@@LIBC

.balign 4
.globl __bf_memmove
.type __bf_memmove, %function
__bf_memmove:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #816]
    cbz x17, 1f
    br x17
1:  mov x0, #102
    bl __bf_resolve_and_call
    br x0
.symver __bf_memmove, memmove@@LIBC

.balign 4
.globl __bf_strtol
.type __bf_strtol, %function
__bf_strtol:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #824]
    cbz x17, 1f
    br x17
1:  mov x0, #103
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtol, strtol@@LIBC

.balign 4
.globl __bf_getpid
.type __bf_getpid, %function
__bf_getpid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #832]
    cbz x17, 1f
    br x17
1:  mov x0, #104
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpid, getpid@@LIBC

.balign 4
.globl __bf_gettimeofday
.type __bf_gettimeofday, %function
__bf_gettimeofday:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #840]
    cbz x17, 1f
    br x17
1:  mov x0, #105
    bl __bf_resolve_and_call
    br x0
.symver __bf_gettimeofday, gettimeofday@@LIBC

.balign 4
.globl __bf_localtime_r
.type __bf_localtime_r, %function
__bf_localtime_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #848]
    cbz x17, 1f
    br x17
1:  mov x0, #106
    bl __bf_resolve_and_call
    br x0
.symver __bf_localtime_r, localtime_r@@LIBC

.balign 4
.globl __bf_fputs
.type __bf_fputs, %function
__bf_fputs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #856]
    cbz x17, 1f
    br x17
1:  mov x0, #107
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputs, fputs@@LIBC

.balign 4
.globl __bf_strerror_r
.type __bf_strerror_r, %function
__bf_strerror_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #864]
    cbz x17, 1f
    br x17
1:  mov x0, #108
    bl __bf_resolve_and_call
    br x0
.symver __bf_strerror_r, strerror_r@@LIBC

.balign 4
.globl __bf_snprintf
.type __bf_snprintf, %function
__bf_snprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #872]
    cbz x17, 1f
    br x17
1:  mov x0, #109
    bl __bf_resolve_and_call
    br x0
.symver __bf_snprintf, snprintf@@LIBC

.balign 4
.globl __bf_prctl
.type __bf_prctl, %function
__bf_prctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #880]
    cbz x17, 1f
    br x17
1:  mov x0, #110
    bl __bf_resolve_and_call
    br x0
.symver __bf_prctl, prctl@@LIBC

.balign 4
.globl __bf_sigaltstack
.type __bf_sigaltstack, %function
__bf_sigaltstack:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #888]
    cbz x17, 1f
    br x17
1:  mov x0, #111
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaltstack, sigaltstack@@LIBC

.balign 4
.globl __bf_getpagesize
.type __bf_getpagesize, %function
__bf_getpagesize:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #896]
    cbz x17, 1f
    br x17
1:  mov x0, #112
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpagesize, getpagesize@@LIBC

.balign 4
.globl __bf_pthread_getspecific
.type __bf_pthread_getspecific, %function
__bf_pthread_getspecific:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #904]
    cbz x17, 1f
    br x17
1:  mov x0, #113
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getspecific, pthread_getspecific@@LIBC

.balign 4
.globl __bf_fork
.type __bf_fork, %function
__bf_fork:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #912]
    cbz x17, 1f
    br x17
1:  mov x0, #114
    bl __bf_resolve_and_call
    br x0
.symver __bf_fork, fork@@LIBC

.balign 4
.globl __bf_waitpid
.type __bf_waitpid, %function
__bf_waitpid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #920]
    cbz x17, 1f
    br x17
1:  mov x0, #115
    bl __bf_resolve_and_call
    br x0
.symver __bf_waitpid, waitpid@@LIBC

.balign 4
.globl __bf_execv
.type __bf_execv, %function
__bf_execv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #928]
    cbz x17, 1f
    br x17
1:  mov x0, #116
    bl __bf_resolve_and_call
    br x0
.symver __bf_execv, execv@@LIBC

.balign 4
.globl __bf__exit
.type __bf__exit, %function
__bf__exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #936]
    cbz x17, 1f
    br x17
1:  mov x0, #117
    bl __bf_resolve_and_call
    br x0
.symver __bf__exit, _exit@@LIBC

.balign 4
.globl __bf_execve
.type __bf_execve, %function
__bf_execve:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #944]
    cbz x17, 1f
    br x17
1:  mov x0, #118
    bl __bf_resolve_and_call
    br x0
.symver __bf_execve, execve@@LIBC

.balign 4
.globl __bf_getopt_long
.type __bf_getopt_long, %function
__bf_getopt_long:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #952]
    cbz x17, 1f
    br x17
1:  mov x0, #119
    bl __bf_resolve_and_call
    br x0
.symver __bf_getopt_long, getopt_long@@LIBC

.balign 4
.globl __bf_getppid
.type __bf_getppid, %function
__bf_getppid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #960]
    cbz x17, 1f
    br x17
1:  mov x0, #120
    bl __bf_resolve_and_call
    br x0
.symver __bf_getppid, getppid@@LIBC

.balign 4
.globl __bf_geteuid
.type __bf_geteuid, %function
__bf_geteuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #968]
    cbz x17, 1f
    br x17
1:  mov x0, #121
    bl __bf_resolve_and_call
    br x0
.symver __bf_geteuid, geteuid@@LIBC

.balign 4
.globl __bf_epoll_create1
.type __bf_epoll_create1, %function
__bf_epoll_create1:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #976]
    cbz x17, 1f
    br x17
1:  mov x0, #122
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_create1, epoll_create1@@LIBC

.balign 4
.globl __bf_eventfd
.type __bf_eventfd, %function
__bf_eventfd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #984]
    cbz x17, 1f
    br x17
1:  mov x0, #123
    bl __bf_resolve_and_call
    br x0
.symver __bf_eventfd, eventfd@@LIBC

.balign 4
.globl __bf_epoll_ctl
.type __bf_epoll_ctl, %function
__bf_epoll_ctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #992]
    cbz x17, 1f
    br x17
1:  mov x0, #124
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_ctl, epoll_ctl@@LIBC

.balign 4
.globl __bf_getsockopt
.type __bf_getsockopt, %function
__bf_getsockopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1000]
    cbz x17, 1f
    br x17
1:  mov x0, #125
    bl __bf_resolve_and_call
    br x0
.symver __bf_getsockopt, getsockopt@@LIBC

.balign 4
.globl __bf_setsockopt
.type __bf_setsockopt, %function
__bf_setsockopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1008]
    cbz x17, 1f
    br x17
1:  mov x0, #126
    bl __bf_resolve_and_call
    br x0
.symver __bf_setsockopt, setsockopt@@LIBC

.balign 4
.globl __bf_epoll_wait
.type __bf_epoll_wait, %function
__bf_epoll_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1016]
    cbz x17, 1f
    br x17
1:  mov x0, #127
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_wait, epoll_wait@@LIBC

.balign 4
.globl __bf_getuid
.type __bf_getuid, %function
__bf_getuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1024]
    cbz x17, 1f
    br x17
1:  mov x0, #128
    bl __bf_resolve_and_call
    br x0
.symver __bf_getuid, getuid@@LIBC

.balign 4
.globl __bf_pthread_mutex_trylock
.type __bf_pthread_mutex_trylock, %function
__bf_pthread_mutex_trylock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1032]
    cbz x17, 1f
    br x17
1:  mov x0, #129
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_trylock, pthread_mutex_trylock@@LIBC

.balign 4
.globl __bf_ldexp
.type __bf_ldexp, %function
__bf_ldexp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1040]
    cbz x17, 1f
    br x17
1:  mov x0, #130
    bl __bf_resolve_and_call
    br x0
.symver __bf_ldexp, ldexp@@LIBC

.balign 4
.globl __bf_strchr
.type __bf_strchr, %function
__bf_strchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1048]
    cbz x17, 1f
    br x17
1:  mov x0, #131
    bl __bf_resolve_and_call
    br x0
.symver __bf_strchr, strchr@@LIBC

.balign 4
.globl __bf_getaddrinfo
.type __bf_getaddrinfo, %function
__bf_getaddrinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1056]
    cbz x17, 1f
    br x17
1:  mov x0, #132
    bl __bf_resolve_and_call
    br x0
.symver __bf_getaddrinfo, getaddrinfo@@LIBC

.balign 4
.globl __bf_socket
.type __bf_socket, %function
__bf_socket:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1064]
    cbz x17, 1f
    br x17
1:  mov x0, #133
    bl __bf_resolve_and_call
    br x0
.symver __bf_socket, socket@@LIBC

.balign 4
.globl __bf_freeaddrinfo
.type __bf_freeaddrinfo, %function
__bf_freeaddrinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1072]
    cbz x17, 1f
    br x17
1:  mov x0, #134
    bl __bf_resolve_and_call
    br x0
.symver __bf_freeaddrinfo, freeaddrinfo@@LIBC

.balign 4
.globl __bf_connect
.type __bf_connect, %function
__bf_connect:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1080]
    cbz x17, 1f
    br x17
1:  mov x0, #135
    bl __bf_resolve_and_call
    br x0
.symver __bf_connect, connect@@LIBC

.balign 4
.globl __bf_poll
.type __bf_poll, %function
__bf_poll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1088]
    cbz x17, 1f
    br x17
1:  mov x0, #136
    bl __bf_resolve_and_call
    br x0
.symver __bf_poll, poll@@LIBC

.balign 4
.globl __bf_sched_getscheduler
.type __bf_sched_getscheduler, %function
__bf_sched_getscheduler:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1096]
    cbz x17, 1f
    br x17
1:  mov x0, #137
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getscheduler, sched_getscheduler@@LIBC

.balign 4
.globl __bf_sched_getparam
.type __bf_sched_getparam, %function
__bf_sched_getparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1104]
    cbz x17, 1f
    br x17
1:  mov x0, #138
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getparam, sched_getparam@@LIBC

.balign 4
.globl __bf_getpriority
.type __bf_getpriority, %function
__bf_getpriority:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1112]
    cbz x17, 1f
    br x17
1:  mov x0, #139
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpriority, getpriority@@LIBC

.balign 4
.globl __bf_uname
.type __bf_uname, %function
__bf_uname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1120]
    cbz x17, 1f
    br x17
1:  mov x0, #140
    bl __bf_resolve_and_call
    br x0
.symver __bf_uname, uname@@LIBC

.balign 4
.globl __bf_tzset
.type __bf_tzset, %function
__bf_tzset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1128]
    cbz x17, 1f
    br x17
1:  mov x0, #141
    bl __bf_resolve_and_call
    br x0
.symver __bf_tzset, tzset@@LIBC

.balign 4
.globl __bf_strnlen
.type __bf_strnlen, %function
__bf_strnlen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1136]
    cbz x17, 1f
    br x17
1:  mov x0, #142
    bl __bf_resolve_and_call
    br x0
.symver __bf_strnlen, strnlen@@LIBC

.balign 4
.globl __bf_writev
.type __bf_writev, %function
__bf_writev:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1144]
    cbz x17, 1f
    br x17
1:  mov x0, #143
    bl __bf_resolve_and_call
    br x0
.symver __bf_writev, writev@@LIBC

.balign 4
.globl __bf_sscanf
.type __bf_sscanf, %function
__bf_sscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1152]
    cbz x17, 1f
    br x17
1:  mov x0, #144
    bl __bf_resolve_and_call
    br x0
.symver __bf_sscanf, sscanf@@LIBC

.balign 4
.globl __bf_strtoul
.type __bf_strtoul, %function
__bf_strtoul:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1160]
    cbz x17, 1f
    br x17
1:  mov x0, #145
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoul, strtoul@@LIBC

.balign 4
.globl __bf_strtoull
.type __bf_strtoull, %function
__bf_strtoull:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1168]
    cbz x17, 1f
    br x17
1:  mov x0, #146
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoull, strtoull@@LIBC

.balign 4
.globl __bf_lseek
.type __bf_lseek, %function
__bf_lseek:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1176]
    cbz x17, 1f
    br x17
1:  mov x0, #147
    bl __bf_resolve_and_call
    br x0
.symver __bf_lseek, lseek@@LIBC

.balign 4
.globl __bf_ftruncate
.type __bf_ftruncate, %function
__bf_ftruncate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1184]
    cbz x17, 1f
    br x17
1:  mov x0, #148
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftruncate, ftruncate@@LIBC

.balign 4
.globl __bf_fstat
.type __bf_fstat, %function
__bf_fstat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1192]
    cbz x17, 1f
    br x17
1:  mov x0, #149
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstat, fstat@@LIBC

.balign 4
.globl __bf_lstat
.type __bf_lstat, %function
__bf_lstat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1200]
    cbz x17, 1f
    br x17
1:  mov x0, #150
    bl __bf_resolve_and_call
    br x0
.symver __bf_lstat, lstat@@LIBC

.balign 4
.globl __bf_rename
.type __bf_rename, %function
__bf_rename:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1208]
    cbz x17, 1f
    br x17
1:  mov x0, #151
    bl __bf_resolve_and_call
    br x0
.symver __bf_rename, rename@@LIBC

.balign 4
.globl __bf_unlink
.type __bf_unlink, %function
__bf_unlink:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1216]
    cbz x17, 1f
    br x17
1:  mov x0, #152
    bl __bf_resolve_and_call
    br x0
.symver __bf_unlink, unlink@@LIBC

.balign 4
.globl __bf_rmdir
.type __bf_rmdir, %function
__bf_rmdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1224]
    cbz x17, 1f
    br x17
1:  mov x0, #153
    bl __bf_resolve_and_call
    br x0
.symver __bf_rmdir, rmdir@@LIBC

.balign 4
.globl __bf_nanosleep
.type __bf_nanosleep, %function
__bf_nanosleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1232]
    cbz x17, 1f
    br x17
1:  mov x0, #154
    bl __bf_resolve_and_call
    br x0
.symver __bf_nanosleep, nanosleep@@LIBC

.balign 4
.globl __bf_sigemptyset
.type __bf_sigemptyset, %function
__bf_sigemptyset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1240]
    cbz x17, 1f
    br x17
1:  mov x0, #155
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigemptyset, sigemptyset@@LIBC

.balign 4
.globl __bf_sigaction
.type __bf_sigaction, %function
__bf_sigaction:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1248]
    cbz x17, 1f
    br x17
1:  mov x0, #156
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaction, sigaction@@LIBC

.balign 4
.globl __bf_raise
.type __bf_raise, %function
__bf_raise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1256]
    cbz x17, 1f
    br x17
1:  mov x0, #157
    bl __bf_resolve_and_call
    br x0
.symver __bf_raise, raise@@LIBC

.balign 4
.globl __bf_fscanf
.type __bf_fscanf, %function
__bf_fscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1264]
    cbz x17, 1f
    br x17
1:  mov x0, #158
    bl __bf_resolve_and_call
    br x0
.symver __bf_fscanf, fscanf@@LIBC

.balign 4
.globl __bf_pread64
.type __bf_pread64, %function
__bf_pread64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1272]
    cbz x17, 1f
    br x17
1:  mov x0, #159
    bl __bf_resolve_and_call
    br x0
.symver __bf_pread64, pread64@@LIBC

.balign 4
.globl __bf_ptrace
.type __bf_ptrace, %function
__bf_ptrace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1280]
    cbz x17, 1f
    br x17
1:  mov x0, #160
    bl __bf_resolve_and_call
    br x0
.symver __bf_ptrace, ptrace@@LIBC

.balign 4
.globl __bf_socketpair
.type __bf_socketpair, %function
__bf_socketpair:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1288]
    cbz x17, 1f
    br x17
1:  mov x0, #161
    bl __bf_resolve_and_call
    br x0
.symver __bf_socketpair, socketpair@@LIBC

.balign 4
.globl __bf_sendmsg
.type __bf_sendmsg, %function
__bf_sendmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1296]
    cbz x17, 1f
    br x17
1:  mov x0, #162
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendmsg, sendmsg@@LIBC

.balign 4
.globl __bf_recvmsg
.type __bf_recvmsg, %function
__bf_recvmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1304]
    cbz x17, 1f
    br x17
1:  mov x0, #163
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvmsg, recvmsg@@LIBC

.balign 4
.globl __bf___cmsg_nxthdr
.type __bf___cmsg_nxthdr, %function
__bf___cmsg_nxthdr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1312]
    cbz x17, 1f
    br x17
1:  mov x0, #164
    bl __bf_resolve_and_call
    br x0
.symver __bf___cmsg_nxthdr, __cmsg_nxthdr@@LIBC

.balign 4
.globl __bf_readlink
.type __bf_readlink, %function
__bf_readlink:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1320]
    cbz x17, 1f
    br x17
1:  mov x0, #165
    bl __bf_resolve_and_call
    br x0
.symver __bf_readlink, readlink@@LIBC

.balign 4
.globl __bf_printf
.type __bf_printf, %function
__bf_printf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1328]
    cbz x17, 1f
    br x17
1:  mov x0, #166
    bl __bf_resolve_and_call
    br x0
.symver __bf_printf, printf@@LIBC

.balign 4
.globl __bf_wmemchr
.type __bf_wmemchr, %function
__bf_wmemchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1336]
    cbz x17, 1f
    br x17
1:  mov x0, #167
    bl __bf_resolve_and_call
    br x0
.symver __bf_wmemchr, wmemchr@@LIBC

.balign 4
.globl __bf_localeconv
.type __bf_localeconv, %function
__bf_localeconv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1344]
    cbz x17, 1f
    br x17
1:  mov x0, #168
    bl __bf_resolve_and_call
    br x0
.symver __bf_localeconv, localeconv@@LIBC

.balign 4
.globl __bf___vsnprintf_chk
.type __bf___vsnprintf_chk, %function
__bf___vsnprintf_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1352]
    cbz x17, 1f
    br x17
1:  mov x0, #169
    bl __bf_resolve_and_call
    br x0
.symver __bf___vsnprintf_chk, __vsnprintf_chk@@LIBC

.balign 4
.globl __bf___memmove_chk
.type __bf___memmove_chk, %function
__bf___memmove_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1360]
    cbz x17, 1f
    br x17
1:  mov x0, #170
    bl __bf_resolve_and_call
    br x0
.symver __bf___memmove_chk, __memmove_chk@@LIBC

.balign 4
.globl __bf_sched_yield
.type __bf_sched_yield, %function
__bf_sched_yield:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1368]
    cbz x17, 1f
    br x17
1:  mov x0, #171
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_yield, sched_yield@@LIBC

.balign 4
.globl __bf_modf
.type __bf_modf, %function
__bf_modf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1376]
    cbz x17, 1f
    br x17
1:  mov x0, #172
    bl __bf_resolve_and_call
    br x0
.symver __bf_modf, modf@@LIBC

.balign 4
.globl __bf_strcasecmp
.type __bf_strcasecmp, %function
__bf_strcasecmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1384]
    cbz x17, 1f
    br x17
1:  mov x0, #173
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcasecmp, strcasecmp@@LIBC

.balign 4
.globl __bf_getnameinfo
.type __bf_getnameinfo, %function
__bf_getnameinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1392]
    cbz x17, 1f
    br x17
1:  mov x0, #174
    bl __bf_resolve_and_call
    br x0
.symver __bf_getnameinfo, getnameinfo@@LIBC

.balign 4
.globl __bf_strftime
.type __bf_strftime, %function
__bf_strftime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1400]
    cbz x17, 1f
    br x17
1:  mov x0, #175
    bl __bf_resolve_and_call
    br x0
.symver __bf_strftime, strftime@@LIBC

.balign 4
.globl __bf___strcpy_chk
.type __bf___strcpy_chk, %function
__bf___strcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1408]
    cbz x17, 1f
    br x17
1:  mov x0, #176
    bl __bf_resolve_and_call
    br x0
.symver __bf___strcpy_chk, __strcpy_chk@@LIBC

.balign 4
.globl __bf_frexpf
.type __bf_frexpf, %function
__bf_frexpf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1416]
    cbz x17, 1f
    br x17
1:  mov x0, #177
    bl __bf_resolve_and_call
    br x0
.symver __bf_frexpf, frexpf@@LIBC

.balign 4
.globl __bf_ldexpf
.type __bf_ldexpf, %function
__bf_ldexpf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1424]
    cbz x17, 1f
    br x17
1:  mov x0, #178
    bl __bf_resolve_and_call
    br x0
.symver __bf_ldexpf, ldexpf@@LIBC

.balign 4
.globl __bf_tanf
.type __bf_tanf, %function
__bf_tanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1432]
    cbz x17, 1f
    br x17
1:  mov x0, #179
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanf, tanf@@LIBC

.balign 4
.globl __bf_atanf
.type __bf_atanf, %function
__bf_atanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1440]
    cbz x17, 1f
    br x17
1:  mov x0, #180
    bl __bf_resolve_and_call
    br x0
.symver __bf_atanf, atanf@@LIBC

.balign 4
.globl __bf_erff
.type __bf_erff, %function
__bf_erff:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1448]
    cbz x17, 1f
    br x17
1:  mov x0, #181
    bl __bf_resolve_and_call
    br x0
.symver __bf_erff, erff@@LIBC

.balign 4
.globl __bf_acosf
.type __bf_acosf, %function
__bf_acosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1456]
    cbz x17, 1f
    br x17
1:  mov x0, #182
    bl __bf_resolve_and_call
    br x0
.symver __bf_acosf, acosf@@LIBC

.balign 4
.globl __bf_strstr
.type __bf_strstr, %function
__bf_strstr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1464]
    cbz x17, 1f
    br x17
1:  mov x0, #183
    bl __bf_resolve_and_call
    br x0
.symver __bf_strstr, strstr@@LIBC

.balign 4
.globl __bf_erfcf
.type __bf_erfcf, %function
__bf_erfcf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1472]
    cbz x17, 1f
    br x17
1:  mov x0, #184
    bl __bf_resolve_and_call
    br x0
.symver __bf_erfcf, erfcf@@LIBC

.balign 4
.globl __bf_modff
.type __bf_modff, %function
__bf_modff:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1480]
    cbz x17, 1f
    br x17
1:  mov x0, #185
    bl __bf_resolve_and_call
    br x0
.symver __bf_modff, modff@@LIBC

.balign 4
.globl __bf_coshf
.type __bf_coshf, %function
__bf_coshf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1488]
    cbz x17, 1f
    br x17
1:  mov x0, #186
    bl __bf_resolve_and_call
    br x0
.symver __bf_coshf, coshf@@LIBC

.balign 4
.globl __bf_sinhf
.type __bf_sinhf, %function
__bf_sinhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1496]
    cbz x17, 1f
    br x17
1:  mov x0, #187
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinhf, sinhf@@LIBC

.balign 4
.globl __bf_tanhf
.type __bf_tanhf, %function
__bf_tanhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1504]
    cbz x17, 1f
    br x17
1:  mov x0, #188
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanhf, tanhf@@LIBC

.balign 4
.globl __bf_atan2
.type __bf_atan2, %function
__bf_atan2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1512]
    cbz x17, 1f
    br x17
1:  mov x0, #189
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan2, atan2@@LIBC

.balign 4
.globl __bf_cbrtf
.type __bf_cbrtf, %function
__bf_cbrtf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1520]
    cbz x17, 1f
    br x17
1:  mov x0, #190
    bl __bf_resolve_and_call
    br x0
.symver __bf_cbrtf, cbrtf@@LIBC

.balign 4
.globl __bf___strchr_chk
.type __bf___strchr_chk, %function
__bf___strchr_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1528]
    cbz x17, 1f
    br x17
1:  mov x0, #191
    bl __bf_resolve_and_call
    br x0
.symver __bf___strchr_chk, __strchr_chk@@LIBC

.balign 4
.globl __bf_clock
.type __bf_clock, %function
__bf_clock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1536]
    cbz x17, 1f
    br x17
1:  mov x0, #192
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock, clock@@LIBC

.balign 4
.globl __bf_fileno
.type __bf_fileno, %function
__bf_fileno:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1544]
    cbz x17, 1f
    br x17
1:  mov x0, #193
    bl __bf_resolve_and_call
    br x0
.symver __bf_fileno, fileno@@LIBC

.balign 4
.globl __bf_remainderf
.type __bf_remainderf, %function
__bf_remainderf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1552]
    cbz x17, 1f
    br x17
1:  mov x0, #194
    bl __bf_resolve_and_call
    br x0
.symver __bf_remainderf, remainderf@@LIBC

.balign 4
.globl __bf_nan
.type __bf_nan, %function
__bf_nan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1560]
    cbz x17, 1f
    br x17
1:  mov x0, #195
    bl __bf_resolve_and_call
    br x0
.symver __bf_nan, nan@@LIBC

.balign 4
.globl __bf_qsort
.type __bf_qsort, %function
__bf_qsort:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1568]
    cbz x17, 1f
    br x17
1:  mov x0, #196
    bl __bf_resolve_and_call
    br x0
.symver __bf_qsort, qsort@@LIBC

.balign 4
.globl __bf_nextafterf
.type __bf_nextafterf, %function
__bf_nextafterf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1576]
    cbz x17, 1f
    br x17
1:  mov x0, #197
    bl __bf_resolve_and_call
    br x0
.symver __bf_nextafterf, nextafterf@@LIBC

.balign 4
.globl __bf_acos
.type __bf_acos, %function
__bf_acos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1584]
    cbz x17, 1f
    br x17
1:  mov x0, #198
    bl __bf_resolve_and_call
    br x0
.symver __bf_acos, acos@@LIBC

.balign 4
.globl __bf_asin
.type __bf_asin, %function
__bf_asin:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1592]
    cbz x17, 1f
    br x17
1:  mov x0, #199
    bl __bf_resolve_and_call
    br x0
.symver __bf_asin, asin@@LIBC

.balign 4
.globl __bf_ilogb
.type __bf_ilogb, %function
__bf_ilogb:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1600]
    cbz x17, 1f
    br x17
1:  mov x0, #200
    bl __bf_resolve_and_call
    br x0
.symver __bf_ilogb, ilogb@@LIBC

.balign 4
.globl __bf___FD_SET_chk
.type __bf___FD_SET_chk, %function
__bf___FD_SET_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1608]
    cbz x17, 1f
    br x17
1:  mov x0, #201
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_SET_chk, __FD_SET_chk@@LIBC

.balign 4
.globl __bf_select
.type __bf_select, %function
__bf_select:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1616]
    cbz x17, 1f
    br x17
1:  mov x0, #202
    bl __bf_resolve_and_call
    br x0
.symver __bf_select, select@@LIBC

.balign 4
.globl __bf___FD_ISSET_chk
.type __bf___FD_ISSET_chk, %function
__bf___FD_ISSET_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1624]
    cbz x17, 1f
    br x17
1:  mov x0, #203
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_ISSET_chk, __FD_ISSET_chk@@LIBC

.balign 4
.globl __bf_sendto
.type __bf_sendto, %function
__bf_sendto:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1632]
    cbz x17, 1f
    br x17
1:  mov x0, #204
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendto, sendto@@LIBC

.balign 4
.globl __bf_recvfrom
.type __bf_recvfrom, %function
__bf_recvfrom:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1640]
    cbz x17, 1f
    br x17
1:  mov x0, #205
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvfrom, recvfrom@@LIBC

.balign 4
.globl __bf___strcat_chk
.type __bf___strcat_chk, %function
__bf___strcat_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1648]
    cbz x17, 1f
    br x17
1:  mov x0, #206
    bl __bf_resolve_and_call
    br x0
.symver __bf___strcat_chk, __strcat_chk@@LIBC

.balign 4
.globl __bf_setpriority
.type __bf_setpriority, %function
__bf_setpriority:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1656]
    cbz x17, 1f
    br x17
1:  mov x0, #207
    bl __bf_resolve_and_call
    br x0
.symver __bf_setpriority, setpriority@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_init
.type __bf_pthread_mutexattr_init, %function
__bf_pthread_mutexattr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1664]
    cbz x17, 1f
    br x17
1:  mov x0, #208
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_init, pthread_mutexattr_init@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_settype
.type __bf_pthread_mutexattr_settype, %function
__bf_pthread_mutexattr_settype:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1672]
    cbz x17, 1f
    br x17
1:  mov x0, #209
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_settype, pthread_mutexattr_settype@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_destroy
.type __bf_pthread_mutexattr_destroy, %function
__bf_pthread_mutexattr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1680]
    cbz x17, 1f
    br x17
1:  mov x0, #210
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_destroy, pthread_mutexattr_destroy@@LIBC

.balign 4
.globl __bf_sem_init
.type __bf_sem_init, %function
__bf_sem_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1688]
    cbz x17, 1f
    br x17
1:  mov x0, #211
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_init, sem_init@@LIBC

.balign 4
.globl __bf_sem_destroy
.type __bf_sem_destroy, %function
__bf_sem_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1696]
    cbz x17, 1f
    br x17
1:  mov x0, #212
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_destroy, sem_destroy@@LIBC

.balign 4
.globl __bf_sem_wait
.type __bf_sem_wait, %function
__bf_sem_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1704]
    cbz x17, 1f
    br x17
1:  mov x0, #213
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_wait, sem_wait@@LIBC

.balign 4
.globl __bf_sem_post
.type __bf_sem_post, %function
__bf_sem_post:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1712]
    cbz x17, 1f
    br x17
1:  mov x0, #214
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_post, sem_post@@LIBC

.balign 4
.globl __bf_inet_ntop
.type __bf_inet_ntop, %function
__bf_inet_ntop:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1720]
    cbz x17, 1f
    br x17
1:  mov x0, #215
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_ntop, inet_ntop@@LIBC

.balign 4
.globl __bf_inet_pton
.type __bf_inet_pton, %function
__bf_inet_pton:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1728]
    cbz x17, 1f
    br x17
1:  mov x0, #216
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_pton, inet_pton@@LIBC

.balign 4
.globl __bf_strncasecmp
.type __bf_strncasecmp, %function
__bf_strncasecmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1736]
    cbz x17, 1f
    br x17
1:  mov x0, #217
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncasecmp, strncasecmp@@LIBC

.balign 4
.globl __bf_pthread_attr_setschedparam
.type __bf_pthread_attr_setschedparam, %function
__bf_pthread_attr_setschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1744]
    cbz x17, 1f
    br x17
1:  mov x0, #218
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setschedparam, pthread_attr_setschedparam@@LIBC

.balign 4
.globl __bf_bind
.type __bf_bind, %function
__bf_bind:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1752]
    cbz x17, 1f
    br x17
1:  mov x0, #219
    bl __bf_resolve_and_call
    br x0
.symver __bf_bind, bind@@LIBC

.balign 4
.globl __bf_getsockname
.type __bf_getsockname, %function
__bf_getsockname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1760]
    cbz x17, 1f
    br x17
1:  mov x0, #220
    bl __bf_resolve_and_call
    br x0
.symver __bf_getsockname, getsockname@@LIBC

.balign 4
.globl __bf_gethostname
.type __bf_gethostname, %function
__bf_gethostname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1768]
    cbz x17, 1f
    br x17
1:  mov x0, #221
    bl __bf_resolve_and_call
    br x0
.symver __bf_gethostname, gethostname@@LIBC

.balign 4
.globl __bf___sendto_chk
.type __bf___sendto_chk, %function
__bf___sendto_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1776]
    cbz x17, 1f
    br x17
1:  mov x0, #222
    bl __bf_resolve_and_call
    br x0
.symver __bf___sendto_chk, __sendto_chk@@LIBC_O

.balign 4
.globl __bf_puts
.type __bf_puts, %function
__bf_puts:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1784]
    cbz x17, 1f
    br x17
1:  mov x0, #223
    bl __bf_resolve_and_call
    br x0
.symver __bf_puts, puts@@LIBC

.balign 4
.globl __bf_gai_strerror
.type __bf_gai_strerror, %function
__bf_gai_strerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1792]
    cbz x17, 1f
    br x17
1:  mov x0, #224
    bl __bf_resolve_and_call
    br x0
.symver __bf_gai_strerror, gai_strerror@@LIBC

.balign 4
.globl __bf___write_chk
.type __bf___write_chk, %function
__bf___write_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1800]
    cbz x17, 1f
    br x17
1:  mov x0, #225
    bl __bf_resolve_and_call
    br x0
.symver __bf___write_chk, __write_chk@@LIBC_N

.balign 4
.globl __bf___poll_chk
.type __bf___poll_chk, %function
__bf___poll_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1808]
    cbz x17, 1f
    br x17
1:  mov x0, #226
    bl __bf_resolve_and_call
    br x0
.symver __bf___poll_chk, __poll_chk@@LIBC

.balign 4
.globl __bf_vprintf
.type __bf_vprintf, %function
__bf_vprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1816]
    cbz x17, 1f
    br x17
1:  mov x0, #227
    bl __bf_resolve_and_call
    br x0
.symver __bf_vprintf, vprintf@@LIBC

.balign 4
.globl __bf_usleep
.type __bf_usleep, %function
__bf_usleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1824]
    cbz x17, 1f
    br x17
1:  mov x0, #228
    bl __bf_resolve_and_call
    br x0
.symver __bf_usleep, usleep@@LIBC

.balign 4
.globl __bf_pthread_kill
.type __bf_pthread_kill, %function
__bf_pthread_kill:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1832]
    cbz x17, 1f
    br x17
1:  mov x0, #229
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_kill, pthread_kill@@LIBC

.balign 4
.globl __bf_pthread_detach
.type __bf_pthread_detach, %function
__bf_pthread_detach:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1840]
    cbz x17, 1f
    br x17
1:  mov x0, #230
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_detach, pthread_detach@@LIBC

.balign 4
.globl __bf_exit
.type __bf_exit, %function
__bf_exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1848]
    cbz x17, 1f
    br x17
1:  mov x0, #231
    bl __bf_resolve_and_call
    br x0
.symver __bf_exit, exit@@LIBC

.balign 4
.globl __bf_ferror
.type __bf_ferror, %function
__bf_ferror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1856]
    cbz x17, 1f
    br x17
1:  mov x0, #232
    bl __bf_resolve_and_call
    br x0
.symver __bf_ferror, ferror@@LIBC

.balign 4
.globl __bf_clearerr
.type __bf_clearerr, %function
__bf_clearerr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1864]
    cbz x17, 1f
    br x17
1:  mov x0, #233
    bl __bf_resolve_and_call
    br x0
.symver __bf_clearerr, clearerr@@LIBC

.balign 4
.globl __bf_wcslen
.type __bf_wcslen, %function
__bf_wcslen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1872]
    cbz x17, 1f
    br x17
1:  mov x0, #234
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcslen, wcslen@@LIBC

.balign 4
.globl __bf_wmemcmp
.type __bf_wmemcmp, %function
__bf_wmemcmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1880]
    cbz x17, 1f
    br x17
1:  mov x0, #235
    bl __bf_resolve_and_call
    br x0
.symver __bf_wmemcmp, wmemcmp@@LIBC

.balign 4
.globl __bf_exp
.type __bf_exp, %function
__bf_exp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1888]
    cbz x17, 1f
    br x17
1:  mov x0, #236
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp, exp@@LIBC

.balign 4
.globl __bf_pow
.type __bf_pow, %function
__bf_pow:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1896]
    cbz x17, 1f
    br x17
1:  mov x0, #237
    bl __bf_resolve_and_call
    br x0
.symver __bf_pow, pow@@LIBC

.balign 4
.globl __bf_fmod
.type __bf_fmod, %function
__bf_fmod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1904]
    cbz x17, 1f
    br x17
1:  mov x0, #238
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmod, fmod@@LIBC

.balign 4
.globl __bf_log
.type __bf_log, %function
__bf_log:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1912]
    cbz x17, 1f
    br x17
1:  mov x0, #239
    bl __bf_resolve_and_call
    br x0
.symver __bf_log, log@@LIBC

.balign 4
.globl __bf_log2
.type __bf_log2, %function
__bf_log2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1920]
    cbz x17, 1f
    br x17
1:  mov x0, #240
    bl __bf_resolve_and_call
    br x0
.symver __bf_log2, log2@@LIBC

.balign 4
.globl __bf_log10
.type __bf_log10, %function
__bf_log10:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1928]
    cbz x17, 1f
    br x17
1:  mov x0, #241
    bl __bf_resolve_and_call
    br x0
.symver __bf_log10, log10@@LIBC

.balign 4
.globl __bf_round
.type __bf_round, %function
__bf_round:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1936]
    cbz x17, 1f
    br x17
1:  mov x0, #242
    bl __bf_resolve_and_call
    br x0
.symver __bf_round, round@@LIBC

.balign 4
.globl __bf_frexp
.type __bf_frexp, %function
__bf_frexp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1944]
    cbz x17, 1f
    br x17
1:  mov x0, #243
    bl __bf_resolve_and_call
    br x0
.symver __bf_frexp, frexp@@LIBC

.balign 4
.globl __bf_sin
.type __bf_sin, %function
__bf_sin:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1952]
    cbz x17, 1f
    br x17
1:  mov x0, #244
    bl __bf_resolve_and_call
    br x0
.symver __bf_sin, sin@@LIBC

.balign 4
.globl __bf_sinh
.type __bf_sinh, %function
__bf_sinh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1960]
    cbz x17, 1f
    br x17
1:  mov x0, #245
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinh, sinh@@LIBC

.balign 4
.globl __bf_cos
.type __bf_cos, %function
__bf_cos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1968]
    cbz x17, 1f
    br x17
1:  mov x0, #246
    bl __bf_resolve_and_call
    br x0
.symver __bf_cos, cos@@LIBC

.balign 4
.globl __bf_cosh
.type __bf_cosh, %function
__bf_cosh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1976]
    cbz x17, 1f
    br x17
1:  mov x0, #247
    bl __bf_resolve_and_call
    br x0
.symver __bf_cosh, cosh@@LIBC

.balign 4
.globl __bf_tan
.type __bf_tan, %function
__bf_tan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1984]
    cbz x17, 1f
    br x17
1:  mov x0, #248
    bl __bf_resolve_and_call
    br x0
.symver __bf_tan, tan@@LIBC

.balign 4
.globl __bf_tanh
.type __bf_tanh, %function
__bf_tanh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1992]
    cbz x17, 1f
    br x17
1:  mov x0, #249
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanh, tanh@@LIBC

.balign 4
.globl __bf_atol
.type __bf_atol, %function
__bf_atol:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2000]
    cbz x17, 1f
    br x17
1:  mov x0, #250
    bl __bf_resolve_and_call
    br x0
.symver __bf_atol, atol@@LIBC

.balign 4
.globl __bf_atof
.type __bf_atof, %function
__bf_atof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2008]
    cbz x17, 1f
    br x17
1:  mov x0, #251
    bl __bf_resolve_and_call
    br x0
.symver __bf_atof, atof@@LIBC

.balign 4
.globl __bf_strspn
.type __bf_strspn, %function
__bf_strspn:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2016]
    cbz x17, 1f
    br x17
1:  mov x0, #252
    bl __bf_resolve_and_call
    br x0
.symver __bf_strspn, strspn@@LIBC

.balign 4
.globl __bf_strtof
.type __bf_strtof, %function
__bf_strtof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2024]
    cbz x17, 1f
    br x17
1:  mov x0, #253
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtof, strtof@@LIBC

.balign 4
.globl __bf_ioctl
.type __bf_ioctl, %function
__bf_ioctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2032]
    cbz x17, 1f
    br x17
1:  mov x0, #254
    bl __bf_resolve_and_call
    br x0
.symver __bf_ioctl, ioctl@@LIBC

.balign 4
.globl __bf_getpeername
.type __bf_getpeername, %function
__bf_getpeername:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2040]
    cbz x17, 1f
    br x17
1:  mov x0, #255
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpeername, getpeername@@LIBC

.balign 4
.globl __bf_listen
.type __bf_listen, %function
__bf_listen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2048]
    cbz x17, 1f
    br x17
1:  mov x0, #256
    bl __bf_resolve_and_call
    br x0
.symver __bf_listen, listen@@LIBC

.balign 4
.globl __bf_accept
.type __bf_accept, %function
__bf_accept:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2056]
    cbz x17, 1f
    br x17
1:  mov x0, #257
    bl __bf_resolve_and_call
    br x0
.symver __bf_accept, accept@@LIBC

.balign 4
.globl __bf_epoll_create
.type __bf_epoll_create, %function
__bf_epoll_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2064]
    cbz x17, 1f
    br x17
1:  mov x0, #258
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_create, epoll_create@@LIBC

.balign 4
.globl __bf___FD_CLR_chk
.type __bf___FD_CLR_chk, %function
__bf___FD_CLR_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2072]
    cbz x17, 1f
    br x17
1:  mov x0, #259
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_CLR_chk, __FD_CLR_chk@@LIBC

.balign 4
.globl __bf_expm1
.type __bf_expm1, %function
__bf_expm1:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2080]
    cbz x17, 1f
    br x17
1:  mov x0, #260
    bl __bf_resolve_and_call
    br x0
.symver __bf_expm1, expm1@@LIBC

.balign 4
.globl __bf_if_indextoname
.type __bf_if_indextoname, %function
__bf_if_indextoname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2088]
    cbz x17, 1f
    br x17
1:  mov x0, #261
    bl __bf_resolve_and_call
    br x0
.symver __bf_if_indextoname, if_indextoname@@LIBC

.balign 4
.globl __bf_sigaddset
.type __bf_sigaddset, %function
__bf_sigaddset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2096]
    cbz x17, 1f
    br x17
1:  mov x0, #262
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaddset, sigaddset@@LIBC

.balign 4
.globl __bf_pthread_sigmask
.type __bf_pthread_sigmask, %function
__bf_pthread_sigmask:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2104]
    cbz x17, 1f
    br x17
1:  mov x0, #263
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_sigmask, pthread_sigmask@@LIBC

.balign 4
.globl __bf_fgets
.type __bf_fgets, %function
__bf_fgets:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2112]
    cbz x17, 1f
    br x17
1:  mov x0, #264
    bl __bf_resolve_and_call
    br x0
.symver __bf_fgets, fgets@@LIBC

.balign 4
.globl __bf_setjmp
.type __bf_setjmp, %function
__bf_setjmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2120]
    cbz x17, 1f
    br x17
1:  mov x0, #265
    bl __bf_resolve_and_call
    br x0
.symver __bf_setjmp, setjmp@@LIBC

.balign 4
.globl __bf_longjmp
.type __bf_longjmp, %function
__bf_longjmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2128]
    cbz x17, 1f
    br x17
1:  mov x0, #266
    bl __bf_resolve_and_call
    br x0
.symver __bf_longjmp, longjmp@@LIBC

.balign 4
.globl __bf_pthread_condattr_init
.type __bf_pthread_condattr_init, %function
__bf_pthread_condattr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2136]
    cbz x17, 1f
    br x17
1:  mov x0, #267
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_init, pthread_condattr_init@@LIBC

.balign 4
.globl __bf_pthread_condattr_setclock
.type __bf_pthread_condattr_setclock, %function
__bf_pthread_condattr_setclock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2144]
    cbz x17, 1f
    br x17
1:  mov x0, #268
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_setclock, pthread_condattr_setclock@@LIBC

.balign 4
.globl __bf_pthread_condattr_destroy
.type __bf_pthread_condattr_destroy, %function
__bf_pthread_condattr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2152]
    cbz x17, 1f
    br x17
1:  mov x0, #269
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_destroy, pthread_condattr_destroy@@LIBC

.balign 4
.globl __bf_sched_get_priority_min
.type __bf_sched_get_priority_min, %function
__bf_sched_get_priority_min:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2160]
    cbz x17, 1f
    br x17
1:  mov x0, #270
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_get_priority_min, sched_get_priority_min@@LIBC

.balign 4
.globl __bf_pthread_setschedparam
.type __bf_pthread_setschedparam, %function
__bf_pthread_setschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2168]
    cbz x17, 1f
    br x17
1:  mov x0, #271
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setschedparam, pthread_setschedparam@@LIBC

.balign 4
.globl __bf_getgid
.type __bf_getgid, %function
__bf_getgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2176]
    cbz x17, 1f
    br x17
1:  mov x0, #272
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgid, getgid@@LIBC

.balign 4
.globl __bf_getegid
.type __bf_getegid, %function
__bf_getegid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2184]
    cbz x17, 1f
    br x17
1:  mov x0, #273
    bl __bf_resolve_and_call
    br x0
.symver __bf_getegid, getegid@@LIBC

.balign 4
.globl __bf_random
.type __bf_random, %function
__bf_random:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2192]
    cbz x17, 1f
    br x17
1:  mov x0, #274
    bl __bf_resolve_and_call
    br x0
.symver __bf_random, random@@LIBC

.balign 4
.globl __bf_sigfillset
.type __bf_sigfillset, %function
__bf_sigfillset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2200]
    cbz x17, 1f
    br x17
1:  mov x0, #275
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigfillset, sigfillset@@LIBC

.balign 4
.globl __bf_fdopen
.type __bf_fdopen, %function
__bf_fdopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2208]
    cbz x17, 1f
    br x17
1:  mov x0, #276
    bl __bf_resolve_and_call
    br x0
.symver __bf_fdopen, fdopen@@LIBC

.balign 4
.globl __bf_timerfd_create
.type __bf_timerfd_create, %function
__bf_timerfd_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2216]
    cbz x17, 1f
    br x17
1:  mov x0, #277
    bl __bf_resolve_and_call
    br x0
.symver __bf_timerfd_create, timerfd_create@@LIBC

.balign 4
.globl __bf_timerfd_settime
.type __bf_timerfd_settime, %function
__bf_timerfd_settime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2224]
    cbz x17, 1f
    br x17
1:  mov x0, #278
    bl __bf_resolve_and_call
    br x0
.symver __bf_timerfd_settime, timerfd_settime@@LIBC

.balign 4
.globl __bf_fputc
.type __bf_fputc, %function
__bf_fputc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2232]
    cbz x17, 1f
    br x17
1:  mov x0, #279
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputc, fputc@@LIBC

.balign 4
.globl __bf_bsearch
.type __bf_bsearch, %function
__bf_bsearch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2240]
    cbz x17, 1f
    br x17
1:  mov x0, #280
    bl __bf_resolve_and_call
    br x0
.symver __bf_bsearch, bsearch@@LIBC

.balign 4
.globl __bf_vfprintf
.type __bf_vfprintf, %function
__bf_vfprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2248]
    cbz x17, 1f
    br x17
1:  mov x0, #281
    bl __bf_resolve_and_call
    br x0
.symver __bf_vfprintf, vfprintf@@LIBC

.balign 4
.globl __bf_pthread_exit
.type __bf_pthread_exit, %function
__bf_pthread_exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2256]
    cbz x17, 1f
    br x17
1:  mov x0, #282
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_exit, pthread_exit@@LIBC

.balign 4
.globl __bf_finitef
.type __bf_finitef, %function
__bf_finitef:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2264]
    cbz x17, 1f
    br x17
1:  mov x0, #283
    bl __bf_resolve_and_call
    br x0
.symver __bf_finitef, finitef@@LIBC

.balign 4
.globl __bf_cbrt
.type __bf_cbrt, %function
__bf_cbrt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2272]
    cbz x17, 1f
    br x17
1:  mov x0, #284
    bl __bf_resolve_and_call
    br x0
.symver __bf_cbrt, cbrt@@LIBC

.balign 4
.globl __bf_remquof
.type __bf_remquof, %function
__bf_remquof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2280]
    cbz x17, 1f
    br x17
1:  mov x0, #285
    bl __bf_resolve_and_call
    br x0
.symver __bf_remquof, remquof@@LIBC

.balign 4
.globl __bf_strcspn
.type __bf_strcspn, %function
__bf_strcspn:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2288]
    cbz x17, 1f
    br x17
1:  mov x0, #286
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcspn, strcspn@@LIBC

.balign 4
.globl __bf_gmtime_r
.type __bf_gmtime_r, %function
__bf_gmtime_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2296]
    cbz x17, 1f
    br x17
1:  mov x0, #287
    bl __bf_resolve_and_call
    br x0
.symver __bf_gmtime_r, gmtime_r@@LIBC

.balign 4
.globl __bf_difftime
.type __bf_difftime, %function
__bf_difftime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2304]
    cbz x17, 1f
    br x17
1:  mov x0, #288
    bl __bf_resolve_and_call
    br x0
.symver __bf_difftime, difftime@@LIBC

.balign 4
.globl __bf_strpbrk
.type __bf_strpbrk, %function
__bf_strpbrk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2312]
    cbz x17, 1f
    br x17
1:  mov x0, #289
    bl __bf_resolve_and_call
    br x0
.symver __bf_strpbrk, strpbrk@@LIBC

.balign 4
.globl __bf_shutdown
.type __bf_shutdown, %function
__bf_shutdown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2320]
    cbz x17, 1f
    br x17
1:  mov x0, #290
    bl __bf_resolve_and_call
    br x0
.symver __bf_shutdown, shutdown@@LIBC

.balign 4
.globl __bf_memrchr
.type __bf_memrchr, %function
__bf_memrchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2328]
    cbz x17, 1f
    br x17
1:  mov x0, #291
    bl __bf_resolve_and_call
    br x0
.symver __bf_memrchr, memrchr@@LIBC

.balign 4
.globl __bf_accept4
.type __bf_accept4, %function
__bf_accept4:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2336]
    cbz x17, 1f
    br x17
1:  mov x0, #292
    bl __bf_resolve_and_call
    br x0
.symver __bf_accept4, accept4@@LIBC

.balign 4
.globl __bf_if_nametoindex
.type __bf_if_nametoindex, %function
__bf_if_nametoindex:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2344]
    cbz x17, 1f
    br x17
1:  mov x0, #293
    bl __bf_resolve_and_call
    br x0
.symver __bf_if_nametoindex, if_nametoindex@@LIBC

.balign 4
.globl __bf_setvbuf
.type __bf_setvbuf, %function
__bf_setvbuf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2352]
    cbz x17, 1f
    br x17
1:  mov x0, #294
    bl __bf_resolve_and_call
    br x0
.symver __bf_setvbuf, setvbuf@@LIBC

.balign 4
.globl __bf_realpath
.type __bf_realpath, %function
__bf_realpath:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2360]
    cbz x17, 1f
    br x17
1:  mov x0, #295
    bl __bf_resolve_and_call
    br x0
.symver __bf_realpath, realpath@@LIBC

.balign 4
.globl __bf_recvmmsg
.type __bf_recvmmsg, %function
__bf_recvmmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2368]
    cbz x17, 1f
    br x17
1:  mov x0, #296
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvmmsg, recvmmsg@@LIBC

.balign 4
.globl __bf_getcwd
.type __bf_getcwd, %function
__bf_getcwd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2376]
    cbz x17, 1f
    br x17
1:  mov x0, #297
    bl __bf_resolve_and_call
    br x0
.symver __bf_getcwd, getcwd@@LIBC

.balign 4
.globl __bf_pread
.type __bf_pread, %function
__bf_pread:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2384]
    cbz x17, 1f
    br x17
1:  mov x0, #298
    bl __bf_resolve_and_call
    br x0
.symver __bf_pread, pread@@LIBC

.balign 4
.globl __bf_pwrite
.type __bf_pwrite, %function
__bf_pwrite:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2392]
    cbz x17, 1f
    br x17
1:  mov x0, #299
    bl __bf_resolve_and_call
    br x0
.symver __bf_pwrite, pwrite@@LIBC

.balign 4
.globl __bf_fchmod
.type __bf_fchmod, %function
__bf_fchmod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2400]
    cbz x17, 1f
    br x17
1:  mov x0, #300
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchmod, fchmod@@LIBC

.balign 4
.globl __bf_fchown
.type __bf_fchown, %function
__bf_fchown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2408]
    cbz x17, 1f
    br x17
1:  mov x0, #301
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchown, fchown@@LIBC

.balign 4
.globl __bf_mremap
.type __bf_mremap, %function
__bf_mremap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2416]
    cbz x17, 1f
    br x17
1:  mov x0, #302
    bl __bf_resolve_and_call
    br x0
.symver __bf_mremap, mremap@@LIBC

.balign 4
.globl __bf_fsync
.type __bf_fsync, %function
__bf_fsync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2424]
    cbz x17, 1f
    br x17
1:  mov x0, #303
    bl __bf_resolve_and_call
    br x0
.symver __bf_fsync, fsync@@LIBC

.balign 4
.globl __bf_utimes
.type __bf_utimes, %function
__bf_utimes:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2432]
    cbz x17, 1f
    br x17
1:  mov x0, #304
    bl __bf_resolve_and_call
    br x0
.symver __bf_utimes, utimes@@LIBC

.balign 4
.globl __bf_msync
.type __bf_msync, %function
__bf_msync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2440]
    cbz x17, 1f
    br x17
1:  mov x0, #305
    bl __bf_resolve_and_call
    br x0
.symver __bf_msync, msync@@LIBC

.balign 4
.globl __bf_statvfs
.type __bf_statvfs, %function
__bf_statvfs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2448]
    cbz x17, 1f
    br x17
1:  mov x0, #306
    bl __bf_resolve_and_call
    br x0
.symver __bf_statvfs, statvfs@@LIBC

.balign 4
.globl __bf_mallinfo
.type __bf_mallinfo, %function
__bf_mallinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2456]
    cbz x17, 1f
    br x17
1:  mov x0, #307
    bl __bf_resolve_and_call
    br x0
.symver __bf_mallinfo, mallinfo@@LIBC

.balign 4
.globl __bf___readlink_chk
.type __bf___readlink_chk, %function
__bf___readlink_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2464]
    cbz x17, 1f
    br x17
1:  mov x0, #308
    bl __bf_resolve_and_call
    br x0
.symver __bf___readlink_chk, __readlink_chk@@LIBC

.balign 4
.globl __bf___gnu_strerror_r
.type __bf___gnu_strerror_r, %function
__bf___gnu_strerror_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2472]
    cbz x17, 1f
    br x17
1:  mov x0, #309
    bl __bf_resolve_and_call
    br x0
.symver __bf___gnu_strerror_r, __gnu_strerror_r@@LIBC

.balign 4
.globl __bf_pthread_getschedparam
.type __bf_pthread_getschedparam, %function
__bf_pthread_getschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2480]
    cbz x17, 1f
    br x17
1:  mov x0, #310
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getschedparam, pthread_getschedparam@@LIBC

.balign 4
.globl __bf_sinf
.type __bf_sinf, %function
__bf_sinf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2488]
    cbz x17, 1f
    br x17
1:  mov x0, #311
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinf, sinf@@LIBC

.balign 4
.globl __bf_sincosf
.type __bf_sincosf, %function
__bf_sincosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2496]
    cbz x17, 1f
    br x17
1:  mov x0, #312
    bl __bf_resolve_and_call
    br x0
.symver __bf_sincosf, sincosf@@LIBC

.balign 4
.globl __bf_exp2
.type __bf_exp2, %function
__bf_exp2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2504]
    cbz x17, 1f
    br x17
1:  mov x0, #313
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp2, exp2@@LIBC

.balign 4
.globl __bf_sincos
.type __bf_sincos, %function
__bf_sincos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2512]
    cbz x17, 1f
    br x17
1:  mov x0, #314
    bl __bf_resolve_and_call
    br x0
.symver __bf_sincos, sincos@@LIBC

.balign 4
.globl __bf_fmal
.type __bf_fmal, %function
__bf_fmal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2520]
    cbz x17, 1f
    br x17
1:  mov x0, #315
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmal, fmal@@LIBC

.balign 4
.globl __bf_exp2f
.type __bf_exp2f, %function
__bf_exp2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2528]
    cbz x17, 1f
    br x17
1:  mov x0, #316
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp2f, exp2f@@LIBC

.balign 4
.globl __bf_log10f
.type __bf_log10f, %function
__bf_log10f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2536]
    cbz x17, 1f
    br x17
1:  mov x0, #317
    bl __bf_resolve_and_call
    br x0
.symver __bf_log10f, log10f@@LIBC

.balign 4
.globl __bf_logf
.type __bf_logf, %function
__bf_logf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2544]
    cbz x17, 1f
    br x17
1:  mov x0, #318
    bl __bf_resolve_and_call
    br x0
.symver __bf_logf, logf@@LIBC

.balign 4
.globl __bf_powf
.type __bf_powf, %function
__bf_powf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2552]
    cbz x17, 1f
    br x17
1:  mov x0, #319
    bl __bf_resolve_and_call
    br x0
.symver __bf_powf, powf@@LIBC

.balign 4
.globl __bf_fmodf
.type __bf_fmodf, %function
__bf_fmodf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2560]
    cbz x17, 1f
    br x17
1:  mov x0, #320
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmodf, fmodf@@LIBC

.balign 4
.globl __bf_log2f
.type __bf_log2f, %function
__bf_log2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2568]
    cbz x17, 1f
    br x17
1:  mov x0, #321
    bl __bf_resolve_and_call
    br x0
.symver __bf_log2f, log2f@@LIBC

.balign 4
.globl __bf_expf
.type __bf_expf, %function
__bf_expf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2576]
    cbz x17, 1f
    br x17
1:  mov x0, #322
    bl __bf_resolve_and_call
    br x0
.symver __bf_expf, expf@@LIBC

.balign 4
.globl __bf_powl
.type __bf_powl, %function
__bf_powl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2584]
    cbz x17, 1f
    br x17
1:  mov x0, #323
    bl __bf_resolve_and_call
    br x0
.symver __bf_powl, powl@@LIBC

.balign 4
.globl __bf_cosf
.type __bf_cosf, %function
__bf_cosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2592]
    cbz x17, 1f
    br x17
1:  mov x0, #324
    bl __bf_resolve_and_call
    br x0
.symver __bf_cosf, cosf@@LIBC

.balign 4
.globl __bf_pthread_key_delete
.type __bf_pthread_key_delete, %function
__bf_pthread_key_delete:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2600]
    cbz x17, 1f
    br x17
1:  mov x0, #325
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_key_delete, pthread_key_delete@@LIBC

.balign 4
.globl __bf_sysinfo
.type __bf_sysinfo, %function
__bf_sysinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2608]
    cbz x17, 1f
    br x17
1:  mov x0, #326
    bl __bf_resolve_and_call
    br x0
.symver __bf_sysinfo, sysinfo@@LIBC

.balign 4
.globl __bf_madvise
.type __bf_madvise, %function
__bf_madvise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2616]
    cbz x17, 1f
    br x17
1:  mov x0, #327
    bl __bf_resolve_and_call
    br x0
.symver __bf_madvise, madvise@@LIBC

.balign 4
.globl __bf_pthread_setname_np
.type __bf_pthread_setname_np, %function
__bf_pthread_setname_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2624]
    cbz x17, 1f
    br x17
1:  mov x0, #328
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setname_np, pthread_setname_np@@LIBC

.balign 4
.globl __bf_pthread_getattr_np
.type __bf_pthread_getattr_np, %function
__bf_pthread_getattr_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2632]
    cbz x17, 1f
    br x17
1:  mov x0, #329
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getattr_np, pthread_getattr_np@@LIBC

.balign 4
.globl __bf_pthread_attr_getstack
.type __bf_pthread_attr_getstack, %function
__bf_pthread_attr_getstack:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2640]
    cbz x17, 1f
    br x17
1:  mov x0, #330
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_getstack, pthread_attr_getstack@@LIBC

.balign 4
.globl __bf_mlock
.type __bf_mlock, %function
__bf_mlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2648]
    cbz x17, 1f
    br x17
1:  mov x0, #331
    bl __bf_resolve_and_call
    br x0
.symver __bf_mlock, mlock@@LIBC

.balign 4
.globl __bf_dl_iterate_phdr
.type __bf_dl_iterate_phdr, %function
__bf_dl_iterate_phdr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2656]
    cbz x17, 1f
    br x17
1:  mov x0, #332
    bl __bf_resolve_and_call
    br x0
.symver __bf_dl_iterate_phdr, dl_iterate_phdr@@LIBC

.balign 4
.globl __bf_isspace
.type __bf_isspace, %function
__bf_isspace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2664]
    cbz x17, 1f
    br x17
1:  mov x0, #333
    bl __bf_resolve_and_call
    br x0
.symver __bf_isspace, isspace@@LIBC

.balign 4
.globl __bf_gethostbyname
.type __bf_gethostbyname, %function
__bf_gethostbyname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2672]
    cbz x17, 1f
    br x17
1:  mov x0, #334
    bl __bf_resolve_and_call
    br x0
.symver __bf_gethostbyname, gethostbyname@@LIBC

.balign 4
.globl __bf_strcat
.type __bf_strcat, %function
__bf_strcat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2680]
    cbz x17, 1f
    br x17
1:  mov x0, #335
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcat, strcat@@LIBC

.balign 4
.globl __bf_sendmmsg
.type __bf_sendmmsg, %function
__bf_sendmmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2688]
    cbz x17, 1f
    br x17
1:  mov x0, #336
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendmmsg, sendmmsg@@LIBC

.balign 4
.globl __bf_tolower
.type __bf_tolower, %function
__bf_tolower:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2696]
    cbz x17, 1f
    br x17
1:  mov x0, #337
    bl __bf_resolve_and_call
    br x0
.symver __bf_tolower, tolower@@LIBC

.balign 4
.globl __bf_pthread_rwlock_destroy
.type __bf_pthread_rwlock_destroy, %function
__bf_pthread_rwlock_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2704]
    cbz x17, 1f
    br x17
1:  mov x0, #338
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_destroy, pthread_rwlock_destroy@@LIBC

.balign 4
.globl __bf_pthread_rwlock_init
.type __bf_pthread_rwlock_init, %function
__bf_pthread_rwlock_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2712]
    cbz x17, 1f
    br x17
1:  mov x0, #339
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_init, pthread_rwlock_init@@LIBC

.balign 4
.globl __bf_pthread_rwlock_rdlock
.type __bf_pthread_rwlock_rdlock, %function
__bf_pthread_rwlock_rdlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2720]
    cbz x17, 1f
    br x17
1:  mov x0, #340
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_rdlock, pthread_rwlock_rdlock@@LIBC

.balign 4
.globl __bf_pthread_rwlock_unlock
.type __bf_pthread_rwlock_unlock, %function
__bf_pthread_rwlock_unlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2728]
    cbz x17, 1f
    br x17
1:  mov x0, #341
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_unlock, pthread_rwlock_unlock@@LIBC

.balign 4
.globl __bf_pthread_rwlock_wrlock
.type __bf_pthread_rwlock_wrlock, %function
__bf_pthread_rwlock_wrlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2736]
    cbz x17, 1f
    br x17
1:  mov x0, #342
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_wrlock, pthread_rwlock_wrlock@@LIBC

.balign 4
.globl __bf_signal
.type __bf_signal, %function
__bf_signal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2744]
    cbz x17, 1f
    br x17
1:  mov x0, #343
    bl __bf_resolve_and_call
    br x0
.symver __bf_signal, signal@@LIBC

.balign 4
.globl __bf_tcgetattr
.type __bf_tcgetattr, %function
__bf_tcgetattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2752]
    cbz x17, 1f
    br x17
1:  mov x0, #344
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcgetattr, tcgetattr@@LIBC

.balign 4
.globl __bf_tcsetattr
.type __bf_tcsetattr, %function
__bf_tcsetattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2760]
    cbz x17, 1f
    br x17
1:  mov x0, #345
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcsetattr, tcsetattr@@LIBC

.balign 4
.globl __bf_utime
.type __bf_utime, %function
__bf_utime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2768]
    cbz x17, 1f
    br x17
1:  mov x0, #346
    bl __bf_resolve_and_call
    br x0
.symver __bf_utime, utime@@LIBC

.balign 4
.globl __bf_vasprintf
.type __bf_vasprintf, %function
__bf_vasprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2776]
    cbz x17, 1f
    br x17
1:  mov x0, #347
    bl __bf_resolve_and_call
    br x0
.symver __bf_vasprintf, vasprintf@@LIBC

.balign 4
.globl __bf_openlog
.type __bf_openlog, %function
__bf_openlog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2784]
    cbz x17, 1f
    br x17
1:  mov x0, #348
    bl __bf_resolve_and_call
    br x0
.symver __bf_openlog, openlog@@LIBC

.balign 4
.globl __bf_syslog
.type __bf_syslog, %function
__bf_syslog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2792]
    cbz x17, 1f
    br x17
1:  mov x0, #349
    bl __bf_resolve_and_call
    br x0
.symver __bf_syslog, syslog@@LIBC

.balign 4
.globl __bf_closelog
.type __bf_closelog, %function
__bf_closelog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2800]
    cbz x17, 1f
    br x17
1:  mov x0, #350
    bl __bf_resolve_and_call
    br x0
.symver __bf_closelog, closelog@@LIBC

.balign 4
.globl __bf_ungetc
.type __bf_ungetc, %function
__bf_ungetc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2808]
    cbz x17, 1f
    br x17
1:  mov x0, #351
    bl __bf_resolve_and_call
    br x0
.symver __bf_ungetc, ungetc@@LIBC

.balign 4
.globl __bf_getc
.type __bf_getc, %function
__bf_getc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2816]
    cbz x17, 1f
    br x17
1:  mov x0, #352
    bl __bf_resolve_and_call
    br x0
.symver __bf_getc, getc@@LIBC

.balign 4
.globl __bf_ungetwc
.type __bf_ungetwc, %function
__bf_ungetwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2824]
    cbz x17, 1f
    br x17
1:  mov x0, #353
    bl __bf_resolve_and_call
    br x0
.symver __bf_ungetwc, ungetwc@@LIBC

.balign 4
.globl __bf_getwc
.type __bf_getwc, %function
__bf_getwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2832]
    cbz x17, 1f
    br x17
1:  mov x0, #354
    bl __bf_resolve_and_call
    br x0
.symver __bf_getwc, getwc@@LIBC

.balign 4
.globl __bf_fputwc
.type __bf_fputwc, %function
__bf_fputwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2840]
    cbz x17, 1f
    br x17
1:  mov x0, #355
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputwc, fputwc@@LIBC

.balign 4
.globl __bf_newlocale
.type __bf_newlocale, %function
__bf_newlocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2848]
    cbz x17, 1f
    br x17
1:  mov x0, #356
    bl __bf_resolve_and_call
    br x0
.symver __bf_newlocale, newlocale@@LIBC

.balign 4
.globl __bf_uselocale
.type __bf_uselocale, %function
__bf_uselocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2856]
    cbz x17, 1f
    br x17
1:  mov x0, #357
    bl __bf_resolve_and_call
    br x0
.symver __bf_uselocale, uselocale@@LIBC

.balign 4
.globl __bf_vsscanf
.type __bf_vsscanf, %function
__bf_vsscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2864]
    cbz x17, 1f
    br x17
1:  mov x0, #358
    bl __bf_resolve_and_call
    br x0
.symver __bf_vsscanf, vsscanf@@LIBC

.balign 4
.globl __bf_strftime_l
.type __bf_strftime_l, %function
__bf_strftime_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2872]
    cbz x17, 1f
    br x17
1:  mov x0, #359
    bl __bf_resolve_and_call
    br x0
.symver __bf_strftime_l, strftime_l@@LIBC

.balign 4
.globl __bf_mbsrtowcs
.type __bf_mbsrtowcs, %function
__bf_mbsrtowcs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2880]
    cbz x17, 1f
    br x17
1:  mov x0, #360
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbsrtowcs, mbsrtowcs@@LIBC

.balign 4
.globl __bf_freelocale
.type __bf_freelocale, %function
__bf_freelocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2888]
    cbz x17, 1f
    br x17
1:  mov x0, #361
    bl __bf_resolve_and_call
    br x0
.symver __bf_freelocale, freelocale@@LIBC

.balign 4
.globl __bf_strcoll_l
.type __bf_strcoll_l, %function
__bf_strcoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2896]
    cbz x17, 1f
    br x17
1:  mov x0, #362
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcoll_l, strcoll_l@@LIBC

.balign 4
.globl __bf_strxfrm_l
.type __bf_strxfrm_l, %function
__bf_strxfrm_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2904]
    cbz x17, 1f
    br x17
1:  mov x0, #363
    bl __bf_resolve_and_call
    br x0
.symver __bf_strxfrm_l, strxfrm_l@@LIBC

.balign 4
.globl __bf_wcscoll_l
.type __bf_wcscoll_l, %function
__bf_wcscoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2912]
    cbz x17, 1f
    br x17
1:  mov x0, #364
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcscoll_l, wcscoll_l@@LIBC

.balign 4
.globl __bf_wcsxfrm_l
.type __bf_wcsxfrm_l, %function
__bf_wcsxfrm_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2920]
    cbz x17, 1f
    br x17
1:  mov x0, #365
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcsxfrm_l, wcsxfrm_l@@LIBC

.balign 4
.globl __bf_iswlower_l
.type __bf_iswlower_l, %function
__bf_iswlower_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2928]
    cbz x17, 1f
    br x17
1:  mov x0, #366
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswlower_l, iswlower_l@@LIBC

.balign 4
.globl __bf_iswspace_l
.type __bf_iswspace_l, %function
__bf_iswspace_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2936]
    cbz x17, 1f
    br x17
1:  mov x0, #367
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswspace_l, iswspace_l@@LIBC

.balign 4
.globl __bf_iswprint_l
.type __bf_iswprint_l, %function
__bf_iswprint_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2944]
    cbz x17, 1f
    br x17
1:  mov x0, #368
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswprint_l, iswprint_l@@LIBC

.balign 4
.globl __bf_iswblank_l
.type __bf_iswblank_l, %function
__bf_iswblank_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2952]
    cbz x17, 1f
    br x17
1:  mov x0, #369
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswblank_l, iswblank_l@@LIBC

.balign 4
.globl __bf_iswcntrl_l
.type __bf_iswcntrl_l, %function
__bf_iswcntrl_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2960]
    cbz x17, 1f
    br x17
1:  mov x0, #370
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswcntrl_l, iswcntrl_l@@LIBC

.balign 4
.globl __bf_iswupper_l
.type __bf_iswupper_l, %function
__bf_iswupper_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2968]
    cbz x17, 1f
    br x17
1:  mov x0, #371
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswupper_l, iswupper_l@@LIBC

.balign 4
.globl __bf_iswalpha_l
.type __bf_iswalpha_l, %function
__bf_iswalpha_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2976]
    cbz x17, 1f
    br x17
1:  mov x0, #372
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswalpha_l, iswalpha_l@@LIBC

.balign 4
.globl __bf_iswdigit_l
.type __bf_iswdigit_l, %function
__bf_iswdigit_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2984]
    cbz x17, 1f
    br x17
1:  mov x0, #373
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswdigit_l, iswdigit_l@@LIBC

.balign 4
.globl __bf_iswpunct_l
.type __bf_iswpunct_l, %function
__bf_iswpunct_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2992]
    cbz x17, 1f
    br x17
1:  mov x0, #374
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswpunct_l, iswpunct_l@@LIBC

.balign 4
.globl __bf_iswxdigit_l
.type __bf_iswxdigit_l, %function
__bf_iswxdigit_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3000]
    cbz x17, 1f
    br x17
1:  mov x0, #375
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswxdigit_l, iswxdigit_l@@LIBC

.balign 4
.globl __bf_towupper_l
.type __bf_towupper_l, %function
__bf_towupper_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3008]
    cbz x17, 1f
    br x17
1:  mov x0, #376
    bl __bf_resolve_and_call
    br x0
.symver __bf_towupper_l, towupper_l@@LIBC

.balign 4
.globl __bf_towlower_l
.type __bf_towlower_l, %function
__bf_towlower_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3016]
    cbz x17, 1f
    br x17
1:  mov x0, #377
    bl __bf_resolve_and_call
    br x0
.symver __bf_towlower_l, towlower_l@@LIBC

.balign 4
.globl __bf_btowc
.type __bf_btowc, %function
__bf_btowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3024]
    cbz x17, 1f
    br x17
1:  mov x0, #378
    bl __bf_resolve_and_call
    br x0
.symver __bf_btowc, btowc@@LIBC

.balign 4
.globl __bf_wctob
.type __bf_wctob, %function
__bf_wctob:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3032]
    cbz x17, 1f
    br x17
1:  mov x0, #379
    bl __bf_resolve_and_call
    br x0
.symver __bf_wctob, wctob@@LIBC

.balign 4
.globl __bf_wcsnrtombs
.type __bf_wcsnrtombs, %function
__bf_wcsnrtombs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3040]
    cbz x17, 1f
    br x17
1:  mov x0, #380
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcsnrtombs, wcsnrtombs@@LIBC

.balign 4
.globl __bf_wcrtomb
.type __bf_wcrtomb, %function
__bf_wcrtomb:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3048]
    cbz x17, 1f
    br x17
1:  mov x0, #381
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcrtomb, wcrtomb@@LIBC

.balign 4
.globl __bf_mbsnrtowcs
.type __bf_mbsnrtowcs, %function
__bf_mbsnrtowcs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3056]
    cbz x17, 1f
    br x17
1:  mov x0, #382
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbsnrtowcs, mbsnrtowcs@@LIBC

.balign 4
.globl __bf_mbrtowc
.type __bf_mbrtowc, %function
__bf_mbrtowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3064]
    cbz x17, 1f
    br x17
1:  mov x0, #383
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbrtowc, mbrtowc@@LIBC

.balign 4
.globl __bf_mbtowc
.type __bf_mbtowc, %function
__bf_mbtowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3072]
    cbz x17, 1f
    br x17
1:  mov x0, #384
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbtowc, mbtowc@@LIBC

.balign 4
.globl __bf___ctype_get_mb_cur_max
.type __bf___ctype_get_mb_cur_max, %function
__bf___ctype_get_mb_cur_max:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3080]
    cbz x17, 1f
    br x17
1:  mov x0, #385
    bl __bf_resolve_and_call
    br x0
.symver __bf___ctype_get_mb_cur_max, __ctype_get_mb_cur_max@@LIBC

.balign 4
.globl __bf_mbrlen
.type __bf_mbrlen, %function
__bf_mbrlen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3088]
    cbz x17, 1f
    br x17
1:  mov x0, #386
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbrlen, mbrlen@@LIBC

.balign 4
.globl __bf_strtoll_l
.type __bf_strtoll_l, %function
__bf_strtoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3096]
    cbz x17, 1f
    br x17
1:  mov x0, #387
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoll_l, strtoll_l@@LIBC

.balign 4
.globl __bf_strtoull_l
.type __bf_strtoull_l, %function
__bf_strtoull_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3104]
    cbz x17, 1f
    br x17
1:  mov x0, #388
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoull_l, strtoull_l@@LIBC

.balign 4
.globl __bf_strtold_l
.type __bf_strtold_l, %function
__bf_strtold_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3112]
    cbz x17, 1f
    br x17
1:  mov x0, #389
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtold_l, strtold_l@@LIBC

.balign 4
.globl __bf___cxa_thread_atexit_impl
.type __bf___cxa_thread_atexit_impl, %function
__bf___cxa_thread_atexit_impl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3120]
    cbz x17, 1f
    br x17
1:  mov x0, #390
    bl __bf_resolve_and_call
    br x0
.symver __bf___cxa_thread_atexit_impl, __cxa_thread_atexit_impl@@LIBC

.balign 4
.globl __bf_strncat
.type __bf_strncat, %function
__bf_strncat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3128]
    cbz x17, 1f
    br x17
1:  mov x0, #391
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncat, strncat@@LIBC

.section .data.bionic_data, "aw"

.balign 8
.globl __bf_data_stderr
.type __bf_data_stderr, %object
.size __bf_data_stderr, 8
__bf_data_stderr:
    .dword 0
.symver __bf_data_stderr, stderr@@LIBC

.balign 8
.globl __bf_data___sF
.type __bf_data___sF, %object
.size __bf_data___sF, 8
__bf_data___sF:
    .dword 0
.symver __bf_data___sF, __sF@@LIBC

.balign 8
.globl __bf_data_optarg
.type __bf_data_optarg, %object
.size __bf_data_optarg, 8
__bf_data_optarg:
    .dword 0
.symver __bf_data_optarg, optarg@@LIBC

.balign 8
.globl __bf_data_optind
.type __bf_data_optind, %object
.size __bf_data_optind, 8
__bf_data_optind:
    .dword 0
.symver __bf_data_optind, optind@@LIBC

.balign 8
.globl __bf_data_tzname
.type __bf_data_tzname, %object
.size __bf_data_tzname, 8
__bf_data_tzname:
    .dword 0
.symver __bf_data_tzname, tzname@@LIBC

.balign 8
.globl __bf_data_daylight
.type __bf_data_daylight, %object
.size __bf_data_daylight, 8
__bf_data_daylight:
    .dword 0
.symver __bf_data_daylight, daylight@@LIBC

.balign 8
.globl __bf_data_timezone
.type __bf_data_timezone, %object
.size __bf_data_timezone, 8
__bf_data_timezone:
    .dword 0
.symver __bf_data_timezone, timezone@@LIBC

.balign 8
.globl __bf_data_environ
.type __bf_data_environ, %object
.size __bf_data_environ, 8
__bf_data_environ:
    .dword 0
.symver __bf_data_environ, environ@@LIBC

.balign 8
.globl __bf_data_in6addr_any
.type __bf_data_in6addr_any, %object
.size __bf_data_in6addr_any, 8
__bf_data_in6addr_any:
    .dword 0
.symver __bf_data_in6addr_any, in6addr_any@@LIBC_N

.balign 8
.globl __bf_data_stdin
.type __bf_data_stdin, %object
.size __bf_data_stdin, 8
__bf_data_stdin:
    .dword 0
.symver __bf_data_stdin, stdin@@LIBC

.balign 8
.globl __bf_data_stdout
.type __bf_data_stdout, %object
.size __bf_data_stdout, 8
__bf_data_stdout:
    .dword 0
.symver __bf_data_stdout, stdout@@LIBC

.balign 8
.globl __bf_data_in6addr_loopback
.type __bf_data_in6addr_loopback, %object
.size __bf_data_in6addr_loopback, 8
__bf_data_in6addr_loopback:
    .dword 0
.symver __bf_data_in6addr_loopback, in6addr_loopback@@LIBC_N

.balign 8
.globl __bf_data___stack_chk_guard
.type __bf_data___stack_chk_guard, %object
.size __bf_data___stack_chk_guard, 8
__bf_data___stack_chk_guard:
    .dword 0
.symver __bf_data___stack_chk_guard, __stack_chk_guard@@LIBC
