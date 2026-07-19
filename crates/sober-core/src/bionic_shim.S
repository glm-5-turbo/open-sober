/* Auto-generated Bionic symbol shim with lazy dispatch. DO NOT EDIT.
 * 785 trampolines, 14 data objs.
 */

.section .data.bionic_tramp, "aw"
.balign 8
.globl __bf_tramp_table
.type __bf_tramp_table, %object
.size __bf_tramp_table, 6280
__bf_tramp_table:
.zero 6280

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
.globl __bf_strncmp
.type __bf_strncmp, %function
__bf_strncmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #96]
    cbz x17, 1f
    br x17
1:  mov x0, #12
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncmp, strncmp@@LIBC

.balign 4
.globl __bf_strcmp
.type __bf_strcmp, %function
__bf_strcmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #104]
    cbz x17, 1f
    br x17
1:  mov x0, #13
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcmp, strcmp@@LIBC

.balign 4
.globl __bf_getauxval
.type __bf_getauxval, %function
__bf_getauxval:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #112]
    cbz x17, 1f
    br x17
1:  mov x0, #14
    bl __bf_resolve_and_call
    br x0
.symver __bf_getauxval, getauxval@@LIBC

.balign 4
.globl __bf___errno
.type __bf___errno, %function
__bf___errno:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #120]
    cbz x17, 1f
    br x17
1:  mov x0, #15
    bl __bf_resolve_and_call
    br x0
.symver __bf___errno, __errno@@LIBC

.balign 4
.globl __bf_close
.type __bf_close, %function
__bf_close:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #128]
    cbz x17, 1f
    br x17
1:  mov x0, #16
    bl __bf_resolve_and_call
    br x0
.symver __bf_close, close@@LIBC

.balign 4
.globl __bf___open_2
.type __bf___open_2, %function
__bf___open_2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #136]
    cbz x17, 1f
    br x17
1:  mov x0, #17
    bl __bf_resolve_and_call
    br x0
.symver __bf___open_2, __open_2@@LIBC

.balign 4
.globl __bf___read_chk
.type __bf___read_chk, %function
__bf___read_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #144]
    cbz x17, 1f
    br x17
1:  mov x0, #18
    bl __bf_resolve_and_call
    br x0
.symver __bf___read_chk, __read_chk@@LIBC

.balign 4
.globl __bf_read
.type __bf_read, %function
__bf_read:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #152]
    cbz x17, 1f
    br x17
1:  mov x0, #19
    bl __bf_resolve_and_call
    br x0
.symver __bf_read, read@@LIBC

.balign 4
.globl __bf_clock_gettime
.type __bf_clock_gettime, %function
__bf_clock_gettime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #160]
    cbz x17, 1f
    br x17
1:  mov x0, #20
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_gettime, clock_gettime@@LIBC

.balign 4
.globl __bf_syscall
.type __bf_syscall, %function
__bf_syscall:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #168]
    cbz x17, 1f
    br x17
1:  mov x0, #21
    bl __bf_resolve_and_call
    br x0
.symver __bf_syscall, syscall@@LIBC

.balign 4
.globl __bf_sched_getcpu
.type __bf_sched_getcpu, %function
__bf_sched_getcpu:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #176]
    cbz x17, 1f
    br x17
1:  mov x0, #22
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getcpu, sched_getcpu@@LIBC

.balign 4
.globl __bf_sysconf
.type __bf_sysconf, %function
__bf_sysconf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #184]
    cbz x17, 1f
    br x17
1:  mov x0, #23
    bl __bf_resolve_and_call
    br x0
.symver __bf_sysconf, sysconf@@LIBC

.balign 4
.globl __bf_mmap
.type __bf_mmap, %function
__bf_mmap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #192]
    cbz x17, 1f
    br x17
1:  mov x0, #24
    bl __bf_resolve_and_call
    br x0
.symver __bf_mmap, mmap@@LIBC

.balign 4
.globl __bf_mprotect
.type __bf_mprotect, %function
__bf_mprotect:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #200]
    cbz x17, 1f
    br x17
1:  mov x0, #25
    bl __bf_resolve_and_call
    br x0
.symver __bf_mprotect, mprotect@@LIBC

.balign 4
.globl __bf_munmap
.type __bf_munmap, %function
__bf_munmap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #208]
    cbz x17, 1f
    br x17
1:  mov x0, #26
    bl __bf_resolve_and_call
    br x0
.symver __bf_munmap, munmap@@LIBC

.balign 4
.globl __bf_pthread_attr_init
.type __bf_pthread_attr_init, %function
__bf_pthread_attr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #216]
    cbz x17, 1f
    br x17
1:  mov x0, #27
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_init, pthread_attr_init@@LIBC

.balign 4
.globl __bf_pthread_attr_setstacksize
.type __bf_pthread_attr_setstacksize, %function
__bf_pthread_attr_setstacksize:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #224]
    cbz x17, 1f
    br x17
1:  mov x0, #28
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setstacksize, pthread_attr_setstacksize@@LIBC

.balign 4
.globl __bf_pthread_create
.type __bf_pthread_create, %function
__bf_pthread_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #232]
    cbz x17, 1f
    br x17
1:  mov x0, #29
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_create, pthread_create@@LIBC

.balign 4
.globl __bf_pthread_attr_destroy
.type __bf_pthread_attr_destroy, %function
__bf_pthread_attr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #240]
    cbz x17, 1f
    br x17
1:  mov x0, #30
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_destroy, pthread_attr_destroy@@LIBC

.balign 4
.globl __bf_pthread_join
.type __bf_pthread_join, %function
__bf_pthread_join:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #248]
    cbz x17, 1f
    br x17
1:  mov x0, #31
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_join, pthread_join@@LIBC

.balign 4
.globl __bf_pthread_self
.type __bf_pthread_self, %function
__bf_pthread_self:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #256]
    cbz x17, 1f
    br x17
1:  mov x0, #32
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_self, pthread_self@@LIBC

.balign 4
.globl __bf_memset
.type __bf_memset, %function
__bf_memset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #264]
    cbz x17, 1f
    br x17
1:  mov x0, #33
    bl __bf_resolve_and_call
    br x0
.symver __bf_memset, memset@@LIBC

.balign 4
.globl __bf_pthread_setspecific
.type __bf_pthread_setspecific, %function
__bf_pthread_setspecific:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #272]
    cbz x17, 1f
    br x17
1:  mov x0, #34
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setspecific, pthread_setspecific@@LIBC

.balign 4
.globl __bf___strncpy_chk
.type __bf___strncpy_chk, %function
__bf___strncpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #280]
    cbz x17, 1f
    br x17
1:  mov x0, #35
    bl __bf_resolve_and_call
    br x0
.symver __bf___strncpy_chk, __strncpy_chk@@LIBC

.balign 4
.globl __bf_sched_get_priority_max
.type __bf_sched_get_priority_max, %function
__bf_sched_get_priority_max:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #288]
    cbz x17, 1f
    br x17
1:  mov x0, #36
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_get_priority_max, sched_get_priority_max@@LIBC

.balign 4
.globl __bf_sched_setscheduler
.type __bf_sched_setscheduler, %function
__bf_sched_setscheduler:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #296]
    cbz x17, 1f
    br x17
1:  mov x0, #37
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_setscheduler, sched_setscheduler@@LIBC

.balign 4
.globl __bf_pthread_mutex_lock
.type __bf_pthread_mutex_lock, %function
__bf_pthread_mutex_lock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #304]
    cbz x17, 1f
    br x17
1:  mov x0, #38
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_lock, pthread_mutex_lock@@LIBC

.balign 4
.globl __bf_pthread_cond_wait
.type __bf_pthread_cond_wait, %function
__bf_pthread_cond_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #312]
    cbz x17, 1f
    br x17
1:  mov x0, #39
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_wait, pthread_cond_wait@@LIBC

.balign 4
.globl __bf_pthread_mutex_unlock
.type __bf_pthread_mutex_unlock, %function
__bf_pthread_mutex_unlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #320]
    cbz x17, 1f
    br x17
1:  mov x0, #40
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_unlock, pthread_mutex_unlock@@LIBC

.balign 4
.globl __bf_pthread_cond_signal
.type __bf_pthread_cond_signal, %function
__bf_pthread_cond_signal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #328]
    cbz x17, 1f
    br x17
1:  mov x0, #41
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_signal, pthread_cond_signal@@LIBC

.balign 4
.globl __bf_pthread_cond_init
.type __bf_pthread_cond_init, %function
__bf_pthread_cond_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #336]
    cbz x17, 1f
    br x17
1:  mov x0, #42
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_init, pthread_cond_init@@LIBC

.balign 4
.globl __bf_gettid
.type __bf_gettid, %function
__bf_gettid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #344]
    cbz x17, 1f
    br x17
1:  mov x0, #43
    bl __bf_resolve_and_call
    br x0
.symver __bf_gettid, gettid@@LIBC

.balign 4
.globl __bf___vsprintf_chk
.type __bf___vsprintf_chk, %function
__bf___vsprintf_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #352]
    cbz x17, 1f
    br x17
1:  mov x0, #44
    bl __bf_resolve_and_call
    br x0
.symver __bf___vsprintf_chk, __vsprintf_chk@@LIBC

.balign 4
.globl __bf_atan2f
.type __bf_atan2f, %function
__bf_atan2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #360]
    cbz x17, 1f
    br x17
1:  mov x0, #45
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan2f, atan2f@@LIBC

.balign 4
.globl __bf_vsnprintf
.type __bf_vsnprintf, %function
__bf_vsnprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #368]
    cbz x17, 1f
    br x17
1:  mov x0, #46
    bl __bf_resolve_and_call
    br x0
.symver __bf_vsnprintf, vsnprintf@@LIBC

.balign 4
.globl __bf_fprintf
.type __bf_fprintf, %function
__bf_fprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #376]
    cbz x17, 1f
    br x17
1:  mov x0, #47
    bl __bf_resolve_and_call
    br x0
.symver __bf_fprintf, fprintf@@LIBC

.balign 4
.globl __bf_rand
.type __bf_rand, %function
__bf_rand:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #384]
    cbz x17, 1f
    br x17
1:  mov x0, #48
    bl __bf_resolve_and_call
    br x0
.symver __bf_rand, rand@@LIBC

.balign 4
.globl __bf_strtoll
.type __bf_strtoll, %function
__bf_strtoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #392]
    cbz x17, 1f
    br x17
1:  mov x0, #49
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoll, strtoll@@LIBC

.balign 4
.globl __bf_time
.type __bf_time, %function
__bf_time:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #400]
    cbz x17, 1f
    br x17
1:  mov x0, #50
    bl __bf_resolve_and_call
    br x0
.symver __bf_time, time@@LIBC

.balign 4
.globl __bf_pthread_cond_broadcast
.type __bf_pthread_cond_broadcast, %function
__bf_pthread_cond_broadcast:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #408]
    cbz x17, 1f
    br x17
1:  mov x0, #51
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_broadcast, pthread_cond_broadcast@@LIBC

.balign 4
.globl __bf_pthread_cond_destroy
.type __bf_pthread_cond_destroy, %function
__bf_pthread_cond_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #416]
    cbz x17, 1f
    br x17
1:  mov x0, #52
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_destroy, pthread_cond_destroy@@LIBC

.balign 4
.globl __bf_stat
.type __bf_stat, %function
__bf_stat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #424]
    cbz x17, 1f
    br x17
1:  mov x0, #53
    bl __bf_resolve_and_call
    br x0
.symver __bf_stat, stat@@LIBC

.balign 4
.globl __bf_opendir
.type __bf_opendir, %function
__bf_opendir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #432]
    cbz x17, 1f
    br x17
1:  mov x0, #54
    bl __bf_resolve_and_call
    br x0
.symver __bf_opendir, opendir@@LIBC

.balign 4
.globl __bf_readdir
.type __bf_readdir, %function
__bf_readdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #440]
    cbz x17, 1f
    br x17
1:  mov x0, #55
    bl __bf_resolve_and_call
    br x0
.symver __bf_readdir, readdir@@LIBC

.balign 4
.globl __bf_closedir
.type __bf_closedir, %function
__bf_closedir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #448]
    cbz x17, 1f
    br x17
1:  mov x0, #56
    bl __bf_resolve_and_call
    br x0
.symver __bf_closedir, closedir@@LIBC

.balign 4
.globl __bf_posix_fallocate
.type __bf_posix_fallocate, %function
__bf_posix_fallocate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #456]
    cbz x17, 1f
    br x17
1:  mov x0, #57
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_fallocate, posix_fallocate@@LIBC

.balign 4
.globl __bf_open
.type __bf_open, %function
__bf_open:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #464]
    cbz x17, 1f
    br x17
1:  mov x0, #58
    bl __bf_resolve_and_call
    br x0
.symver __bf_open, open@@LIBC

.balign 4
.globl __bf_pthread_equal
.type __bf_pthread_equal, %function
__bf_pthread_equal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #472]
    cbz x17, 1f
    br x17
1:  mov x0, #59
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_equal, pthread_equal@@LIBC

.balign 4
.globl __bf_atoll
.type __bf_atoll, %function
__bf_atoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #480]
    cbz x17, 1f
    br x17
1:  mov x0, #60
    bl __bf_resolve_and_call
    br x0
.symver __bf_atoll, atoll@@LIBC

.balign 4
.globl __bf_srand
.type __bf_srand, %function
__bf_srand:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #488]
    cbz x17, 1f
    br x17
1:  mov x0, #61
    bl __bf_resolve_and_call
    br x0
.symver __bf_srand, srand@@LIBC

.balign 4
.globl __bf_strtod
.type __bf_strtod, %function
__bf_strtod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #496]
    cbz x17, 1f
    br x17
1:  mov x0, #62
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtod, strtod@@LIBC

.balign 4
.globl __bf_localtime
.type __bf_localtime, %function
__bf_localtime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #504]
    cbz x17, 1f
    br x17
1:  mov x0, #63
    bl __bf_resolve_and_call
    br x0
.symver __bf_localtime, localtime@@LIBC

.balign 4
.globl __bf_fseek
.type __bf_fseek, %function
__bf_fseek:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #512]
    cbz x17, 1f
    br x17
1:  mov x0, #64
    bl __bf_resolve_and_call
    br x0
.symver __bf_fseek, fseek@@LIBC

.balign 4
.globl __bf_ftell
.type __bf_ftell, %function
__bf_ftell:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #520]
    cbz x17, 1f
    br x17
1:  mov x0, #65
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftell, ftell@@LIBC

.balign 4
.globl __bf_fclose
.type __bf_fclose, %function
__bf_fclose:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #528]
    cbz x17, 1f
    br x17
1:  mov x0, #66
    bl __bf_resolve_and_call
    br x0
.symver __bf_fclose, fclose@@LIBC

.balign 4
.globl __bf_atan
.type __bf_atan, %function
__bf_atan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #536]
    cbz x17, 1f
    br x17
1:  mov x0, #67
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan, atan@@LIBC

.balign 4
.globl __bf_mkdir
.type __bf_mkdir, %function
__bf_mkdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #544]
    cbz x17, 1f
    br x17
1:  mov x0, #68
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkdir, mkdir@@LIBC

.balign 4
.globl __bf___fread_chk
.type __bf___fread_chk, %function
__bf___fread_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #552]
    cbz x17, 1f
    br x17
1:  mov x0, #69
    bl __bf_resolve_and_call
    br x0
.symver __bf___fread_chk, __fread_chk@@LIBC_N

.balign 4
.globl __bf_fread
.type __bf_fread, %function
__bf_fread:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #560]
    cbz x17, 1f
    br x17
1:  mov x0, #70
    bl __bf_resolve_and_call
    br x0
.symver __bf_fread, fread@@LIBC

.balign 4
.globl __bf_asinf
.type __bf_asinf, %function
__bf_asinf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #568]
    cbz x17, 1f
    br x17
1:  mov x0, #71
    bl __bf_resolve_and_call
    br x0
.symver __bf_asinf, asinf@@LIBC

.balign 4
.globl __bf_access
.type __bf_access, %function
__bf_access:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #576]
    cbz x17, 1f
    br x17
1:  mov x0, #72
    bl __bf_resolve_and_call
    br x0
.symver __bf_access, access@@LIBC

.balign 4
.globl __bf_getenv
.type __bf_getenv, %function
__bf_getenv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #584]
    cbz x17, 1f
    br x17
1:  mov x0, #73
    bl __bf_resolve_and_call
    br x0
.symver __bf_getenv, getenv@@LIBC

.balign 4
.globl __bf_strncpy
.type __bf_strncpy, %function
__bf_strncpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #592]
    cbz x17, 1f
    br x17
1:  mov x0, #74
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncpy, strncpy@@LIBC

.balign 4
.globl __bf_feof
.type __bf_feof, %function
__bf_feof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #600]
    cbz x17, 1f
    br x17
1:  mov x0, #75
    bl __bf_resolve_and_call
    br x0
.symver __bf_feof, feof@@LIBC

.balign 4
.globl __bf_fopen
.type __bf_fopen, %function
__bf_fopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #608]
    cbz x17, 1f
    br x17
1:  mov x0, #76
    bl __bf_resolve_and_call
    br x0
.symver __bf_fopen, fopen@@LIBC

.balign 4
.globl __bf_fseeko
.type __bf_fseeko, %function
__bf_fseeko:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #616]
    cbz x17, 1f
    br x17
1:  mov x0, #77
    bl __bf_resolve_and_call
    br x0
.symver __bf_fseeko, fseeko@@LIBC

.balign 4
.globl __bf_ftello
.type __bf_ftello, %function
__bf_ftello:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #624]
    cbz x17, 1f
    br x17
1:  mov x0, #78
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftello, ftello@@LIBC

.balign 4
.globl __bf_fwrite
.type __bf_fwrite, %function
__bf_fwrite:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #632]
    cbz x17, 1f
    br x17
1:  mov x0, #79
    bl __bf_resolve_and_call
    br x0
.symver __bf_fwrite, fwrite@@LIBC

.balign 4
.globl __bf_fflush
.type __bf_fflush, %function
__bf_fflush:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #640]
    cbz x17, 1f
    br x17
1:  mov x0, #80
    bl __bf_resolve_and_call
    br x0
.symver __bf_fflush, fflush@@LIBC

.balign 4
.globl __bf_gmtime
.type __bf_gmtime, %function
__bf_gmtime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #648]
    cbz x17, 1f
    br x17
1:  mov x0, #81
    bl __bf_resolve_and_call
    br x0
.symver __bf_gmtime, gmtime@@LIBC

.balign 4
.globl __bf_mktime
.type __bf_mktime, %function
__bf_mktime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #656]
    cbz x17, 1f
    br x17
1:  mov x0, #82
    bl __bf_resolve_and_call
    br x0
.symver __bf_mktime, mktime@@LIBC

.balign 4
.globl __bf_pipe
.type __bf_pipe, %function
__bf_pipe:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #664]
    cbz x17, 1f
    br x17
1:  mov x0, #83
    bl __bf_resolve_and_call
    br x0
.symver __bf_pipe, pipe@@LIBC

.balign 4
.globl __bf_write
.type __bf_write, %function
__bf_write:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #672]
    cbz x17, 1f
    br x17
1:  mov x0, #84
    bl __bf_resolve_and_call
    br x0
.symver __bf_write, write@@LIBC

.balign 4
.globl __bf_pthread_key_create
.type __bf_pthread_key_create, %function
__bf_pthread_key_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #680]
    cbz x17, 1f
    br x17
1:  mov x0, #85
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_key_create, pthread_key_create@@LIBC

.balign 4
.globl __bf_abort
.type __bf_abort, %function
__bf_abort:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #688]
    cbz x17, 1f
    br x17
1:  mov x0, #86
    bl __bf_resolve_and_call
    br x0
.symver __bf_abort, abort@@LIBC

.balign 4
.globl __bf___assert
.type __bf___assert, %function
__bf___assert:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #696]
    cbz x17, 1f
    br x17
1:  mov x0, #87
    bl __bf_resolve_and_call
    br x0
.symver __bf___assert, __assert@@LIBC

.balign 4
.globl __bf_strrchr
.type __bf_strrchr, %function
__bf_strrchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #704]
    cbz x17, 1f
    br x17
1:  mov x0, #88
    bl __bf_resolve_and_call
    br x0
.symver __bf_strrchr, strrchr@@LIBC

.balign 4
.globl __bf___stack_chk_fail
.type __bf___stack_chk_fail, %function
__bf___stack_chk_fail:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #712]
    cbz x17, 1f
    br x17
1:  mov x0, #89
    bl __bf_resolve_and_call
    br x0
.symver __bf___stack_chk_fail, __stack_chk_fail@@LIBC

.balign 4
.globl __bf_atoi
.type __bf_atoi, %function
__bf_atoi:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #720]
    cbz x17, 1f
    br x17
1:  mov x0, #90
    bl __bf_resolve_and_call
    br x0
.symver __bf_atoi, atoi@@LIBC

.balign 4
.globl __bf_fcntl
.type __bf_fcntl, %function
__bf_fcntl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #728]
    cbz x17, 1f
    br x17
1:  mov x0, #91
    bl __bf_resolve_and_call
    br x0
.symver __bf_fcntl, fcntl@@LIBC

.balign 4
.globl __bf_memcpy
.type __bf_memcpy, %function
__bf_memcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #736]
    cbz x17, 1f
    br x17
1:  mov x0, #92
    bl __bf_resolve_and_call
    br x0
.symver __bf_memcpy, memcpy@@LIBC

.balign 4
.globl __bf_strcpy
.type __bf_strcpy, %function
__bf_strcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #744]
    cbz x17, 1f
    br x17
1:  mov x0, #93
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcpy, strcpy@@LIBC

.balign 4
.globl __bf_strerror
.type __bf_strerror, %function
__bf_strerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #752]
    cbz x17, 1f
    br x17
1:  mov x0, #94
    bl __bf_resolve_and_call
    br x0
.symver __bf_strerror, strerror@@LIBC

.balign 4
.globl __bf_pthread_attr_setdetachstate
.type __bf_pthread_attr_setdetachstate, %function
__bf_pthread_attr_setdetachstate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #760]
    cbz x17, 1f
    br x17
1:  mov x0, #95
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setdetachstate, pthread_attr_setdetachstate@@LIBC

.balign 4
.globl __bf_pthread_cond_timedwait
.type __bf_pthread_cond_timedwait, %function
__bf_pthread_cond_timedwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #768]
    cbz x17, 1f
    br x17
1:  mov x0, #96
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_timedwait, pthread_cond_timedwait@@LIBC

.balign 4
.globl __bf_memmove
.type __bf_memmove, %function
__bf_memmove:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #776]
    cbz x17, 1f
    br x17
1:  mov x0, #97
    bl __bf_resolve_and_call
    br x0
.symver __bf_memmove, memmove@@LIBC

.balign 4
.globl __bf_strtol
.type __bf_strtol, %function
__bf_strtol:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #784]
    cbz x17, 1f
    br x17
1:  mov x0, #98
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtol, strtol@@LIBC

.balign 4
.globl __bf_getpid
.type __bf_getpid, %function
__bf_getpid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #792]
    cbz x17, 1f
    br x17
1:  mov x0, #99
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpid, getpid@@LIBC

.balign 4
.globl __bf_gettimeofday
.type __bf_gettimeofday, %function
__bf_gettimeofday:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #800]
    cbz x17, 1f
    br x17
1:  mov x0, #100
    bl __bf_resolve_and_call
    br x0
.symver __bf_gettimeofday, gettimeofday@@LIBC

.balign 4
.globl __bf_localtime_r
.type __bf_localtime_r, %function
__bf_localtime_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #808]
    cbz x17, 1f
    br x17
1:  mov x0, #101
    bl __bf_resolve_and_call
    br x0
.symver __bf_localtime_r, localtime_r@@LIBC

.balign 4
.globl __bf_fputs
.type __bf_fputs, %function
__bf_fputs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #816]
    cbz x17, 1f
    br x17
1:  mov x0, #102
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputs, fputs@@LIBC

.balign 4
.globl __bf_strerror_r
.type __bf_strerror_r, %function
__bf_strerror_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #824]
    cbz x17, 1f
    br x17
1:  mov x0, #103
    bl __bf_resolve_and_call
    br x0
.symver __bf_strerror_r, strerror_r@@LIBC

.balign 4
.globl __bf_snprintf
.type __bf_snprintf, %function
__bf_snprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #832]
    cbz x17, 1f
    br x17
1:  mov x0, #104
    bl __bf_resolve_and_call
    br x0
.symver __bf_snprintf, snprintf@@LIBC

.balign 4
.globl __bf_prctl
.type __bf_prctl, %function
__bf_prctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #840]
    cbz x17, 1f
    br x17
1:  mov x0, #105
    bl __bf_resolve_and_call
    br x0
.symver __bf_prctl, prctl@@LIBC

.balign 4
.globl __bf_sigaltstack
.type __bf_sigaltstack, %function
__bf_sigaltstack:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #848]
    cbz x17, 1f
    br x17
1:  mov x0, #106
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaltstack, sigaltstack@@LIBC

.balign 4
.globl __bf_getpagesize
.type __bf_getpagesize, %function
__bf_getpagesize:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #856]
    cbz x17, 1f
    br x17
1:  mov x0, #107
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpagesize, getpagesize@@LIBC

.balign 4
.globl __bf_pthread_getspecific
.type __bf_pthread_getspecific, %function
__bf_pthread_getspecific:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #864]
    cbz x17, 1f
    br x17
1:  mov x0, #108
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getspecific, pthread_getspecific@@LIBC

.balign 4
.globl __bf_fork
.type __bf_fork, %function
__bf_fork:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #872]
    cbz x17, 1f
    br x17
1:  mov x0, #109
    bl __bf_resolve_and_call
    br x0
.symver __bf_fork, fork@@LIBC

.balign 4
.globl __bf_waitpid
.type __bf_waitpid, %function
__bf_waitpid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #880]
    cbz x17, 1f
    br x17
1:  mov x0, #110
    bl __bf_resolve_and_call
    br x0
.symver __bf_waitpid, waitpid@@LIBC

.balign 4
.globl __bf_execv
.type __bf_execv, %function
__bf_execv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #888]
    cbz x17, 1f
    br x17
1:  mov x0, #111
    bl __bf_resolve_and_call
    br x0
.symver __bf_execv, execv@@LIBC

.balign 4
.globl __bf__exit
.type __bf__exit, %function
__bf__exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #896]
    cbz x17, 1f
    br x17
1:  mov x0, #112
    bl __bf_resolve_and_call
    br x0
.symver __bf__exit, _exit@@LIBC

.balign 4
.globl __bf_execve
.type __bf_execve, %function
__bf_execve:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #904]
    cbz x17, 1f
    br x17
1:  mov x0, #113
    bl __bf_resolve_and_call
    br x0
.symver __bf_execve, execve@@LIBC

.balign 4
.globl __bf_getopt_long
.type __bf_getopt_long, %function
__bf_getopt_long:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #912]
    cbz x17, 1f
    br x17
1:  mov x0, #114
    bl __bf_resolve_and_call
    br x0
.symver __bf_getopt_long, getopt_long@@LIBC

.balign 4
.globl __bf_getppid
.type __bf_getppid, %function
__bf_getppid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #920]
    cbz x17, 1f
    br x17
1:  mov x0, #115
    bl __bf_resolve_and_call
    br x0
.symver __bf_getppid, getppid@@LIBC

.balign 4
.globl __bf_geteuid
.type __bf_geteuid, %function
__bf_geteuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #928]
    cbz x17, 1f
    br x17
1:  mov x0, #116
    bl __bf_resolve_and_call
    br x0
.symver __bf_geteuid, geteuid@@LIBC

.balign 4
.globl __bf_epoll_create1
.type __bf_epoll_create1, %function
__bf_epoll_create1:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #936]
    cbz x17, 1f
    br x17
1:  mov x0, #117
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_create1, epoll_create1@@LIBC

.balign 4
.globl __bf_eventfd
.type __bf_eventfd, %function
__bf_eventfd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #944]
    cbz x17, 1f
    br x17
1:  mov x0, #118
    bl __bf_resolve_and_call
    br x0
.symver __bf_eventfd, eventfd@@LIBC

.balign 4
.globl __bf_epoll_ctl
.type __bf_epoll_ctl, %function
__bf_epoll_ctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #952]
    cbz x17, 1f
    br x17
1:  mov x0, #119
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_ctl, epoll_ctl@@LIBC

.balign 4
.globl __bf_getsockopt
.type __bf_getsockopt, %function
__bf_getsockopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #960]
    cbz x17, 1f
    br x17
1:  mov x0, #120
    bl __bf_resolve_and_call
    br x0
.symver __bf_getsockopt, getsockopt@@LIBC

.balign 4
.globl __bf_setsockopt
.type __bf_setsockopt, %function
__bf_setsockopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #968]
    cbz x17, 1f
    br x17
1:  mov x0, #121
    bl __bf_resolve_and_call
    br x0
.symver __bf_setsockopt, setsockopt@@LIBC

.balign 4
.globl __bf_epoll_wait
.type __bf_epoll_wait, %function
__bf_epoll_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #976]
    cbz x17, 1f
    br x17
1:  mov x0, #122
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_wait, epoll_wait@@LIBC

.balign 4
.globl __bf_getuid
.type __bf_getuid, %function
__bf_getuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #984]
    cbz x17, 1f
    br x17
1:  mov x0, #123
    bl __bf_resolve_and_call
    br x0
.symver __bf_getuid, getuid@@LIBC

.balign 4
.globl __bf_pthread_mutex_trylock
.type __bf_pthread_mutex_trylock, %function
__bf_pthread_mutex_trylock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #992]
    cbz x17, 1f
    br x17
1:  mov x0, #124
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_trylock, pthread_mutex_trylock@@LIBC

.balign 4
.globl __bf_ldexp
.type __bf_ldexp, %function
__bf_ldexp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1000]
    cbz x17, 1f
    br x17
1:  mov x0, #125
    bl __bf_resolve_and_call
    br x0
.symver __bf_ldexp, ldexp@@LIBC

.balign 4
.globl __bf_strchr
.type __bf_strchr, %function
__bf_strchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1008]
    cbz x17, 1f
    br x17
1:  mov x0, #126
    bl __bf_resolve_and_call
    br x0
.symver __bf_strchr, strchr@@LIBC

.balign 4
.globl __bf_getaddrinfo
.type __bf_getaddrinfo, %function
__bf_getaddrinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1016]
    cbz x17, 1f
    br x17
1:  mov x0, #127
    bl __bf_resolve_and_call
    br x0
.symver __bf_getaddrinfo, getaddrinfo@@LIBC

.balign 4
.globl __bf_socket
.type __bf_socket, %function
__bf_socket:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1024]
    cbz x17, 1f
    br x17
1:  mov x0, #128
    bl __bf_resolve_and_call
    br x0
.symver __bf_socket, socket@@LIBC

.balign 4
.globl __bf_freeaddrinfo
.type __bf_freeaddrinfo, %function
__bf_freeaddrinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1032]
    cbz x17, 1f
    br x17
1:  mov x0, #129
    bl __bf_resolve_and_call
    br x0
.symver __bf_freeaddrinfo, freeaddrinfo@@LIBC

.balign 4
.globl __bf_connect
.type __bf_connect, %function
__bf_connect:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1040]
    cbz x17, 1f
    br x17
1:  mov x0, #130
    bl __bf_resolve_and_call
    br x0
.symver __bf_connect, connect@@LIBC

.balign 4
.globl __bf_poll
.type __bf_poll, %function
__bf_poll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1048]
    cbz x17, 1f
    br x17
1:  mov x0, #131
    bl __bf_resolve_and_call
    br x0
.symver __bf_poll, poll@@LIBC

.balign 4
.globl __bf_sched_getscheduler
.type __bf_sched_getscheduler, %function
__bf_sched_getscheduler:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1056]
    cbz x17, 1f
    br x17
1:  mov x0, #132
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getscheduler, sched_getscheduler@@LIBC

.balign 4
.globl __bf_sched_getparam
.type __bf_sched_getparam, %function
__bf_sched_getparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1064]
    cbz x17, 1f
    br x17
1:  mov x0, #133
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getparam, sched_getparam@@LIBC

.balign 4
.globl __bf_getpriority
.type __bf_getpriority, %function
__bf_getpriority:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1072]
    cbz x17, 1f
    br x17
1:  mov x0, #134
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpriority, getpriority@@LIBC

.balign 4
.globl __bf_uname
.type __bf_uname, %function
__bf_uname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1080]
    cbz x17, 1f
    br x17
1:  mov x0, #135
    bl __bf_resolve_and_call
    br x0
.symver __bf_uname, uname@@LIBC

.balign 4
.globl __bf_tzset
.type __bf_tzset, %function
__bf_tzset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1088]
    cbz x17, 1f
    br x17
1:  mov x0, #136
    bl __bf_resolve_and_call
    br x0
.symver __bf_tzset, tzset@@LIBC

.balign 4
.globl __bf_strnlen
.type __bf_strnlen, %function
__bf_strnlen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1096]
    cbz x17, 1f
    br x17
1:  mov x0, #137
    bl __bf_resolve_and_call
    br x0
.symver __bf_strnlen, strnlen@@LIBC

.balign 4
.globl __bf_writev
.type __bf_writev, %function
__bf_writev:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1104]
    cbz x17, 1f
    br x17
1:  mov x0, #138
    bl __bf_resolve_and_call
    br x0
.symver __bf_writev, writev@@LIBC

.balign 4
.globl __bf_sscanf
.type __bf_sscanf, %function
__bf_sscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1112]
    cbz x17, 1f
    br x17
1:  mov x0, #139
    bl __bf_resolve_and_call
    br x0
.symver __bf_sscanf, sscanf@@LIBC

.balign 4
.globl __bf_strtoul
.type __bf_strtoul, %function
__bf_strtoul:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1120]
    cbz x17, 1f
    br x17
1:  mov x0, #140
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoul, strtoul@@LIBC

.balign 4
.globl __bf_strtoull
.type __bf_strtoull, %function
__bf_strtoull:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1128]
    cbz x17, 1f
    br x17
1:  mov x0, #141
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoull, strtoull@@LIBC

.balign 4
.globl __bf_lseek
.type __bf_lseek, %function
__bf_lseek:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1136]
    cbz x17, 1f
    br x17
1:  mov x0, #142
    bl __bf_resolve_and_call
    br x0
.symver __bf_lseek, lseek@@LIBC

.balign 4
.globl __bf_ftruncate
.type __bf_ftruncate, %function
__bf_ftruncate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1144]
    cbz x17, 1f
    br x17
1:  mov x0, #143
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftruncate, ftruncate@@LIBC

.balign 4
.globl __bf_fstat
.type __bf_fstat, %function
__bf_fstat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1152]
    cbz x17, 1f
    br x17
1:  mov x0, #144
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstat, fstat@@LIBC

.balign 4
.globl __bf_lstat
.type __bf_lstat, %function
__bf_lstat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1160]
    cbz x17, 1f
    br x17
1:  mov x0, #145
    bl __bf_resolve_and_call
    br x0
.symver __bf_lstat, lstat@@LIBC

.balign 4
.globl __bf_rename
.type __bf_rename, %function
__bf_rename:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1168]
    cbz x17, 1f
    br x17
1:  mov x0, #146
    bl __bf_resolve_and_call
    br x0
.symver __bf_rename, rename@@LIBC

.balign 4
.globl __bf_unlink
.type __bf_unlink, %function
__bf_unlink:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1176]
    cbz x17, 1f
    br x17
1:  mov x0, #147
    bl __bf_resolve_and_call
    br x0
.symver __bf_unlink, unlink@@LIBC

.balign 4
.globl __bf_rmdir
.type __bf_rmdir, %function
__bf_rmdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1184]
    cbz x17, 1f
    br x17
1:  mov x0, #148
    bl __bf_resolve_and_call
    br x0
.symver __bf_rmdir, rmdir@@LIBC

.balign 4
.globl __bf_nanosleep
.type __bf_nanosleep, %function
__bf_nanosleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1192]
    cbz x17, 1f
    br x17
1:  mov x0, #149
    bl __bf_resolve_and_call
    br x0
.symver __bf_nanosleep, nanosleep@@LIBC

.balign 4
.globl __bf_sigemptyset
.type __bf_sigemptyset, %function
__bf_sigemptyset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1200]
    cbz x17, 1f
    br x17
1:  mov x0, #150
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigemptyset, sigemptyset@@LIBC

.balign 4
.globl __bf_sigaction
.type __bf_sigaction, %function
__bf_sigaction:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1208]
    cbz x17, 1f
    br x17
1:  mov x0, #151
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaction, sigaction@@LIBC

.balign 4
.globl __bf_raise
.type __bf_raise, %function
__bf_raise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1216]
    cbz x17, 1f
    br x17
1:  mov x0, #152
    bl __bf_resolve_and_call
    br x0
.symver __bf_raise, raise@@LIBC

.balign 4
.globl __bf_fscanf
.type __bf_fscanf, %function
__bf_fscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1224]
    cbz x17, 1f
    br x17
1:  mov x0, #153
    bl __bf_resolve_and_call
    br x0
.symver __bf_fscanf, fscanf@@LIBC

.balign 4
.globl __bf_pread64
.type __bf_pread64, %function
__bf_pread64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1232]
    cbz x17, 1f
    br x17
1:  mov x0, #154
    bl __bf_resolve_and_call
    br x0
.symver __bf_pread64, pread64@@LIBC

.balign 4
.globl __bf_ptrace
.type __bf_ptrace, %function
__bf_ptrace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1240]
    cbz x17, 1f
    br x17
1:  mov x0, #155
    bl __bf_resolve_and_call
    br x0
.symver __bf_ptrace, ptrace@@LIBC

.balign 4
.globl __bf_socketpair
.type __bf_socketpair, %function
__bf_socketpair:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1248]
    cbz x17, 1f
    br x17
1:  mov x0, #156
    bl __bf_resolve_and_call
    br x0
.symver __bf_socketpair, socketpair@@LIBC

.balign 4
.globl __bf_sendmsg
.type __bf_sendmsg, %function
__bf_sendmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1256]
    cbz x17, 1f
    br x17
1:  mov x0, #157
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendmsg, sendmsg@@LIBC

.balign 4
.globl __bf_recvmsg
.type __bf_recvmsg, %function
__bf_recvmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1264]
    cbz x17, 1f
    br x17
1:  mov x0, #158
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvmsg, recvmsg@@LIBC

.balign 4
.globl __bf___cmsg_nxthdr
.type __bf___cmsg_nxthdr, %function
__bf___cmsg_nxthdr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1272]
    cbz x17, 1f
    br x17
1:  mov x0, #159
    bl __bf_resolve_and_call
    br x0
.symver __bf___cmsg_nxthdr, __cmsg_nxthdr@@LIBC

.balign 4
.globl __bf_readlink
.type __bf_readlink, %function
__bf_readlink:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1280]
    cbz x17, 1f
    br x17
1:  mov x0, #160
    bl __bf_resolve_and_call
    br x0
.symver __bf_readlink, readlink@@LIBC

.balign 4
.globl __bf_printf
.type __bf_printf, %function
__bf_printf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1288]
    cbz x17, 1f
    br x17
1:  mov x0, #161
    bl __bf_resolve_and_call
    br x0
.symver __bf_printf, printf@@LIBC

.balign 4
.globl __bf_wmemchr
.type __bf_wmemchr, %function
__bf_wmemchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1296]
    cbz x17, 1f
    br x17
1:  mov x0, #162
    bl __bf_resolve_and_call
    br x0
.symver __bf_wmemchr, wmemchr@@LIBC

.balign 4
.globl __bf_localeconv
.type __bf_localeconv, %function
__bf_localeconv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1304]
    cbz x17, 1f
    br x17
1:  mov x0, #163
    bl __bf_resolve_and_call
    br x0
.symver __bf_localeconv, localeconv@@LIBC

.balign 4
.globl __bf___vsnprintf_chk
.type __bf___vsnprintf_chk, %function
__bf___vsnprintf_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1312]
    cbz x17, 1f
    br x17
1:  mov x0, #164
    bl __bf_resolve_and_call
    br x0
.symver __bf___vsnprintf_chk, __vsnprintf_chk@@LIBC

.balign 4
.globl __bf___memmove_chk
.type __bf___memmove_chk, %function
__bf___memmove_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1320]
    cbz x17, 1f
    br x17
1:  mov x0, #165
    bl __bf_resolve_and_call
    br x0
.symver __bf___memmove_chk, __memmove_chk@@LIBC

.balign 4
.globl __bf_sched_yield
.type __bf_sched_yield, %function
__bf_sched_yield:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1328]
    cbz x17, 1f
    br x17
1:  mov x0, #166
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_yield, sched_yield@@LIBC

.balign 4
.globl __bf_modf
.type __bf_modf, %function
__bf_modf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1336]
    cbz x17, 1f
    br x17
1:  mov x0, #167
    bl __bf_resolve_and_call
    br x0
.symver __bf_modf, modf@@LIBC

.balign 4
.globl __bf_strcasecmp
.type __bf_strcasecmp, %function
__bf_strcasecmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1344]
    cbz x17, 1f
    br x17
1:  mov x0, #168
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcasecmp, strcasecmp@@LIBC

.balign 4
.globl __bf_getnameinfo
.type __bf_getnameinfo, %function
__bf_getnameinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1352]
    cbz x17, 1f
    br x17
1:  mov x0, #169
    bl __bf_resolve_and_call
    br x0
.symver __bf_getnameinfo, getnameinfo@@LIBC

.balign 4
.globl __bf_strftime
.type __bf_strftime, %function
__bf_strftime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1360]
    cbz x17, 1f
    br x17
1:  mov x0, #170
    bl __bf_resolve_and_call
    br x0
.symver __bf_strftime, strftime@@LIBC

.balign 4
.globl __bf___strcpy_chk
.type __bf___strcpy_chk, %function
__bf___strcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1368]
    cbz x17, 1f
    br x17
1:  mov x0, #171
    bl __bf_resolve_and_call
    br x0
.symver __bf___strcpy_chk, __strcpy_chk@@LIBC

.balign 4
.globl __bf_frexpf
.type __bf_frexpf, %function
__bf_frexpf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1376]
    cbz x17, 1f
    br x17
1:  mov x0, #172
    bl __bf_resolve_and_call
    br x0
.symver __bf_frexpf, frexpf@@LIBC

.balign 4
.globl __bf_ldexpf
.type __bf_ldexpf, %function
__bf_ldexpf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1384]
    cbz x17, 1f
    br x17
1:  mov x0, #173
    bl __bf_resolve_and_call
    br x0
.symver __bf_ldexpf, ldexpf@@LIBC

.balign 4
.globl __bf_tanf
.type __bf_tanf, %function
__bf_tanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1392]
    cbz x17, 1f
    br x17
1:  mov x0, #174
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanf, tanf@@LIBC

.balign 4
.globl __bf_atanf
.type __bf_atanf, %function
__bf_atanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1400]
    cbz x17, 1f
    br x17
1:  mov x0, #175
    bl __bf_resolve_and_call
    br x0
.symver __bf_atanf, atanf@@LIBC

.balign 4
.globl __bf_erff
.type __bf_erff, %function
__bf_erff:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1408]
    cbz x17, 1f
    br x17
1:  mov x0, #176
    bl __bf_resolve_and_call
    br x0
.symver __bf_erff, erff@@LIBC

.balign 4
.globl __bf_acosf
.type __bf_acosf, %function
__bf_acosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1416]
    cbz x17, 1f
    br x17
1:  mov x0, #177
    bl __bf_resolve_and_call
    br x0
.symver __bf_acosf, acosf@@LIBC

.balign 4
.globl __bf_strstr
.type __bf_strstr, %function
__bf_strstr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1424]
    cbz x17, 1f
    br x17
1:  mov x0, #178
    bl __bf_resolve_and_call
    br x0
.symver __bf_strstr, strstr@@LIBC

.balign 4
.globl __bf_erfcf
.type __bf_erfcf, %function
__bf_erfcf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1432]
    cbz x17, 1f
    br x17
1:  mov x0, #179
    bl __bf_resolve_and_call
    br x0
.symver __bf_erfcf, erfcf@@LIBC

.balign 4
.globl __bf_modff
.type __bf_modff, %function
__bf_modff:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1440]
    cbz x17, 1f
    br x17
1:  mov x0, #180
    bl __bf_resolve_and_call
    br x0
.symver __bf_modff, modff@@LIBC

.balign 4
.globl __bf_coshf
.type __bf_coshf, %function
__bf_coshf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1448]
    cbz x17, 1f
    br x17
1:  mov x0, #181
    bl __bf_resolve_and_call
    br x0
.symver __bf_coshf, coshf@@LIBC

.balign 4
.globl __bf_sinhf
.type __bf_sinhf, %function
__bf_sinhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1456]
    cbz x17, 1f
    br x17
1:  mov x0, #182
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinhf, sinhf@@LIBC

.balign 4
.globl __bf_tanhf
.type __bf_tanhf, %function
__bf_tanhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1464]
    cbz x17, 1f
    br x17
1:  mov x0, #183
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanhf, tanhf@@LIBC

.balign 4
.globl __bf_atan2
.type __bf_atan2, %function
__bf_atan2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1472]
    cbz x17, 1f
    br x17
1:  mov x0, #184
    bl __bf_resolve_and_call
    br x0
.symver __bf_atan2, atan2@@LIBC

.balign 4
.globl __bf_cbrtf
.type __bf_cbrtf, %function
__bf_cbrtf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1480]
    cbz x17, 1f
    br x17
1:  mov x0, #185
    bl __bf_resolve_and_call
    br x0
.symver __bf_cbrtf, cbrtf@@LIBC

.balign 4
.globl __bf___strchr_chk
.type __bf___strchr_chk, %function
__bf___strchr_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1488]
    cbz x17, 1f
    br x17
1:  mov x0, #186
    bl __bf_resolve_and_call
    br x0
.symver __bf___strchr_chk, __strchr_chk@@LIBC

.balign 4
.globl __bf_clock
.type __bf_clock, %function
__bf_clock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1496]
    cbz x17, 1f
    br x17
1:  mov x0, #187
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock, clock@@LIBC

.balign 4
.globl __bf_fileno
.type __bf_fileno, %function
__bf_fileno:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1504]
    cbz x17, 1f
    br x17
1:  mov x0, #188
    bl __bf_resolve_and_call
    br x0
.symver __bf_fileno, fileno@@LIBC

.balign 4
.globl __bf_remainderf
.type __bf_remainderf, %function
__bf_remainderf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1512]
    cbz x17, 1f
    br x17
1:  mov x0, #189
    bl __bf_resolve_and_call
    br x0
.symver __bf_remainderf, remainderf@@LIBC

.balign 4
.globl __bf_nan
.type __bf_nan, %function
__bf_nan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1520]
    cbz x17, 1f
    br x17
1:  mov x0, #190
    bl __bf_resolve_and_call
    br x0
.symver __bf_nan, nan@@LIBC

.balign 4
.globl __bf_qsort
.type __bf_qsort, %function
__bf_qsort:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1528]
    cbz x17, 1f
    br x17
1:  mov x0, #191
    bl __bf_resolve_and_call
    br x0
.symver __bf_qsort, qsort@@LIBC

.balign 4
.globl __bf_nextafterf
.type __bf_nextafterf, %function
__bf_nextafterf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1536]
    cbz x17, 1f
    br x17
1:  mov x0, #192
    bl __bf_resolve_and_call
    br x0
.symver __bf_nextafterf, nextafterf@@LIBC

.balign 4
.globl __bf_acos
.type __bf_acos, %function
__bf_acos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1544]
    cbz x17, 1f
    br x17
1:  mov x0, #193
    bl __bf_resolve_and_call
    br x0
.symver __bf_acos, acos@@LIBC

.balign 4
.globl __bf_asin
.type __bf_asin, %function
__bf_asin:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1552]
    cbz x17, 1f
    br x17
1:  mov x0, #194
    bl __bf_resolve_and_call
    br x0
.symver __bf_asin, asin@@LIBC

.balign 4
.globl __bf_ilogb
.type __bf_ilogb, %function
__bf_ilogb:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1560]
    cbz x17, 1f
    br x17
1:  mov x0, #195
    bl __bf_resolve_and_call
    br x0
.symver __bf_ilogb, ilogb@@LIBC

.balign 4
.globl __bf___FD_SET_chk
.type __bf___FD_SET_chk, %function
__bf___FD_SET_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1568]
    cbz x17, 1f
    br x17
1:  mov x0, #196
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_SET_chk, __FD_SET_chk@@LIBC

.balign 4
.globl __bf_select
.type __bf_select, %function
__bf_select:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1576]
    cbz x17, 1f
    br x17
1:  mov x0, #197
    bl __bf_resolve_and_call
    br x0
.symver __bf_select, select@@LIBC

.balign 4
.globl __bf___FD_ISSET_chk
.type __bf___FD_ISSET_chk, %function
__bf___FD_ISSET_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1584]
    cbz x17, 1f
    br x17
1:  mov x0, #198
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_ISSET_chk, __FD_ISSET_chk@@LIBC

.balign 4
.globl __bf_sendto
.type __bf_sendto, %function
__bf_sendto:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1592]
    cbz x17, 1f
    br x17
1:  mov x0, #199
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendto, sendto@@LIBC

.balign 4
.globl __bf_recvfrom
.type __bf_recvfrom, %function
__bf_recvfrom:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1600]
    cbz x17, 1f
    br x17
1:  mov x0, #200
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvfrom, recvfrom@@LIBC

.balign 4
.globl __bf___strcat_chk
.type __bf___strcat_chk, %function
__bf___strcat_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1608]
    cbz x17, 1f
    br x17
1:  mov x0, #201
    bl __bf_resolve_and_call
    br x0
.symver __bf___strcat_chk, __strcat_chk@@LIBC

.balign 4
.globl __bf_setpriority
.type __bf_setpriority, %function
__bf_setpriority:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1616]
    cbz x17, 1f
    br x17
1:  mov x0, #202
    bl __bf_resolve_and_call
    br x0
.symver __bf_setpriority, setpriority@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_init
.type __bf_pthread_mutexattr_init, %function
__bf_pthread_mutexattr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1624]
    cbz x17, 1f
    br x17
1:  mov x0, #203
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_init, pthread_mutexattr_init@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_settype
.type __bf_pthread_mutexattr_settype, %function
__bf_pthread_mutexattr_settype:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1632]
    cbz x17, 1f
    br x17
1:  mov x0, #204
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_settype, pthread_mutexattr_settype@@LIBC

.balign 4
.globl __bf_pthread_mutexattr_destroy
.type __bf_pthread_mutexattr_destroy, %function
__bf_pthread_mutexattr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1640]
    cbz x17, 1f
    br x17
1:  mov x0, #205
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutexattr_destroy, pthread_mutexattr_destroy@@LIBC

.balign 4
.globl __bf_sem_init
.type __bf_sem_init, %function
__bf_sem_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1648]
    cbz x17, 1f
    br x17
1:  mov x0, #206
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_init, sem_init@@LIBC

.balign 4
.globl __bf_sem_destroy
.type __bf_sem_destroy, %function
__bf_sem_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1656]
    cbz x17, 1f
    br x17
1:  mov x0, #207
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_destroy, sem_destroy@@LIBC

.balign 4
.globl __bf_sem_wait
.type __bf_sem_wait, %function
__bf_sem_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1664]
    cbz x17, 1f
    br x17
1:  mov x0, #208
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_wait, sem_wait@@LIBC

.balign 4
.globl __bf_sem_post
.type __bf_sem_post, %function
__bf_sem_post:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1672]
    cbz x17, 1f
    br x17
1:  mov x0, #209
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_post, sem_post@@LIBC

.balign 4
.globl __bf_inet_ntop
.type __bf_inet_ntop, %function
__bf_inet_ntop:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1680]
    cbz x17, 1f
    br x17
1:  mov x0, #210
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_ntop, inet_ntop@@LIBC

.balign 4
.globl __bf_inet_pton
.type __bf_inet_pton, %function
__bf_inet_pton:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1688]
    cbz x17, 1f
    br x17
1:  mov x0, #211
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_pton, inet_pton@@LIBC

.balign 4
.globl __bf_strncasecmp
.type __bf_strncasecmp, %function
__bf_strncasecmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1696]
    cbz x17, 1f
    br x17
1:  mov x0, #212
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncasecmp, strncasecmp@@LIBC

.balign 4
.globl __bf_pthread_attr_setschedparam
.type __bf_pthread_attr_setschedparam, %function
__bf_pthread_attr_setschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1704]
    cbz x17, 1f
    br x17
1:  mov x0, #213
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_setschedparam, pthread_attr_setschedparam@@LIBC

.balign 4
.globl __bf_bind
.type __bf_bind, %function
__bf_bind:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1712]
    cbz x17, 1f
    br x17
1:  mov x0, #214
    bl __bf_resolve_and_call
    br x0
.symver __bf_bind, bind@@LIBC

.balign 4
.globl __bf_getsockname
.type __bf_getsockname, %function
__bf_getsockname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1720]
    cbz x17, 1f
    br x17
1:  mov x0, #215
    bl __bf_resolve_and_call
    br x0
.symver __bf_getsockname, getsockname@@LIBC

.balign 4
.globl __bf_gethostname
.type __bf_gethostname, %function
__bf_gethostname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1728]
    cbz x17, 1f
    br x17
1:  mov x0, #216
    bl __bf_resolve_and_call
    br x0
.symver __bf_gethostname, gethostname@@LIBC

.balign 4
.globl __bf___sendto_chk
.type __bf___sendto_chk, %function
__bf___sendto_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1736]
    cbz x17, 1f
    br x17
1:  mov x0, #217
    bl __bf_resolve_and_call
    br x0
.symver __bf___sendto_chk, __sendto_chk@@LIBC_O

.balign 4
.globl __bf_puts
.type __bf_puts, %function
__bf_puts:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1744]
    cbz x17, 1f
    br x17
1:  mov x0, #218
    bl __bf_resolve_and_call
    br x0
.symver __bf_puts, puts@@LIBC

.balign 4
.globl __bf_gai_strerror
.type __bf_gai_strerror, %function
__bf_gai_strerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1752]
    cbz x17, 1f
    br x17
1:  mov x0, #219
    bl __bf_resolve_and_call
    br x0
.symver __bf_gai_strerror, gai_strerror@@LIBC

.balign 4
.globl __bf___write_chk
.type __bf___write_chk, %function
__bf___write_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1760]
    cbz x17, 1f
    br x17
1:  mov x0, #220
    bl __bf_resolve_and_call
    br x0
.symver __bf___write_chk, __write_chk@@LIBC_N

.balign 4
.globl __bf___poll_chk
.type __bf___poll_chk, %function
__bf___poll_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1768]
    cbz x17, 1f
    br x17
1:  mov x0, #221
    bl __bf_resolve_and_call
    br x0
.symver __bf___poll_chk, __poll_chk@@LIBC

.balign 4
.globl __bf_vprintf
.type __bf_vprintf, %function
__bf_vprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1776]
    cbz x17, 1f
    br x17
1:  mov x0, #222
    bl __bf_resolve_and_call
    br x0
.symver __bf_vprintf, vprintf@@LIBC

.balign 4
.globl __bf_usleep
.type __bf_usleep, %function
__bf_usleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1784]
    cbz x17, 1f
    br x17
1:  mov x0, #223
    bl __bf_resolve_and_call
    br x0
.symver __bf_usleep, usleep@@LIBC

.balign 4
.globl __bf_pthread_kill
.type __bf_pthread_kill, %function
__bf_pthread_kill:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1792]
    cbz x17, 1f
    br x17
1:  mov x0, #224
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_kill, pthread_kill@@LIBC

.balign 4
.globl __bf_pthread_detach
.type __bf_pthread_detach, %function
__bf_pthread_detach:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1800]
    cbz x17, 1f
    br x17
1:  mov x0, #225
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_detach, pthread_detach@@LIBC

.balign 4
.globl __bf_exit
.type __bf_exit, %function
__bf_exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1808]
    cbz x17, 1f
    br x17
1:  mov x0, #226
    bl __bf_resolve_and_call
    br x0
.symver __bf_exit, exit@@LIBC

.balign 4
.globl __bf_ferror
.type __bf_ferror, %function
__bf_ferror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1816]
    cbz x17, 1f
    br x17
1:  mov x0, #227
    bl __bf_resolve_and_call
    br x0
.symver __bf_ferror, ferror@@LIBC

.balign 4
.globl __bf_clearerr
.type __bf_clearerr, %function
__bf_clearerr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1824]
    cbz x17, 1f
    br x17
1:  mov x0, #228
    bl __bf_resolve_and_call
    br x0
.symver __bf_clearerr, clearerr@@LIBC

.balign 4
.globl __bf_wcslen
.type __bf_wcslen, %function
__bf_wcslen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1832]
    cbz x17, 1f
    br x17
1:  mov x0, #229
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcslen, wcslen@@LIBC

.balign 4
.globl __bf_wmemcmp
.type __bf_wmemcmp, %function
__bf_wmemcmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1840]
    cbz x17, 1f
    br x17
1:  mov x0, #230
    bl __bf_resolve_and_call
    br x0
.symver __bf_wmemcmp, wmemcmp@@LIBC

.balign 4
.globl __bf_exp
.type __bf_exp, %function
__bf_exp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1848]
    cbz x17, 1f
    br x17
1:  mov x0, #231
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp, exp@@LIBC

.balign 4
.globl __bf_pow
.type __bf_pow, %function
__bf_pow:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1856]
    cbz x17, 1f
    br x17
1:  mov x0, #232
    bl __bf_resolve_and_call
    br x0
.symver __bf_pow, pow@@LIBC

.balign 4
.globl __bf_fmod
.type __bf_fmod, %function
__bf_fmod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1864]
    cbz x17, 1f
    br x17
1:  mov x0, #233
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmod, fmod@@LIBC

.balign 4
.globl __bf_log
.type __bf_log, %function
__bf_log:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1872]
    cbz x17, 1f
    br x17
1:  mov x0, #234
    bl __bf_resolve_and_call
    br x0
.symver __bf_log, log@@LIBC

.balign 4
.globl __bf_log2
.type __bf_log2, %function
__bf_log2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1880]
    cbz x17, 1f
    br x17
1:  mov x0, #235
    bl __bf_resolve_and_call
    br x0
.symver __bf_log2, log2@@LIBC

.balign 4
.globl __bf_log10
.type __bf_log10, %function
__bf_log10:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1888]
    cbz x17, 1f
    br x17
1:  mov x0, #236
    bl __bf_resolve_and_call
    br x0
.symver __bf_log10, log10@@LIBC

.balign 4
.globl __bf_round
.type __bf_round, %function
__bf_round:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1896]
    cbz x17, 1f
    br x17
1:  mov x0, #237
    bl __bf_resolve_and_call
    br x0
.symver __bf_round, round@@LIBC

.balign 4
.globl __bf_frexp
.type __bf_frexp, %function
__bf_frexp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1904]
    cbz x17, 1f
    br x17
1:  mov x0, #238
    bl __bf_resolve_and_call
    br x0
.symver __bf_frexp, frexp@@LIBC

.balign 4
.globl __bf_sin
.type __bf_sin, %function
__bf_sin:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1912]
    cbz x17, 1f
    br x17
1:  mov x0, #239
    bl __bf_resolve_and_call
    br x0
.symver __bf_sin, sin@@LIBC

.balign 4
.globl __bf_sinh
.type __bf_sinh, %function
__bf_sinh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1920]
    cbz x17, 1f
    br x17
1:  mov x0, #240
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinh, sinh@@LIBC

.balign 4
.globl __bf_cos
.type __bf_cos, %function
__bf_cos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1928]
    cbz x17, 1f
    br x17
1:  mov x0, #241
    bl __bf_resolve_and_call
    br x0
.symver __bf_cos, cos@@LIBC

.balign 4
.globl __bf_cosh
.type __bf_cosh, %function
__bf_cosh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1936]
    cbz x17, 1f
    br x17
1:  mov x0, #242
    bl __bf_resolve_and_call
    br x0
.symver __bf_cosh, cosh@@LIBC

.balign 4
.globl __bf_tan
.type __bf_tan, %function
__bf_tan:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1944]
    cbz x17, 1f
    br x17
1:  mov x0, #243
    bl __bf_resolve_and_call
    br x0
.symver __bf_tan, tan@@LIBC

.balign 4
.globl __bf_tanh
.type __bf_tanh, %function
__bf_tanh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1952]
    cbz x17, 1f
    br x17
1:  mov x0, #244
    bl __bf_resolve_and_call
    br x0
.symver __bf_tanh, tanh@@LIBC

.balign 4
.globl __bf_atol
.type __bf_atol, %function
__bf_atol:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1960]
    cbz x17, 1f
    br x17
1:  mov x0, #245
    bl __bf_resolve_and_call
    br x0
.symver __bf_atol, atol@@LIBC

.balign 4
.globl __bf_atof
.type __bf_atof, %function
__bf_atof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1968]
    cbz x17, 1f
    br x17
1:  mov x0, #246
    bl __bf_resolve_and_call
    br x0
.symver __bf_atof, atof@@LIBC

.balign 4
.globl __bf_strspn
.type __bf_strspn, %function
__bf_strspn:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1976]
    cbz x17, 1f
    br x17
1:  mov x0, #247
    bl __bf_resolve_and_call
    br x0
.symver __bf_strspn, strspn@@LIBC

.balign 4
.globl __bf_strtof
.type __bf_strtof, %function
__bf_strtof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1984]
    cbz x17, 1f
    br x17
1:  mov x0, #248
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtof, strtof@@LIBC

.balign 4
.globl __bf_ioctl
.type __bf_ioctl, %function
__bf_ioctl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #1992]
    cbz x17, 1f
    br x17
1:  mov x0, #249
    bl __bf_resolve_and_call
    br x0
.symver __bf_ioctl, ioctl@@LIBC

.balign 4
.globl __bf_getpeername
.type __bf_getpeername, %function
__bf_getpeername:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2000]
    cbz x17, 1f
    br x17
1:  mov x0, #250
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpeername, getpeername@@LIBC

.balign 4
.globl __bf_listen
.type __bf_listen, %function
__bf_listen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2008]
    cbz x17, 1f
    br x17
1:  mov x0, #251
    bl __bf_resolve_and_call
    br x0
.symver __bf_listen, listen@@LIBC

.balign 4
.globl __bf_accept
.type __bf_accept, %function
__bf_accept:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2016]
    cbz x17, 1f
    br x17
1:  mov x0, #252
    bl __bf_resolve_and_call
    br x0
.symver __bf_accept, accept@@LIBC

.balign 4
.globl __bf_epoll_create
.type __bf_epoll_create, %function
__bf_epoll_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2024]
    cbz x17, 1f
    br x17
1:  mov x0, #253
    bl __bf_resolve_and_call
    br x0
.symver __bf_epoll_create, epoll_create@@LIBC

.balign 4
.globl __bf___FD_CLR_chk
.type __bf___FD_CLR_chk, %function
__bf___FD_CLR_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2032]
    cbz x17, 1f
    br x17
1:  mov x0, #254
    bl __bf_resolve_and_call
    br x0
.symver __bf___FD_CLR_chk, __FD_CLR_chk@@LIBC

.balign 4
.globl __bf_expm1
.type __bf_expm1, %function
__bf_expm1:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2040]
    cbz x17, 1f
    br x17
1:  mov x0, #255
    bl __bf_resolve_and_call
    br x0
.symver __bf_expm1, expm1@@LIBC

.balign 4
.globl __bf_if_indextoname
.type __bf_if_indextoname, %function
__bf_if_indextoname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2048]
    cbz x17, 1f
    br x17
1:  mov x0, #256
    bl __bf_resolve_and_call
    br x0
.symver __bf_if_indextoname, if_indextoname@@LIBC

.balign 4
.globl __bf_sigaddset
.type __bf_sigaddset, %function
__bf_sigaddset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2056]
    cbz x17, 1f
    br x17
1:  mov x0, #257
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigaddset, sigaddset@@LIBC

.balign 4
.globl __bf_pthread_sigmask
.type __bf_pthread_sigmask, %function
__bf_pthread_sigmask:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2064]
    cbz x17, 1f
    br x17
1:  mov x0, #258
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_sigmask, pthread_sigmask@@LIBC

.balign 4
.globl __bf_fgets
.type __bf_fgets, %function
__bf_fgets:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2072]
    cbz x17, 1f
    br x17
1:  mov x0, #259
    bl __bf_resolve_and_call
    br x0
.symver __bf_fgets, fgets@@LIBC

.balign 4
.globl __bf_setjmp
.type __bf_setjmp, %function
__bf_setjmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2080]
    cbz x17, 1f
    br x17
1:  mov x0, #260
    bl __bf_resolve_and_call
    br x0
.symver __bf_setjmp, setjmp@@LIBC

.balign 4
.globl __bf_longjmp
.type __bf_longjmp, %function
__bf_longjmp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2088]
    cbz x17, 1f
    br x17
1:  mov x0, #261
    bl __bf_resolve_and_call
    br x0
.symver __bf_longjmp, longjmp@@LIBC

.balign 4
.globl __bf_pthread_condattr_init
.type __bf_pthread_condattr_init, %function
__bf_pthread_condattr_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2096]
    cbz x17, 1f
    br x17
1:  mov x0, #262
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_init, pthread_condattr_init@@LIBC

.balign 4
.globl __bf_pthread_condattr_setclock
.type __bf_pthread_condattr_setclock, %function
__bf_pthread_condattr_setclock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2104]
    cbz x17, 1f
    br x17
1:  mov x0, #263
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_setclock, pthread_condattr_setclock@@LIBC

.balign 4
.globl __bf_pthread_condattr_destroy
.type __bf_pthread_condattr_destroy, %function
__bf_pthread_condattr_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2112]
    cbz x17, 1f
    br x17
1:  mov x0, #264
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_condattr_destroy, pthread_condattr_destroy@@LIBC

.balign 4
.globl __bf_sched_get_priority_min
.type __bf_sched_get_priority_min, %function
__bf_sched_get_priority_min:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2120]
    cbz x17, 1f
    br x17
1:  mov x0, #265
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_get_priority_min, sched_get_priority_min@@LIBC

.balign 4
.globl __bf_pthread_setschedparam
.type __bf_pthread_setschedparam, %function
__bf_pthread_setschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2128]
    cbz x17, 1f
    br x17
1:  mov x0, #266
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setschedparam, pthread_setschedparam@@LIBC

.balign 4
.globl __bf_getgid
.type __bf_getgid, %function
__bf_getgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2136]
    cbz x17, 1f
    br x17
1:  mov x0, #267
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgid, getgid@@LIBC

.balign 4
.globl __bf_getegid
.type __bf_getegid, %function
__bf_getegid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2144]
    cbz x17, 1f
    br x17
1:  mov x0, #268
    bl __bf_resolve_and_call
    br x0
.symver __bf_getegid, getegid@@LIBC

.balign 4
.globl __bf_random
.type __bf_random, %function
__bf_random:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2152]
    cbz x17, 1f
    br x17
1:  mov x0, #269
    bl __bf_resolve_and_call
    br x0
.symver __bf_random, random@@LIBC

.balign 4
.globl __bf_sigfillset
.type __bf_sigfillset, %function
__bf_sigfillset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2160]
    cbz x17, 1f
    br x17
1:  mov x0, #270
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigfillset, sigfillset@@LIBC

.balign 4
.globl __bf_fdopen
.type __bf_fdopen, %function
__bf_fdopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2168]
    cbz x17, 1f
    br x17
1:  mov x0, #271
    bl __bf_resolve_and_call
    br x0
.symver __bf_fdopen, fdopen@@LIBC

.balign 4
.globl __bf_timerfd_create
.type __bf_timerfd_create, %function
__bf_timerfd_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2176]
    cbz x17, 1f
    br x17
1:  mov x0, #272
    bl __bf_resolve_and_call
    br x0
.symver __bf_timerfd_create, timerfd_create@@LIBC

.balign 4
.globl __bf_timerfd_settime
.type __bf_timerfd_settime, %function
__bf_timerfd_settime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2184]
    cbz x17, 1f
    br x17
1:  mov x0, #273
    bl __bf_resolve_and_call
    br x0
.symver __bf_timerfd_settime, timerfd_settime@@LIBC

.balign 4
.globl __bf_fputc
.type __bf_fputc, %function
__bf_fputc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2192]
    cbz x17, 1f
    br x17
1:  mov x0, #274
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputc, fputc@@LIBC

.balign 4
.globl __bf_bsearch
.type __bf_bsearch, %function
__bf_bsearch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2200]
    cbz x17, 1f
    br x17
1:  mov x0, #275
    bl __bf_resolve_and_call
    br x0
.symver __bf_bsearch, bsearch@@LIBC

.balign 4
.globl __bf_vfprintf
.type __bf_vfprintf, %function
__bf_vfprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2208]
    cbz x17, 1f
    br x17
1:  mov x0, #276
    bl __bf_resolve_and_call
    br x0
.symver __bf_vfprintf, vfprintf@@LIBC

.balign 4
.globl __bf_pthread_exit
.type __bf_pthread_exit, %function
__bf_pthread_exit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2216]
    cbz x17, 1f
    br x17
1:  mov x0, #277
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_exit, pthread_exit@@LIBC

.balign 4
.globl __bf_finitef
.type __bf_finitef, %function
__bf_finitef:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2224]
    cbz x17, 1f
    br x17
1:  mov x0, #278
    bl __bf_resolve_and_call
    br x0
.symver __bf_finitef, finitef@@LIBC

.balign 4
.globl __bf_cbrt
.type __bf_cbrt, %function
__bf_cbrt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2232]
    cbz x17, 1f
    br x17
1:  mov x0, #279
    bl __bf_resolve_and_call
    br x0
.symver __bf_cbrt, cbrt@@LIBC

.balign 4
.globl __bf_remquof
.type __bf_remquof, %function
__bf_remquof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2240]
    cbz x17, 1f
    br x17
1:  mov x0, #280
    bl __bf_resolve_and_call
    br x0
.symver __bf_remquof, remquof@@LIBC

.balign 4
.globl __bf_strcspn
.type __bf_strcspn, %function
__bf_strcspn:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2248]
    cbz x17, 1f
    br x17
1:  mov x0, #281
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcspn, strcspn@@LIBC

.balign 4
.globl __bf_gmtime_r
.type __bf_gmtime_r, %function
__bf_gmtime_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2256]
    cbz x17, 1f
    br x17
1:  mov x0, #282
    bl __bf_resolve_and_call
    br x0
.symver __bf_gmtime_r, gmtime_r@@LIBC

.balign 4
.globl __bf_difftime
.type __bf_difftime, %function
__bf_difftime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2264]
    cbz x17, 1f
    br x17
1:  mov x0, #283
    bl __bf_resolve_and_call
    br x0
.symver __bf_difftime, difftime@@LIBC

.balign 4
.globl __bf_strpbrk
.type __bf_strpbrk, %function
__bf_strpbrk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2272]
    cbz x17, 1f
    br x17
1:  mov x0, #284
    bl __bf_resolve_and_call
    br x0
.symver __bf_strpbrk, strpbrk@@LIBC

.balign 4
.globl __bf_shutdown
.type __bf_shutdown, %function
__bf_shutdown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2280]
    cbz x17, 1f
    br x17
1:  mov x0, #285
    bl __bf_resolve_and_call
    br x0
.symver __bf_shutdown, shutdown@@LIBC

.balign 4
.globl __bf_memrchr
.type __bf_memrchr, %function
__bf_memrchr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2288]
    cbz x17, 1f
    br x17
1:  mov x0, #286
    bl __bf_resolve_and_call
    br x0
.symver __bf_memrchr, memrchr@@LIBC

.balign 4
.globl __bf_accept4
.type __bf_accept4, %function
__bf_accept4:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2296]
    cbz x17, 1f
    br x17
1:  mov x0, #287
    bl __bf_resolve_and_call
    br x0
.symver __bf_accept4, accept4@@LIBC

.balign 4
.globl __bf_if_nametoindex
.type __bf_if_nametoindex, %function
__bf_if_nametoindex:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2304]
    cbz x17, 1f
    br x17
1:  mov x0, #288
    bl __bf_resolve_and_call
    br x0
.symver __bf_if_nametoindex, if_nametoindex@@LIBC

.balign 4
.globl __bf_setvbuf
.type __bf_setvbuf, %function
__bf_setvbuf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2312]
    cbz x17, 1f
    br x17
1:  mov x0, #289
    bl __bf_resolve_and_call
    br x0
.symver __bf_setvbuf, setvbuf@@LIBC

.balign 4
.globl __bf_realpath
.type __bf_realpath, %function
__bf_realpath:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2320]
    cbz x17, 1f
    br x17
1:  mov x0, #290
    bl __bf_resolve_and_call
    br x0
.symver __bf_realpath, realpath@@LIBC

.balign 4
.globl __bf_recvmmsg
.type __bf_recvmmsg, %function
__bf_recvmmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2328]
    cbz x17, 1f
    br x17
1:  mov x0, #291
    bl __bf_resolve_and_call
    br x0
.symver __bf_recvmmsg, recvmmsg@@LIBC

.balign 4
.globl __bf_getcwd
.type __bf_getcwd, %function
__bf_getcwd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2336]
    cbz x17, 1f
    br x17
1:  mov x0, #292
    bl __bf_resolve_and_call
    br x0
.symver __bf_getcwd, getcwd@@LIBC

.balign 4
.globl __bf_pread
.type __bf_pread, %function
__bf_pread:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2344]
    cbz x17, 1f
    br x17
1:  mov x0, #293
    bl __bf_resolve_and_call
    br x0
.symver __bf_pread, pread@@LIBC

.balign 4
.globl __bf_pwrite
.type __bf_pwrite, %function
__bf_pwrite:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2352]
    cbz x17, 1f
    br x17
1:  mov x0, #294
    bl __bf_resolve_and_call
    br x0
.symver __bf_pwrite, pwrite@@LIBC

.balign 4
.globl __bf_fchmod
.type __bf_fchmod, %function
__bf_fchmod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2360]
    cbz x17, 1f
    br x17
1:  mov x0, #295
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchmod, fchmod@@LIBC

.balign 4
.globl __bf_fchown
.type __bf_fchown, %function
__bf_fchown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2368]
    cbz x17, 1f
    br x17
1:  mov x0, #296
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchown, fchown@@LIBC

.balign 4
.globl __bf_mremap
.type __bf_mremap, %function
__bf_mremap:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2376]
    cbz x17, 1f
    br x17
1:  mov x0, #297
    bl __bf_resolve_and_call
    br x0
.symver __bf_mremap, mremap@@LIBC

.balign 4
.globl __bf_fsync
.type __bf_fsync, %function
__bf_fsync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2384]
    cbz x17, 1f
    br x17
1:  mov x0, #298
    bl __bf_resolve_and_call
    br x0
.symver __bf_fsync, fsync@@LIBC

.balign 4
.globl __bf_utimes
.type __bf_utimes, %function
__bf_utimes:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2392]
    cbz x17, 1f
    br x17
1:  mov x0, #299
    bl __bf_resolve_and_call
    br x0
.symver __bf_utimes, utimes@@LIBC

.balign 4
.globl __bf_msync
.type __bf_msync, %function
__bf_msync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2400]
    cbz x17, 1f
    br x17
1:  mov x0, #300
    bl __bf_resolve_and_call
    br x0
.symver __bf_msync, msync@@LIBC

.balign 4
.globl __bf_statvfs
.type __bf_statvfs, %function
__bf_statvfs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2408]
    cbz x17, 1f
    br x17
1:  mov x0, #301
    bl __bf_resolve_and_call
    br x0
.symver __bf_statvfs, statvfs@@LIBC

.balign 4
.globl __bf_mallinfo
.type __bf_mallinfo, %function
__bf_mallinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2416]
    cbz x17, 1f
    br x17
1:  mov x0, #302
    bl __bf_resolve_and_call
    br x0
.symver __bf_mallinfo, mallinfo@@LIBC

.balign 4
.globl __bf___readlink_chk
.type __bf___readlink_chk, %function
__bf___readlink_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2424]
    cbz x17, 1f
    br x17
1:  mov x0, #303
    bl __bf_resolve_and_call
    br x0
.symver __bf___readlink_chk, __readlink_chk@@LIBC

.balign 4
.globl __bf___gnu_strerror_r
.type __bf___gnu_strerror_r, %function
__bf___gnu_strerror_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2432]
    cbz x17, 1f
    br x17
1:  mov x0, #304
    bl __bf_resolve_and_call
    br x0
.symver __bf___gnu_strerror_r, __gnu_strerror_r@@LIBC

.balign 4
.globl __bf_pthread_getschedparam
.type __bf_pthread_getschedparam, %function
__bf_pthread_getschedparam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2440]
    cbz x17, 1f
    br x17
1:  mov x0, #305
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getschedparam, pthread_getschedparam@@LIBC

.balign 4
.globl __bf_sinf
.type __bf_sinf, %function
__bf_sinf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2448]
    cbz x17, 1f
    br x17
1:  mov x0, #306
    bl __bf_resolve_and_call
    br x0
.symver __bf_sinf, sinf@@LIBC

.balign 4
.globl __bf_sincosf
.type __bf_sincosf, %function
__bf_sincosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2456]
    cbz x17, 1f
    br x17
1:  mov x0, #307
    bl __bf_resolve_and_call
    br x0
.symver __bf_sincosf, sincosf@@LIBC

.balign 4
.globl __bf_exp2
.type __bf_exp2, %function
__bf_exp2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2464]
    cbz x17, 1f
    br x17
1:  mov x0, #308
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp2, exp2@@LIBC

.balign 4
.globl __bf_sincos
.type __bf_sincos, %function
__bf_sincos:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2472]
    cbz x17, 1f
    br x17
1:  mov x0, #309
    bl __bf_resolve_and_call
    br x0
.symver __bf_sincos, sincos@@LIBC

.balign 4
.globl __bf_fmal
.type __bf_fmal, %function
__bf_fmal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2480]
    cbz x17, 1f
    br x17
1:  mov x0, #310
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmal, fmal@@LIBC

.balign 4
.globl __bf_exp2f
.type __bf_exp2f, %function
__bf_exp2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2488]
    cbz x17, 1f
    br x17
1:  mov x0, #311
    bl __bf_resolve_and_call
    br x0
.symver __bf_exp2f, exp2f@@LIBC

.balign 4
.globl __bf_log10f
.type __bf_log10f, %function
__bf_log10f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2496]
    cbz x17, 1f
    br x17
1:  mov x0, #312
    bl __bf_resolve_and_call
    br x0
.symver __bf_log10f, log10f@@LIBC

.balign 4
.globl __bf_logf
.type __bf_logf, %function
__bf_logf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2504]
    cbz x17, 1f
    br x17
1:  mov x0, #313
    bl __bf_resolve_and_call
    br x0
.symver __bf_logf, logf@@LIBC

.balign 4
.globl __bf_powf
.type __bf_powf, %function
__bf_powf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2512]
    cbz x17, 1f
    br x17
1:  mov x0, #314
    bl __bf_resolve_and_call
    br x0
.symver __bf_powf, powf@@LIBC

.balign 4
.globl __bf_fmodf
.type __bf_fmodf, %function
__bf_fmodf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2520]
    cbz x17, 1f
    br x17
1:  mov x0, #315
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmodf, fmodf@@LIBC

.balign 4
.globl __bf_log2f
.type __bf_log2f, %function
__bf_log2f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2528]
    cbz x17, 1f
    br x17
1:  mov x0, #316
    bl __bf_resolve_and_call
    br x0
.symver __bf_log2f, log2f@@LIBC

.balign 4
.globl __bf_expf
.type __bf_expf, %function
__bf_expf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2536]
    cbz x17, 1f
    br x17
1:  mov x0, #317
    bl __bf_resolve_and_call
    br x0
.symver __bf_expf, expf@@LIBC

.balign 4
.globl __bf_powl
.type __bf_powl, %function
__bf_powl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2544]
    cbz x17, 1f
    br x17
1:  mov x0, #318
    bl __bf_resolve_and_call
    br x0
.symver __bf_powl, powl@@LIBC

.balign 4
.globl __bf_cosf
.type __bf_cosf, %function
__bf_cosf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2552]
    cbz x17, 1f
    br x17
1:  mov x0, #319
    bl __bf_resolve_and_call
    br x0
.symver __bf_cosf, cosf@@LIBC

.balign 4
.globl __bf_pthread_key_delete
.type __bf_pthread_key_delete, %function
__bf_pthread_key_delete:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2560]
    cbz x17, 1f
    br x17
1:  mov x0, #320
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_key_delete, pthread_key_delete@@LIBC

.balign 4
.globl __bf_sysinfo
.type __bf_sysinfo, %function
__bf_sysinfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2568]
    cbz x17, 1f
    br x17
1:  mov x0, #321
    bl __bf_resolve_and_call
    br x0
.symver __bf_sysinfo, sysinfo@@LIBC

.balign 4
.globl __bf_madvise
.type __bf_madvise, %function
__bf_madvise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2576]
    cbz x17, 1f
    br x17
1:  mov x0, #322
    bl __bf_resolve_and_call
    br x0
.symver __bf_madvise, madvise@@LIBC

.balign 4
.globl __bf_pthread_setname_np
.type __bf_pthread_setname_np, %function
__bf_pthread_setname_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2584]
    cbz x17, 1f
    br x17
1:  mov x0, #323
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setname_np, pthread_setname_np@@LIBC

.balign 4
.globl __bf_pthread_getattr_np
.type __bf_pthread_getattr_np, %function
__bf_pthread_getattr_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2592]
    cbz x17, 1f
    br x17
1:  mov x0, #324
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getattr_np, pthread_getattr_np@@LIBC

.balign 4
.globl __bf_pthread_attr_getstack
.type __bf_pthread_attr_getstack, %function
__bf_pthread_attr_getstack:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2600]
    cbz x17, 1f
    br x17
1:  mov x0, #325
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_attr_getstack, pthread_attr_getstack@@LIBC

.balign 4
.globl __bf_mlock
.type __bf_mlock, %function
__bf_mlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2608]
    cbz x17, 1f
    br x17
1:  mov x0, #326
    bl __bf_resolve_and_call
    br x0
.symver __bf_mlock, mlock@@LIBC

.balign 4
.globl __bf_dl_iterate_phdr
.type __bf_dl_iterate_phdr, %function
__bf_dl_iterate_phdr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2616]
    cbz x17, 1f
    br x17
1:  mov x0, #327
    bl __bf_resolve_and_call
    br x0
.symver __bf_dl_iterate_phdr, dl_iterate_phdr@@LIBC

.balign 4
.globl __bf_isspace
.type __bf_isspace, %function
__bf_isspace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2624]
    cbz x17, 1f
    br x17
1:  mov x0, #328
    bl __bf_resolve_and_call
    br x0
.symver __bf_isspace, isspace@@LIBC

.balign 4
.globl __bf_gethostbyname
.type __bf_gethostbyname, %function
__bf_gethostbyname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2632]
    cbz x17, 1f
    br x17
1:  mov x0, #329
    bl __bf_resolve_and_call
    br x0
.symver __bf_gethostbyname, gethostbyname@@LIBC

.balign 4
.globl __bf_strcat
.type __bf_strcat, %function
__bf_strcat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2640]
    cbz x17, 1f
    br x17
1:  mov x0, #330
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcat, strcat@@LIBC

.balign 4
.globl __bf_sendmmsg
.type __bf_sendmmsg, %function
__bf_sendmmsg:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2648]
    cbz x17, 1f
    br x17
1:  mov x0, #331
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendmmsg, sendmmsg@@LIBC

.balign 4
.globl __bf_tolower
.type __bf_tolower, %function
__bf_tolower:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2656]
    cbz x17, 1f
    br x17
1:  mov x0, #332
    bl __bf_resolve_and_call
    br x0
.symver __bf_tolower, tolower@@LIBC

.balign 4
.globl __bf_pthread_rwlock_destroy
.type __bf_pthread_rwlock_destroy, %function
__bf_pthread_rwlock_destroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2664]
    cbz x17, 1f
    br x17
1:  mov x0, #333
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_destroy, pthread_rwlock_destroy@@LIBC

.balign 4
.globl __bf_pthread_rwlock_init
.type __bf_pthread_rwlock_init, %function
__bf_pthread_rwlock_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2672]
    cbz x17, 1f
    br x17
1:  mov x0, #334
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_init, pthread_rwlock_init@@LIBC

.balign 4
.globl __bf_pthread_rwlock_rdlock
.type __bf_pthread_rwlock_rdlock, %function
__bf_pthread_rwlock_rdlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2680]
    cbz x17, 1f
    br x17
1:  mov x0, #335
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_rdlock, pthread_rwlock_rdlock@@LIBC

.balign 4
.globl __bf_pthread_rwlock_unlock
.type __bf_pthread_rwlock_unlock, %function
__bf_pthread_rwlock_unlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2688]
    cbz x17, 1f
    br x17
1:  mov x0, #336
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_unlock, pthread_rwlock_unlock@@LIBC

.balign 4
.globl __bf_pthread_rwlock_wrlock
.type __bf_pthread_rwlock_wrlock, %function
__bf_pthread_rwlock_wrlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2696]
    cbz x17, 1f
    br x17
1:  mov x0, #337
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_rwlock_wrlock, pthread_rwlock_wrlock@@LIBC

.balign 4
.globl __bf_signal
.type __bf_signal, %function
__bf_signal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2704]
    cbz x17, 1f
    br x17
1:  mov x0, #338
    bl __bf_resolve_and_call
    br x0
.symver __bf_signal, signal@@LIBC

.balign 4
.globl __bf_tcgetattr
.type __bf_tcgetattr, %function
__bf_tcgetattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2712]
    cbz x17, 1f
    br x17
1:  mov x0, #339
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcgetattr, tcgetattr@@LIBC

.balign 4
.globl __bf_tcsetattr
.type __bf_tcsetattr, %function
__bf_tcsetattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2720]
    cbz x17, 1f
    br x17
1:  mov x0, #340
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcsetattr, tcsetattr@@LIBC

.balign 4
.globl __bf_utime
.type __bf_utime, %function
__bf_utime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2728]
    cbz x17, 1f
    br x17
1:  mov x0, #341
    bl __bf_resolve_and_call
    br x0
.symver __bf_utime, utime@@LIBC

.balign 4
.globl __bf_vasprintf
.type __bf_vasprintf, %function
__bf_vasprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2736]
    cbz x17, 1f
    br x17
1:  mov x0, #342
    bl __bf_resolve_and_call
    br x0
.symver __bf_vasprintf, vasprintf@@LIBC

.balign 4
.globl __bf_openlog
.type __bf_openlog, %function
__bf_openlog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2744]
    cbz x17, 1f
    br x17
1:  mov x0, #343
    bl __bf_resolve_and_call
    br x0
.symver __bf_openlog, openlog@@LIBC

.balign 4
.globl __bf_syslog
.type __bf_syslog, %function
__bf_syslog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2752]
    cbz x17, 1f
    br x17
1:  mov x0, #344
    bl __bf_resolve_and_call
    br x0
.symver __bf_syslog, syslog@@LIBC

.balign 4
.globl __bf_closelog
.type __bf_closelog, %function
__bf_closelog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2760]
    cbz x17, 1f
    br x17
1:  mov x0, #345
    bl __bf_resolve_and_call
    br x0
.symver __bf_closelog, closelog@@LIBC

.balign 4
.globl __bf_ungetc
.type __bf_ungetc, %function
__bf_ungetc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2768]
    cbz x17, 1f
    br x17
1:  mov x0, #346
    bl __bf_resolve_and_call
    br x0
.symver __bf_ungetc, ungetc@@LIBC

.balign 4
.globl __bf_getc
.type __bf_getc, %function
__bf_getc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2776]
    cbz x17, 1f
    br x17
1:  mov x0, #347
    bl __bf_resolve_and_call
    br x0
.symver __bf_getc, getc@@LIBC

.balign 4
.globl __bf_ungetwc
.type __bf_ungetwc, %function
__bf_ungetwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2784]
    cbz x17, 1f
    br x17
1:  mov x0, #348
    bl __bf_resolve_and_call
    br x0
.symver __bf_ungetwc, ungetwc@@LIBC

.balign 4
.globl __bf_getwc
.type __bf_getwc, %function
__bf_getwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2792]
    cbz x17, 1f
    br x17
1:  mov x0, #349
    bl __bf_resolve_and_call
    br x0
.symver __bf_getwc, getwc@@LIBC

.balign 4
.globl __bf_fputwc
.type __bf_fputwc, %function
__bf_fputwc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2800]
    cbz x17, 1f
    br x17
1:  mov x0, #350
    bl __bf_resolve_and_call
    br x0
.symver __bf_fputwc, fputwc@@LIBC

.balign 4
.globl __bf_newlocale
.type __bf_newlocale, %function
__bf_newlocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2808]
    cbz x17, 1f
    br x17
1:  mov x0, #351
    bl __bf_resolve_and_call
    br x0
.symver __bf_newlocale, newlocale@@LIBC

.balign 4
.globl __bf_uselocale
.type __bf_uselocale, %function
__bf_uselocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2816]
    cbz x17, 1f
    br x17
1:  mov x0, #352
    bl __bf_resolve_and_call
    br x0
.symver __bf_uselocale, uselocale@@LIBC

.balign 4
.globl __bf_vsscanf
.type __bf_vsscanf, %function
__bf_vsscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2824]
    cbz x17, 1f
    br x17
1:  mov x0, #353
    bl __bf_resolve_and_call
    br x0
.symver __bf_vsscanf, vsscanf@@LIBC

.balign 4
.globl __bf_strftime_l
.type __bf_strftime_l, %function
__bf_strftime_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2832]
    cbz x17, 1f
    br x17
1:  mov x0, #354
    bl __bf_resolve_and_call
    br x0
.symver __bf_strftime_l, strftime_l@@LIBC

.balign 4
.globl __bf_mbsrtowcs
.type __bf_mbsrtowcs, %function
__bf_mbsrtowcs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2840]
    cbz x17, 1f
    br x17
1:  mov x0, #355
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbsrtowcs, mbsrtowcs@@LIBC

.balign 4
.globl __bf_freelocale
.type __bf_freelocale, %function
__bf_freelocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2848]
    cbz x17, 1f
    br x17
1:  mov x0, #356
    bl __bf_resolve_and_call
    br x0
.symver __bf_freelocale, freelocale@@LIBC

.balign 4
.globl __bf_strcoll_l
.type __bf_strcoll_l, %function
__bf_strcoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2856]
    cbz x17, 1f
    br x17
1:  mov x0, #357
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcoll_l, strcoll_l@@LIBC

.balign 4
.globl __bf_strxfrm_l
.type __bf_strxfrm_l, %function
__bf_strxfrm_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2864]
    cbz x17, 1f
    br x17
1:  mov x0, #358
    bl __bf_resolve_and_call
    br x0
.symver __bf_strxfrm_l, strxfrm_l@@LIBC

.balign 4
.globl __bf_wcscoll_l
.type __bf_wcscoll_l, %function
__bf_wcscoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2872]
    cbz x17, 1f
    br x17
1:  mov x0, #359
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcscoll_l, wcscoll_l@@LIBC

.balign 4
.globl __bf_wcsxfrm_l
.type __bf_wcsxfrm_l, %function
__bf_wcsxfrm_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2880]
    cbz x17, 1f
    br x17
1:  mov x0, #360
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcsxfrm_l, wcsxfrm_l@@LIBC

.balign 4
.globl __bf_iswlower_l
.type __bf_iswlower_l, %function
__bf_iswlower_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2888]
    cbz x17, 1f
    br x17
1:  mov x0, #361
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswlower_l, iswlower_l@@LIBC

.balign 4
.globl __bf_iswspace_l
.type __bf_iswspace_l, %function
__bf_iswspace_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2896]
    cbz x17, 1f
    br x17
1:  mov x0, #362
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswspace_l, iswspace_l@@LIBC

.balign 4
.globl __bf_iswprint_l
.type __bf_iswprint_l, %function
__bf_iswprint_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2904]
    cbz x17, 1f
    br x17
1:  mov x0, #363
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswprint_l, iswprint_l@@LIBC

.balign 4
.globl __bf_iswblank_l
.type __bf_iswblank_l, %function
__bf_iswblank_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2912]
    cbz x17, 1f
    br x17
1:  mov x0, #364
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswblank_l, iswblank_l@@LIBC

.balign 4
.globl __bf_iswcntrl_l
.type __bf_iswcntrl_l, %function
__bf_iswcntrl_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2920]
    cbz x17, 1f
    br x17
1:  mov x0, #365
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswcntrl_l, iswcntrl_l@@LIBC

.balign 4
.globl __bf_iswupper_l
.type __bf_iswupper_l, %function
__bf_iswupper_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2928]
    cbz x17, 1f
    br x17
1:  mov x0, #366
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswupper_l, iswupper_l@@LIBC

.balign 4
.globl __bf_iswalpha_l
.type __bf_iswalpha_l, %function
__bf_iswalpha_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2936]
    cbz x17, 1f
    br x17
1:  mov x0, #367
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswalpha_l, iswalpha_l@@LIBC

.balign 4
.globl __bf_iswdigit_l
.type __bf_iswdigit_l, %function
__bf_iswdigit_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2944]
    cbz x17, 1f
    br x17
1:  mov x0, #368
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswdigit_l, iswdigit_l@@LIBC

.balign 4
.globl __bf_iswpunct_l
.type __bf_iswpunct_l, %function
__bf_iswpunct_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2952]
    cbz x17, 1f
    br x17
1:  mov x0, #369
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswpunct_l, iswpunct_l@@LIBC

.balign 4
.globl __bf_iswxdigit_l
.type __bf_iswxdigit_l, %function
__bf_iswxdigit_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2960]
    cbz x17, 1f
    br x17
1:  mov x0, #370
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswxdigit_l, iswxdigit_l@@LIBC

.balign 4
.globl __bf_towupper_l
.type __bf_towupper_l, %function
__bf_towupper_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2968]
    cbz x17, 1f
    br x17
1:  mov x0, #371
    bl __bf_resolve_and_call
    br x0
.symver __bf_towupper_l, towupper_l@@LIBC

.balign 4
.globl __bf_towlower_l
.type __bf_towlower_l, %function
__bf_towlower_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2976]
    cbz x17, 1f
    br x17
1:  mov x0, #372
    bl __bf_resolve_and_call
    br x0
.symver __bf_towlower_l, towlower_l@@LIBC

.balign 4
.globl __bf_btowc
.type __bf_btowc, %function
__bf_btowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2984]
    cbz x17, 1f
    br x17
1:  mov x0, #373
    bl __bf_resolve_and_call
    br x0
.symver __bf_btowc, btowc@@LIBC

.balign 4
.globl __bf_wctob
.type __bf_wctob, %function
__bf_wctob:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #2992]
    cbz x17, 1f
    br x17
1:  mov x0, #374
    bl __bf_resolve_and_call
    br x0
.symver __bf_wctob, wctob@@LIBC

.balign 4
.globl __bf_wcsnrtombs
.type __bf_wcsnrtombs, %function
__bf_wcsnrtombs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3000]
    cbz x17, 1f
    br x17
1:  mov x0, #375
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcsnrtombs, wcsnrtombs@@LIBC

.balign 4
.globl __bf_wcrtomb
.type __bf_wcrtomb, %function
__bf_wcrtomb:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3008]
    cbz x17, 1f
    br x17
1:  mov x0, #376
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcrtomb, wcrtomb@@LIBC

.balign 4
.globl __bf_mbsnrtowcs
.type __bf_mbsnrtowcs, %function
__bf_mbsnrtowcs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3016]
    cbz x17, 1f
    br x17
1:  mov x0, #377
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbsnrtowcs, mbsnrtowcs@@LIBC

.balign 4
.globl __bf_mbrtowc
.type __bf_mbrtowc, %function
__bf_mbrtowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3024]
    cbz x17, 1f
    br x17
1:  mov x0, #378
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbrtowc, mbrtowc@@LIBC

.balign 4
.globl __bf_mbtowc
.type __bf_mbtowc, %function
__bf_mbtowc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3032]
    cbz x17, 1f
    br x17
1:  mov x0, #379
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbtowc, mbtowc@@LIBC

.balign 4
.globl __bf___ctype_get_mb_cur_max
.type __bf___ctype_get_mb_cur_max, %function
__bf___ctype_get_mb_cur_max:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3040]
    cbz x17, 1f
    br x17
1:  mov x0, #380
    bl __bf_resolve_and_call
    br x0
.symver __bf___ctype_get_mb_cur_max, __ctype_get_mb_cur_max@@LIBC

.balign 4
.globl __bf_mbrlen
.type __bf_mbrlen, %function
__bf_mbrlen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3048]
    cbz x17, 1f
    br x17
1:  mov x0, #381
    bl __bf_resolve_and_call
    br x0
.symver __bf_mbrlen, mbrlen@@LIBC

.balign 4
.globl __bf_strtoll_l
.type __bf_strtoll_l, %function
__bf_strtoll_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3056]
    cbz x17, 1f
    br x17
1:  mov x0, #382
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoll_l, strtoll_l@@LIBC

.balign 4
.globl __bf_strtoull_l
.type __bf_strtoull_l, %function
__bf_strtoull_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3064]
    cbz x17, 1f
    br x17
1:  mov x0, #383
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoull_l, strtoull_l@@LIBC

.balign 4
.globl __bf_strtold_l
.type __bf_strtold_l, %function
__bf_strtold_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3072]
    cbz x17, 1f
    br x17
1:  mov x0, #384
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtold_l, strtold_l@@LIBC

.balign 4
.globl __bf___cxa_thread_atexit_impl
.type __bf___cxa_thread_atexit_impl, %function
__bf___cxa_thread_atexit_impl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3080]
    cbz x17, 1f
    br x17
1:  mov x0, #385
    bl __bf_resolve_and_call
    br x0
.symver __bf___cxa_thread_atexit_impl, __cxa_thread_atexit_impl@@LIBC

.balign 4
.globl __bf_strncat
.type __bf_strncat, %function
__bf_strncat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3088]
    cbz x17, 1f
    br x17
1:  mov x0, #386
    bl __bf_resolve_and_call
    br x0
.symver __bf_strncat, strncat@@LIBC

.balign 4
.globl __bf___cfi_slowpath
.type __bf___cfi_slowpath, %function
__bf___cfi_slowpath:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3096]
    cbz x17, 1f
    br x17
1:  mov x0, #387
    bl __bf_resolve_and_call
    br x0
.symver __bf___cfi_slowpath, __cfi_slowpath@@LIBC_OMR1

.balign 4
.globl __bf_android_fdsan_close_with_tag
.type __bf_android_fdsan_close_with_tag, %function
__bf_android_fdsan_close_with_tag:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3104]
    cbz x17, 1f
    br x17
1:  mov x0, #388
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_close_with_tag, android_fdsan_close_with_tag@@LIBC_Q

.balign 4
.globl __bf_android_fdsan_create_owner_tag
.type __bf_android_fdsan_create_owner_tag, %function
__bf_android_fdsan_create_owner_tag:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3112]
    cbz x17, 1f
    br x17
1:  mov x0, #389
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_create_owner_tag, android_fdsan_create_owner_tag@@LIBC_Q

.balign 4
.globl __bf_android_fdsan_exchange_owner_tag
.type __bf_android_fdsan_exchange_owner_tag, %function
__bf_android_fdsan_exchange_owner_tag:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3120]
    cbz x17, 1f
    br x17
1:  mov x0, #390
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_exchange_owner_tag, android_fdsan_exchange_owner_tag@@LIBC_Q

.balign 4
.globl __bf_free
.type __bf_free, %function
__bf_free:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3128]
    cbz x17, 1f
    br x17
1:  mov x0, #391
    bl __bf_resolve_and_call
    br x0
.symver __bf_free, free@@LIBC

.balign 4
.globl __bf_malloc
.type __bf_malloc, %function
__bf_malloc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3136]
    cbz x17, 1f
    br x17
1:  mov x0, #392
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc, malloc@@LIBC

.balign 4
.globl __bf_dup
.type __bf_dup, %function
__bf_dup:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3144]
    cbz x17, 1f
    br x17
1:  mov x0, #393
    bl __bf_resolve_and_call
    br x0
.symver __bf_dup, dup@@LIBC

.balign 4
.globl __bf_android_mallopt
.type __bf_android_mallopt, %function
__bf_android_mallopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3152]
    cbz x17, 1f
    br x17
1:  mov x0, #394
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_mallopt, android_mallopt@@LIBC_Q

.balign 4
.globl __bf___system_property_find
.type __bf___system_property_find, %function
__bf___system_property_find:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3160]
    cbz x17, 1f
    br x17
1:  mov x0, #395
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_find, __system_property_find@@LIBC

.balign 4
.globl __bf___system_property_read_callback
.type __bf___system_property_read_callback, %function
__bf___system_property_read_callback:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3168]
    cbz x17, 1f
    br x17
1:  mov x0, #396
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_read_callback, __system_property_read_callback@@LIBC_O

.balign 4
.globl __bf_getprogname
.type __bf_getprogname, %function
__bf_getprogname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3176]
    cbz x17, 1f
    br x17
1:  mov x0, #397
    bl __bf_resolve_and_call
    br x0
.symver __bf_getprogname, getprogname@@LIBC

.balign 4
.globl __bf_sleep
.type __bf_sleep, %function
__bf_sleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3184]
    cbz x17, 1f
    br x17
1:  mov x0, #398
    bl __bf_resolve_and_call
    br x0
.symver __bf_sleep, sleep@@LIBC

.balign 4
.globl __bf_android_dlopen_ext
.type __bf_android_dlopen_ext, %function
__bf_android_dlopen_ext:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3192]
    cbz x17, 1f
    br x17
1:  mov x0, #399
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_dlopen_ext, android_dlopen_ext@@LIBC

.balign 4
.globl __bf_nrand48
.type __bf_nrand48, %function
__bf_nrand48:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3200]
    cbz x17, 1f
    br x17
1:  mov x0, #400
    bl __bf_resolve_and_call
    br x0
.symver __bf_nrand48, nrand48@@LIBC

.balign 4
.globl __bf_remove
.type __bf_remove, %function
__bf_remove:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3208]
    cbz x17, 1f
    br x17
1:  mov x0, #401
    bl __bf_resolve_and_call
    br x0
.symver __bf_remove, remove@@LIBC

.balign 4
.globl __bf_utimensat
.type __bf_utimensat, %function
__bf_utimensat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3216]
    cbz x17, 1f
    br x17
1:  mov x0, #402
    bl __bf_resolve_and_call
    br x0
.symver __bf_utimensat, utimensat@@LIBC

.balign 4
.globl __bf_futimens
.type __bf_futimens, %function
__bf_futimens:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3224]
    cbz x17, 1f
    br x17
1:  mov x0, #403
    bl __bf_resolve_and_call
    br x0
.symver __bf_futimens, futimens@@LIBC

.balign 4
.globl __bf_getpwuid
.type __bf_getpwuid, %function
__bf_getpwuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3232]
    cbz x17, 1f
    br x17
1:  mov x0, #404
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpwuid, getpwuid@@LIBC

.balign 4
.globl __bf_getgrgid
.type __bf_getgrgid, %function
__bf_getgrgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3240]
    cbz x17, 1f
    br x17
1:  mov x0, #405
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgrgid, getgrgid@@LIBC

.balign 4
.globl __bf_aligned_alloc
.type __bf_aligned_alloc, %function
__bf_aligned_alloc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3248]
    cbz x17, 1f
    br x17
1:  mov x0, #406
    bl __bf_resolve_and_call
    br x0
.symver __bf_aligned_alloc, aligned_alloc@@LIBC_P

.balign 4
.globl __bf__Unwind_RaiseException
.type __bf__Unwind_RaiseException, %function
__bf__Unwind_RaiseException:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3256]
    cbz x17, 1f
    br x17
1:  mov x0, #407
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_RaiseException, _Unwind_RaiseException@@LIBC_R

.balign 4
.globl __bf__Unwind_Resume
.type __bf__Unwind_Resume, %function
__bf__Unwind_Resume:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3264]
    cbz x17, 1f
    br x17
1:  mov x0, #408
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_Resume, _Unwind_Resume@@LIBC_R

.balign 4
.globl __bf__Unwind_SetGR
.type __bf__Unwind_SetGR, %function
__bf__Unwind_SetGR:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3272]
    cbz x17, 1f
    br x17
1:  mov x0, #409
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_SetGR, _Unwind_SetGR@@LIBC_R

.balign 4
.globl __bf__Unwind_SetIP
.type __bf__Unwind_SetIP, %function
__bf__Unwind_SetIP:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3280]
    cbz x17, 1f
    br x17
1:  mov x0, #410
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_SetIP, _Unwind_SetIP@@LIBC_R

.balign 4
.globl __bf__Unwind_GetLanguageSpecificData
.type __bf__Unwind_GetLanguageSpecificData, %function
__bf__Unwind_GetLanguageSpecificData:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3288]
    cbz x17, 1f
    br x17
1:  mov x0, #411
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetLanguageSpecificData, _Unwind_GetLanguageSpecificData@@LIBC_R

.balign 4
.globl __bf__Unwind_GetIP
.type __bf__Unwind_GetIP, %function
__bf__Unwind_GetIP:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3296]
    cbz x17, 1f
    br x17
1:  mov x0, #412
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetIP, _Unwind_GetIP@@LIBC_R

.balign 4
.globl __bf__Unwind_GetRegionStart
.type __bf__Unwind_GetRegionStart, %function
__bf__Unwind_GetRegionStart:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3304]
    cbz x17, 1f
    br x17
1:  mov x0, #413
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetRegionStart, _Unwind_GetRegionStart@@LIBC_R

.balign 4
.globl __bf_realloc
.type __bf_realloc, %function
__bf_realloc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3312]
    cbz x17, 1f
    br x17
1:  mov x0, #414
    bl __bf_resolve_and_call
    br x0
.symver __bf_realloc, realloc@@LIBC

.balign 4
.globl __bf_posix_memalign
.type __bf_posix_memalign, %function
__bf_posix_memalign:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3320]
    cbz x17, 1f
    br x17
1:  mov x0, #415
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_memalign, posix_memalign@@LIBC

.balign 4
.globl __bf_pthread_cond_clockwait
.type __bf_pthread_cond_clockwait, %function
__bf_pthread_cond_clockwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3328]
    cbz x17, 1f
    br x17
1:  mov x0, #416
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_cond_clockwait, pthread_cond_clockwait@@LIBC_R

.balign 4
.globl __bf_asinhf
.type __bf_asinhf, %function
__bf_asinhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3336]
    cbz x17, 1f
    br x17
1:  mov x0, #417
    bl __bf_resolve_and_call
    br x0
.symver __bf_asinhf, asinhf@@LIBC

.balign 4
.globl __bf_acoshf
.type __bf_acoshf, %function
__bf_acoshf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3344]
    cbz x17, 1f
    br x17
1:  mov x0, #418
    bl __bf_resolve_and_call
    br x0
.symver __bf_acoshf, acoshf@@LIBC

.balign 4
.globl __bf_atanhf
.type __bf_atanhf, %function
__bf_atanhf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3352]
    cbz x17, 1f
    br x17
1:  mov x0, #419
    bl __bf_resolve_and_call
    br x0
.symver __bf_atanhf, atanhf@@LIBC

.balign 4
.globl __bf_calloc
.type __bf_calloc, %function
__bf_calloc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3360]
    cbz x17, 1f
    br x17
1:  mov x0, #420
    bl __bf_resolve_and_call
    br x0
.symver __bf_calloc, calloc@@LIBC

.balign 4
.globl __bf_memalign
.type __bf_memalign, %function
__bf_memalign:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3368]
    cbz x17, 1f
    br x17
1:  mov x0, #421
    bl __bf_resolve_and_call
    br x0
.symver __bf_memalign, memalign@@LIBC

.balign 4
.globl __bf_unsetenv
.type __bf_unsetenv, %function
__bf_unsetenv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3376]
    cbz x17, 1f
    br x17
1:  mov x0, #422
    bl __bf_resolve_and_call
    br x0
.symver __bf_unsetenv, unsetenv@@LIBC

.balign 4
.globl __bf_setenv
.type __bf_setenv, %function
__bf_setenv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3384]
    cbz x17, 1f
    br x17
1:  mov x0, #423
    bl __bf_resolve_and_call
    br x0
.symver __bf_setenv, setenv@@LIBC

.balign 4
.globl __bf__Unwind_DeleteException
.type __bf__Unwind_DeleteException, %function
__bf__Unwind_DeleteException:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3392]
    cbz x17, 1f
    br x17
1:  mov x0, #424
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_DeleteException, _Unwind_DeleteException@@LIBC_R

.balign 4
.globl __bf_setbuf
.type __bf_setbuf, %function
__bf_setbuf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3400]
    cbz x17, 1f
    br x17
1:  mov x0, #425
    bl __bf_resolve_and_call
    br x0
.symver __bf_setbuf, setbuf@@LIBC

.balign 4
.globl __bf_strtof_l
.type __bf_strtof_l, %function
__bf_strtof_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3408]
    cbz x17, 1f
    br x17
1:  mov x0, #426
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtof_l, strtof_l@@LIBC_O

.balign 4
.globl __bf_strtod_l
.type __bf_strtod_l, %function
__bf_strtod_l:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3416]
    cbz x17, 1f
    br x17
1:  mov x0, #427
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtod_l, strtod_l@@LIBC_O

.balign 4
.globl __bf_strdup
.type __bf_strdup, %function
__bf_strdup:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3424]
    cbz x17, 1f
    br x17
1:  mov x0, #428
    bl __bf_resolve_and_call
    br x0
.symver __bf_strdup, strdup@@LIBC

.balign 4
.globl __bf_fabs
.type __bf_fabs, %function
__bf_fabs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3432]
    cbz x17, 1f
    br x17
1:  mov x0, #429
    bl __bf_resolve_and_call
    br x0
.symver __bf_fabs, fabs@@LIBC

.balign 4
.globl __bf_ceil
.type __bf_ceil, %function
__bf_ceil:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3440]
    cbz x17, 1f
    br x17
1:  mov x0, #430
    bl __bf_resolve_and_call
    br x0
.symver __bf_ceil, ceil@@LIBC

.balign 4
.globl __bf_floor
.type __bf_floor, %function
__bf_floor:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3448]
    cbz x17, 1f
    br x17
1:  mov x0, #431
    bl __bf_resolve_and_call
    br x0
.symver __bf_floor, floor@@LIBC

.balign 4
.globl __bf_sqrt
.type __bf_sqrt, %function
__bf_sqrt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3456]
    cbz x17, 1f
    br x17
1:  mov x0, #432
    bl __bf_resolve_and_call
    br x0
.symver __bf_sqrt, sqrt@@LIBC

.balign 4
.globl __bf_sigprocmask
.type __bf_sigprocmask, %function
__bf_sigprocmask:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3464]
    cbz x17, 1f
    br x17
1:  mov x0, #433
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigprocmask, sigprocmask@@LIBC

.balign 4
.globl __bf___pread_chk
.type __bf___pread_chk, %function
__bf___pread_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3472]
    cbz x17, 1f
    br x17
1:  mov x0, #434
    bl __bf_resolve_and_call
    br x0
.symver __bf___pread_chk, __pread_chk@@LIBC

.balign 4
.globl __bf_statfs
.type __bf_statfs, %function
__bf_statfs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3480]
    cbz x17, 1f
    br x17
1:  mov x0, #435
    bl __bf_resolve_and_call
    br x0
.symver __bf_statfs, statfs@@LIBC

.balign 4
.globl __bf_symlink
.type __bf_symlink, %function
__bf_symlink:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3488]
    cbz x17, 1f
    br x17
1:  mov x0, #436
    bl __bf_resolve_and_call
    br x0
.symver __bf_symlink, symlink@@LIBC

.balign 4
.globl __bf_strsep
.type __bf_strsep, %function
__bf_strsep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3496]
    cbz x17, 1f
    br x17
1:  mov x0, #437
    bl __bf_resolve_and_call
    br x0
.symver __bf_strsep, strsep@@LIBC

.balign 4
.globl __bf_setrlimit
.type __bf_setrlimit, %function
__bf_setrlimit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3504]
    cbz x17, 1f
    br x17
1:  mov x0, #438
    bl __bf_resolve_and_call
    br x0
.symver __bf_setrlimit, setrlimit@@LIBC

.balign 4
.globl __bf_dup2
.type __bf_dup2, %function
__bf_dup2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3512]
    cbz x17, 1f
    br x17
1:  mov x0, #439
    bl __bf_resolve_and_call
    br x0
.symver __bf_dup2, dup2@@LIBC

.balign 4
.globl __bf_isatty
.type __bf_isatty, %function
__bf_isatty:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3520]
    cbz x17, 1f
    br x17
1:  mov x0, #440
    bl __bf_resolve_and_call
    br x0
.symver __bf_isatty, isatty@@LIBC

.balign 4
.globl __bf_getrusage
.type __bf_getrusage, %function
__bf_getrusage:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3528]
    cbz x17, 1f
    br x17
1:  mov x0, #441
    bl __bf_resolve_and_call
    br x0
.symver __bf_getrusage, getrusage@@LIBC

.balign 4
.globl __bf_alarm
.type __bf_alarm, %function
__bf_alarm:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3536]
    cbz x17, 1f
    br x17
1:  mov x0, #442
    bl __bf_resolve_and_call
    br x0
.symver __bf_alarm, alarm@@LIBC

.balign 4
.globl __bf_kill
.type __bf_kill, %function
__bf_kill:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3544]
    cbz x17, 1f
    br x17
1:  mov x0, #443
    bl __bf_resolve_and_call
    br x0
.symver __bf_kill, kill@@LIBC

.balign 4
.globl __bf_wait
.type __bf_wait, %function
__bf_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3552]
    cbz x17, 1f
    br x17
1:  mov x0, #444
    bl __bf_resolve_and_call
    br x0
.symver __bf_wait, wait@@LIBC

.balign 4
.globl __bf_strsignal
.type __bf_strsignal, %function
__bf_strsignal:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3560]
    cbz x17, 1f
    br x17
1:  mov x0, #445
    bl __bf_resolve_and_call
    br x0
.symver __bf_strsignal, strsignal@@LIBC

.balign 4
.globl __bf_getrlimit
.type __bf_getrlimit, %function
__bf_getrlimit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3568]
    cbz x17, 1f
    br x17
1:  mov x0, #446
    bl __bf_resolve_and_call
    br x0
.symver __bf_getrlimit, getrlimit@@LIBC

.balign 4
.globl __bf___register_frame
.type __bf___register_frame, %function
__bf___register_frame:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3576]
    cbz x17, 1f
    br x17
1:  mov x0, #447
    bl __bf_resolve_and_call
    br x0
.symver __bf___register_frame, __register_frame@@LIBC_R

.balign 4
.globl __bf___deregister_frame
.type __bf___deregister_frame, %function
__bf___deregister_frame:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3584]
    cbz x17, 1f
    br x17
1:  mov x0, #448
    bl __bf_resolve_and_call
    br x0
.symver __bf___deregister_frame, __deregister_frame@@LIBC_R

.balign 4
.globl __bf___strlcpy_chk
.type __bf___strlcpy_chk, %function
__bf___strlcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3592]
    cbz x17, 1f
    br x17
1:  mov x0, #449
    bl __bf_resolve_and_call
    br x0
.symver __bf___strlcpy_chk, __strlcpy_chk@@LIBC

.balign 4
.globl __bf_arc4random
.type __bf_arc4random, %function
__bf_arc4random:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3600]
    cbz x17, 1f
    br x17
1:  mov x0, #450
    bl __bf_resolve_and_call
    br x0
.symver __bf_arc4random, arc4random@@LIBC

.balign 4
.globl __bf___strrchr_chk
.type __bf___strrchr_chk, %function
__bf___strrchr_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3608]
    cbz x17, 1f
    br x17
1:  mov x0, #451
    bl __bf_resolve_and_call
    br x0
.symver __bf___strrchr_chk, __strrchr_chk@@LIBC

.balign 4
.globl __bf___strncat_chk
.type __bf___strncat_chk, %function
__bf___strncat_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3616]
    cbz x17, 1f
    br x17
1:  mov x0, #452
    bl __bf_resolve_and_call
    br x0
.symver __bf___strncat_chk, __strncat_chk@@LIBC

.balign 4
.globl __bf_tgammaf
.type __bf_tgammaf, %function
__bf_tgammaf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3624]
    cbz x17, 1f
    br x17
1:  mov x0, #453
    bl __bf_resolve_and_call
    br x0
.symver __bf_tgammaf, tgammaf@@LIBC

.balign 4
.globl __bf_expm1f
.type __bf_expm1f, %function
__bf_expm1f:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3632]
    cbz x17, 1f
    br x17
1:  mov x0, #454
    bl __bf_resolve_and_call
    br x0
.symver __bf_expm1f, expm1f@@LIBC

.balign 4
.globl __bf_fdimf
.type __bf_fdimf, %function
__bf_fdimf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3640]
    cbz x17, 1f
    br x17
1:  mov x0, #455
    bl __bf_resolve_and_call
    br x0
.symver __bf_fdimf, fdimf@@LIBC

.balign 4
.globl __bf_hypotf
.type __bf_hypotf, %function
__bf_hypotf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3648]
    cbz x17, 1f
    br x17
1:  mov x0, #456
    bl __bf_resolve_and_call
    br x0
.symver __bf_hypotf, hypotf@@LIBC

.balign 4
.globl __bf_ilogbf
.type __bf_ilogbf, %function
__bf_ilogbf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3656]
    cbz x17, 1f
    br x17
1:  mov x0, #457
    bl __bf_resolve_and_call
    br x0
.symver __bf_ilogbf, ilogbf@@LIBC

.balign 4
.globl __bf_lgammaf
.type __bf_lgammaf, %function
__bf_lgammaf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3664]
    cbz x17, 1f
    br x17
1:  mov x0, #458
    bl __bf_resolve_and_call
    br x0
.symver __bf_lgammaf, lgammaf@@LIBC

.balign 4
.globl __bf_lgammaf_r
.type __bf_lgammaf_r, %function
__bf_lgammaf_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3672]
    cbz x17, 1f
    br x17
1:  mov x0, #459
    bl __bf_resolve_and_call
    br x0
.symver __bf_lgammaf_r, lgammaf_r@@LIBC

.balign 4
.globl __bf_log1pf
.type __bf_log1pf, %function
__bf_log1pf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3680]
    cbz x17, 1f
    br x17
1:  mov x0, #460
    bl __bf_resolve_and_call
    br x0
.symver __bf_log1pf, log1pf@@LIBC

.balign 4
.globl __bf_logbf
.type __bf_logbf, %function
__bf_logbf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3688]
    cbz x17, 1f
    br x17
1:  mov x0, #461
    bl __bf_resolve_and_call
    br x0
.symver __bf_logbf, logbf@@LIBC

.balign 4
.globl __bf_strndup
.type __bf_strndup, %function
__bf_strndup:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3696]
    cbz x17, 1f
    br x17
1:  mov x0, #462
    bl __bf_resolve_and_call
    br x0
.symver __bf_strndup, strndup@@LIBC

.balign 4
.globl __bf_strlcpy
.type __bf_strlcpy, %function
__bf_strlcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3704]
    cbz x17, 1f
    br x17
1:  mov x0, #463
    bl __bf_resolve_and_call
    br x0
.symver __bf_strlcpy, strlcpy@@LIBC

.balign 4
.globl __bf_setpgid
.type __bf_setpgid, %function
__bf_setpgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3712]
    cbz x17, 1f
    br x17
1:  mov x0, #464
    bl __bf_resolve_and_call
    br x0
.symver __bf_setpgid, setpgid@@LIBC

.balign 4
.globl __bf___strlcat_chk
.type __bf___strlcat_chk, %function
__bf___strlcat_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3720]
    cbz x17, 1f
    br x17
1:  mov x0, #465
    bl __bf_resolve_and_call
    br x0
.symver __bf___strlcat_chk, __strlcat_chk@@LIBC

.balign 4
.globl __bf_clock_nanosleep
.type __bf_clock_nanosleep, %function
__bf_clock_nanosleep:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3728]
    cbz x17, 1f
    br x17
1:  mov x0, #466
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_nanosleep, clock_nanosleep@@LIBC

.balign 4
.globl __bf__Unwind_GetCFA
.type __bf__Unwind_GetCFA, %function
__bf__Unwind_GetCFA:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3736]
    cbz x17, 1f
    br x17
1:  mov x0, #467
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetCFA, _Unwind_GetCFA@@LIBC_R

.balign 4
.globl __bf__Unwind_FindEnclosingFunction
.type __bf__Unwind_FindEnclosingFunction, %function
__bf__Unwind_FindEnclosingFunction:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3744]
    cbz x17, 1f
    br x17
1:  mov x0, #468
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_FindEnclosingFunction, _Unwind_FindEnclosingFunction@@LIBC_R

.balign 4
.globl __bf__Unwind_Backtrace
.type __bf__Unwind_Backtrace, %function
__bf__Unwind_Backtrace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3752]
    cbz x17, 1f
    br x17
1:  mov x0, #469
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_Backtrace, _Unwind_Backtrace@@LIBC_R

.balign 4
.globl __bf_asprintf
.type __bf_asprintf, %function
__bf_asprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3760]
    cbz x17, 1f
    br x17
1:  mov x0, #470
    bl __bf_resolve_and_call
    br x0
.symver __bf_asprintf, asprintf@@LIBC

.balign 4
.globl __bf___system_property_area_serial
.type __bf___system_property_area_serial, %function
__bf___system_property_area_serial:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3768]
    cbz x17, 1f
    br x17
1:  mov x0, #471
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_area_serial, __system_property_area_serial@@LIBC

.balign 4
.globl __bf___system_property_serial
.type __bf___system_property_serial, %function
__bf___system_property_serial:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3776]
    cbz x17, 1f
    br x17
1:  mov x0, #472
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_serial, __system_property_serial@@LIBC

.balign 4
.globl __bf___system_property_read
.type __bf___system_property_read, %function
__bf___system_property_read:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3784]
    cbz x17, 1f
    br x17
1:  mov x0, #473
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_read, __system_property_read@@LIBC

.balign 4
.globl __bf_fnmatch
.type __bf_fnmatch, %function
__bf_fnmatch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3792]
    cbz x17, 1f
    br x17
1:  mov x0, #474
    bl __bf_resolve_and_call
    br x0
.symver __bf_fnmatch, fnmatch@@LIBC

.balign 4
.globl __bf_timerfd_gettime
.type __bf_timerfd_gettime, %function
__bf_timerfd_gettime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3800]
    cbz x17, 1f
    br x17
1:  mov x0, #475
    bl __bf_resolve_and_call
    br x0
.symver __bf_timerfd_gettime, timerfd_gettime@@LIBC

.balign 4
.globl __bf___openat_2
.type __bf___openat_2, %function
__bf___openat_2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3808]
    cbz x17, 1f
    br x17
1:  mov x0, #476
    bl __bf_resolve_and_call
    br x0
.symver __bf___openat_2, __openat_2@@LIBC

.balign 4
.globl __bf_android_getaddrinfofornet
.type __bf_android_getaddrinfofornet, %function
__bf_android_getaddrinfofornet:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3816]
    cbz x17, 1f
    br x17
1:  mov x0, #477
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_getaddrinfofornet, android_getaddrinfofornet@@LIBC_Q

.balign 4
.globl __bf___system_property_set
.type __bf___system_property_set, %function
__bf___system_property_set:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3824]
    cbz x17, 1f
    br x17
1:  mov x0, #478
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_set, __system_property_set@@LIBC

.balign 4
.globl __bf_setprogname
.type __bf_setprogname, %function
__bf_setprogname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3832]
    cbz x17, 1f
    br x17
1:  mov x0, #479
    bl __bf_resolve_and_call
    br x0
.symver __bf_setprogname, setprogname@@LIBC

.balign 4
.globl __bf_fallocate
.type __bf_fallocate, %function
__bf_fallocate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3840]
    cbz x17, 1f
    br x17
1:  mov x0, #480
    bl __bf_resolve_and_call
    br x0
.symver __bf_fallocate, fallocate@@LIBC

.balign 4
.globl __bf_lstat64
.type __bf_lstat64, %function
__bf_lstat64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3848]
    cbz x17, 1f
    br x17
1:  mov x0, #481
    bl __bf_resolve_and_call
    br x0
.symver __bf_lstat64, lstat64@@LIBC

.balign 4
.globl __bf_mkstemp
.type __bf_mkstemp, %function
__bf_mkstemp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3856]
    cbz x17, 1f
    br x17
1:  mov x0, #482
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkstemp, mkstemp@@LIBC

.balign 4
.globl __bf_chmod
.type __bf_chmod, %function
__bf_chmod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3864]
    cbz x17, 1f
    br x17
1:  mov x0, #483
    bl __bf_resolve_and_call
    br x0
.symver __bf_chmod, chmod@@LIBC

.balign 4
.globl __bf_statfs64
.type __bf_statfs64, %function
__bf_statfs64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3872]
    cbz x17, 1f
    br x17
1:  mov x0, #484
    bl __bf_resolve_and_call
    br x0
.symver __bf_statfs64, statfs64@@LIBC

.balign 4
.globl __bf_android_dlwarning
.type __bf_android_dlwarning, %function
__bf_android_dlwarning:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3880]
    cbz x17, 1f
    br x17
1:  mov x0, #485
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_dlwarning, android_dlwarning@@LIBDL_ANDROID

.balign 4
.globl __bf_mallopt
.type __bf_mallopt, %function
__bf_mallopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3888]
    cbz x17, 1f
    br x17
1:  mov x0, #486
    bl __bf_resolve_and_call
    br x0
.symver __bf_mallopt, mallopt@@LIBC_O

.balign 4
.globl __bf_malloc_info
.type __bf_malloc_info, %function
__bf_malloc_info:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3896]
    cbz x17, 1f
    br x17
1:  mov x0, #487
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_info, malloc_info@@LIBC

.balign 4
.globl __bf_fdatasync
.type __bf_fdatasync, %function
__bf_fdatasync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3904]
    cbz x17, 1f
    br x17
1:  mov x0, #488
    bl __bf_resolve_and_call
    br x0
.symver __bf_fdatasync, fdatasync@@LIBC

.balign 4
.globl __bf_android_get_application_target_sdk_version
.type __bf_android_get_application_target_sdk_version, %function
__bf_android_get_application_target_sdk_version:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3912]
    cbz x17, 1f
    br x17
1:  mov x0, #489
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_get_application_target_sdk_version, android_get_application_target_sdk_version@@LIBC_N

.balign 4
.globl __bf_getpwnam
.type __bf_getpwnam, %function
__bf_getpwnam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3920]
    cbz x17, 1f
    br x17
1:  mov x0, #490
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpwnam, getpwnam@@LIBC

.balign 4
.globl __bf_getgrnam
.type __bf_getgrnam, %function
__bf_getgrnam:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3928]
    cbz x17, 1f
    br x17
1:  mov x0, #491
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgrnam, getgrnam@@LIBC

.balign 4
.globl __bf___sched_cpucount
.type __bf___sched_cpucount, %function
__bf___sched_cpucount:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3936]
    cbz x17, 1f
    br x17
1:  mov x0, #492
    bl __bf_resolve_and_call
    br x0
.symver __bf___sched_cpucount, __sched_cpucount@@LIBC

.balign 4
.globl __bf_sched_getaffinity
.type __bf_sched_getaffinity, %function
__bf_sched_getaffinity:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3944]
    cbz x17, 1f
    br x17
1:  mov x0, #493
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_getaffinity, sched_getaffinity@@LIBC

.balign 4
.globl __bf_get_nprocs_conf
.type __bf_get_nprocs_conf, %function
__bf_get_nprocs_conf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3952]
    cbz x17, 1f
    br x17
1:  mov x0, #494
    bl __bf_resolve_and_call
    br x0
.symver __bf_get_nprocs_conf, get_nprocs_conf@@LIBC

.balign 4
.globl __bf_setuid
.type __bf_setuid, %function
__bf_setuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3960]
    cbz x17, 1f
    br x17
1:  mov x0, #495
    bl __bf_resolve_and_call
    br x0
.symver __bf_setuid, setuid@@LIBC

.balign 4
.globl __bf_setgid
.type __bf_setgid, %function
__bf_setgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3968]
    cbz x17, 1f
    br x17
1:  mov x0, #496
    bl __bf_resolve_and_call
    br x0
.symver __bf_setgid, setgid@@LIBC

.balign 4
.globl __bf_tgkill
.type __bf_tgkill, %function
__bf_tgkill:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3976]
    cbz x17, 1f
    br x17
1:  mov x0, #497
    bl __bf_resolve_and_call
    br x0
.symver __bf_tgkill, tgkill@@LIBC

.balign 4
.globl __bf_getline
.type __bf_getline, %function
__bf_getline:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3984]
    cbz x17, 1f
    br x17
1:  mov x0, #498
    bl __bf_resolve_and_call
    br x0
.symver __bf_getline, getline@@LIBC

.balign 4
.globl __bf_strtok_r
.type __bf_strtok_r, %function
__bf_strtok_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #3992]
    cbz x17, 1f
    br x17
1:  mov x0, #499
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtok_r, strtok_r@@LIBC

.balign 4
.globl __bf_pidfd_open
.type __bf_pidfd_open, %function
__bf_pidfd_open:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4000]
    cbz x17, 1f
    br x17
1:  mov x0, #500
    bl __bf_resolve_and_call
    br x0
.symver __bf_pidfd_open, pidfd_open@@LIBC_S

.balign 4
.globl __bf_tcflush
.type __bf_tcflush, %function
__bf_tcflush:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4008]
    cbz x17, 1f
    br x17
1:  mov x0, #501
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcflush, tcflush@@LIBC

.balign 4
.globl __bf_tcsendbreak
.type __bf_tcsendbreak, %function
__bf_tcsendbreak:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4016]
    cbz x17, 1f
    br x17
1:  mov x0, #502
    bl __bf_resolve_and_call
    br x0
.symver __bf_tcsendbreak, tcsendbreak@@LIBC

.balign 4
.globl __bf_inotify_init1
.type __bf_inotify_init1, %function
__bf_inotify_init1:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4024]
    cbz x17, 1f
    br x17
1:  mov x0, #503
    bl __bf_resolve_and_call
    br x0
.symver __bf_inotify_init1, inotify_init1@@LIBC

.balign 4
.globl __bf_inotify_add_watch
.type __bf_inotify_add_watch, %function
__bf_inotify_add_watch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4032]
    cbz x17, 1f
    br x17
1:  mov x0, #504
    bl __bf_resolve_and_call
    br x0
.symver __bf_inotify_add_watch, inotify_add_watch@@LIBC

.balign 4
.globl __bf_inotify_rm_watch
.type __bf_inotify_rm_watch, %function
__bf_inotify_rm_watch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4040]
    cbz x17, 1f
    br x17
1:  mov x0, #505
    bl __bf_resolve_and_call
    br x0
.symver __bf_inotify_rm_watch, inotify_rm_watch@@LIBC

.balign 4
.globl __bf_android_fdsan_get_error_level
.type __bf_android_fdsan_get_error_level, %function
__bf_android_fdsan_get_error_level:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4048]
    cbz x17, 1f
    br x17
1:  mov x0, #506
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_get_error_level, android_fdsan_get_error_level@@LIBC_Q

.balign 4
.globl __bf_android_reset_stack_guards
.type __bf_android_reset_stack_guards, %function
__bf_android_reset_stack_guards:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4056]
    cbz x17, 1f
    br x17
1:  mov x0, #507
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_reset_stack_guards, android_reset_stack_guards@@LIBC_S

.balign 4
.globl __bf_android_fdsan_set_error_level
.type __bf_android_fdsan_set_error_level, %function
__bf_android_fdsan_set_error_level:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4064]
    cbz x17, 1f
    br x17
1:  mov x0, #508
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_set_error_level, android_fdsan_set_error_level@@LIBC_Q

.balign 4
.globl __bf_dup3
.type __bf_dup3, %function
__bf_dup3:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4072]
    cbz x17, 1f
    br x17
1:  mov x0, #509
    bl __bf_resolve_and_call
    br x0
.symver __bf_dup3, dup3@@LIBC

.balign 4
.globl __bf_capget
.type __bf_capget, %function
__bf_capget:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4080]
    cbz x17, 1f
    br x17
1:  mov x0, #510
    bl __bf_resolve_and_call
    br x0
.symver __bf_capget, capget@@LIBC

.balign 4
.globl __bf_setresgid
.type __bf_setresgid, %function
__bf_setresgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4088]
    cbz x17, 1f
    br x17
1:  mov x0, #511
    bl __bf_resolve_and_call
    br x0
.symver __bf_setresgid, setresgid@@LIBC

.balign 4
.globl __bf_setresuid
.type __bf_setresuid, %function
__bf_setresuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4096]
    cbz x17, 1f
    br x17
1:  mov x0, #512
    bl __bf_resolve_and_call
    br x0
.symver __bf_setresuid, setresuid@@LIBC

.balign 4
.globl __bf_android_set_16kb_appcompat_mode
.type __bf_android_set_16kb_appcompat_mode, %function
__bf_android_set_16kb_appcompat_mode:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4104]
    cbz x17, 1f
    br x17
1:  mov x0, #513
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_set_16kb_appcompat_mode, android_set_16kb_appcompat_mode@@LIBDL_ANDROID

.balign 4
.globl __bf_capset
.type __bf_capset, %function
__bf_capset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4112]
    cbz x17, 1f
    br x17
1:  mov x0, #514
    bl __bf_resolve_and_call
    br x0
.symver __bf_capset, capset@@LIBC

.balign 4
.globl __bf_mount
.type __bf_mount, %function
__bf_mount:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4120]
    cbz x17, 1f
    br x17
1:  mov x0, #515
    bl __bf_resolve_and_call
    br x0
.symver __bf_mount, mount@@LIBC

.balign 4
.globl __bf_unshare
.type __bf_unshare, %function
__bf_unshare:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4128]
    cbz x17, 1f
    br x17
1:  mov x0, #516
    bl __bf_resolve_and_call
    br x0
.symver __bf_unshare, unshare@@LIBC

.balign 4
.globl __bf___system_properties_zygote_reload
.type __bf___system_properties_zygote_reload, %function
__bf___system_properties_zygote_reload:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4136]
    cbz x17, 1f
    br x17
1:  mov x0, #517
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_properties_zygote_reload, __system_properties_zygote_reload@@LIBC_V

.balign 4
.globl __bf_setgroups
.type __bf_setgroups, %function
__bf_setgroups:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4144]
    cbz x17, 1f
    br x17
1:  mov x0, #518
    bl __bf_resolve_and_call
    br x0
.symver __bf_setgroups, setgroups@@LIBC

.balign 4
.globl __bf_setmntent
.type __bf_setmntent, %function
__bf_setmntent:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4152]
    cbz x17, 1f
    br x17
1:  mov x0, #519
    bl __bf_resolve_and_call
    br x0
.symver __bf_setmntent, setmntent@@LIBC

.balign 4
.globl __bf_getmntent
.type __bf_getmntent, %function
__bf_getmntent:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4160]
    cbz x17, 1f
    br x17
1:  mov x0, #520
    bl __bf_resolve_and_call
    br x0
.symver __bf_getmntent, getmntent@@LIBC

.balign 4
.globl __bf_endmntent
.type __bf_endmntent, %function
__bf_endmntent:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4168]
    cbz x17, 1f
    br x17
1:  mov x0, #521
    bl __bf_resolve_and_call
    br x0
.symver __bf_endmntent, endmntent@@LIBC

.balign 4
.globl __bf_umount2
.type __bf_umount2, %function
__bf_umount2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4176]
    cbz x17, 1f
    br x17
1:  mov x0, #522
    bl __bf_resolve_and_call
    br x0
.symver __bf_umount2, umount2@@LIBC

.balign 4
.globl __bf_statx
.type __bf_statx, %function
__bf_statx:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4184]
    cbz x17, 1f
    br x17
1:  mov x0, #523
    bl __bf_resolve_and_call
    br x0
.symver __bf_statx, statx@@LIBC_R

.balign 4
.globl __bf_memset_explicit
.type __bf_memset_explicit, %function
__bf_memset_explicit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4192]
    cbz x17, 1f
    br x17
1:  mov x0, #524
    bl __bf_resolve_and_call
    br x0
.symver __bf_memset_explicit, memset_explicit@@LIBC_U

.balign 4
.globl __bf_lseek64
.type __bf_lseek64, %function
__bf_lseek64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4200]
    cbz x17, 1f
    br x17
1:  mov x0, #525
    bl __bf_resolve_and_call
    br x0
.symver __bf_lseek64, lseek64@@LIBC

.balign 4
.globl __bf_dirfd
.type __bf_dirfd, %function
__bf_dirfd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4208]
    cbz x17, 1f
    br x17
1:  mov x0, #526
    bl __bf_resolve_and_call
    br x0
.symver __bf_dirfd, dirfd@@LIBC

.balign 4
.globl __bf_malloc_usable_size
.type __bf_malloc_usable_size, %function
__bf_malloc_usable_size:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4216]
    cbz x17, 1f
    br x17
1:  mov x0, #527
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_usable_size, malloc_usable_size@@LIBC

.balign 4
.globl __bf_fstatfs
.type __bf_fstatfs, %function
__bf_fstatfs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4224]
    cbz x17, 1f
    br x17
1:  mov x0, #528
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstatfs, fstatfs@@LIBC

.balign 4
.globl __bf__Unwind_GetTextRelBase
.type __bf__Unwind_GetTextRelBase, %function
__bf__Unwind_GetTextRelBase:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4232]
    cbz x17, 1f
    br x17
1:  mov x0, #529
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetTextRelBase, _Unwind_GetTextRelBase@@LIBC_R

.balign 4
.globl __bf__Unwind_GetDataRelBase
.type __bf__Unwind_GetDataRelBase, %function
__bf__Unwind_GetDataRelBase:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4240]
    cbz x17, 1f
    br x17
1:  mov x0, #530
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetDataRelBase, _Unwind_GetDataRelBase@@LIBC_R

.balign 4
.globl __bf__Unwind_GetIPInfo
.type __bf__Unwind_GetIPInfo, %function
__bf__Unwind_GetIPInfo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4248]
    cbz x17, 1f
    br x17
1:  mov x0, #531
    bl __bf_resolve_and_call
    br x0
.symver __bf__Unwind_GetIPInfo, _Unwind_GetIPInfo@@LIBC_R

.balign 4
.globl __bf_settimeofday
.type __bf_settimeofday, %function
__bf_settimeofday:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4256]
    cbz x17, 1f
    br x17
1:  mov x0, #532
    bl __bf_resolve_and_call
    br x0
.symver __bf_settimeofday, settimeofday@@LIBC

.balign 4
.globl __bf_sigqueue
.type __bf_sigqueue, %function
__bf_sigqueue:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4264]
    cbz x17, 1f
    br x17
1:  mov x0, #533
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigqueue, sigqueue@@LIBC

.balign 4
.globl __bf_eventfd_write
.type __bf_eventfd_write, %function
__bf_eventfd_write:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4272]
    cbz x17, 1f
    br x17
1:  mov x0, #534
    bl __bf_resolve_and_call
    br x0
.symver __bf_eventfd_write, eventfd_write@@LIBC

.balign 4
.globl __bf_process_madvise
.type __bf_process_madvise, %function
__bf_process_madvise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4280]
    cbz x17, 1f
    br x17
1:  mov x0, #535
    bl __bf_resolve_and_call
    br x0
.symver __bf_process_madvise, process_madvise@@LIBC_S

.balign 4
.globl __bf_inotify_init
.type __bf_inotify_init, %function
__bf_inotify_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4288]
    cbz x17, 1f
    br x17
1:  mov x0, #536
    bl __bf_resolve_and_call
    br x0
.symver __bf_inotify_init, inotify_init@@LIBC

.balign 4
.globl __bf_flock
.type __bf_flock, %function
__bf_flock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4296]
    cbz x17, 1f
    br x17
1:  mov x0, #537
    bl __bf_resolve_and_call
    br x0
.symver __bf_flock, flock@@LIBC

.balign 4
.globl __bf_execvp
.type __bf_execvp, %function
__bf_execvp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4304]
    cbz x17, 1f
    br x17
1:  mov x0, #538
    bl __bf_resolve_and_call
    br x0
.symver __bf_execvp, execvp@@LIBC

.balign 4
.globl __bf_putchar
.type __bf_putchar, %function
__bf_putchar:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4312]
    cbz x17, 1f
    br x17
1:  mov x0, #539
    bl __bf_resolve_and_call
    br x0
.symver __bf_putchar, putchar@@LIBC

.balign 4
.globl __bf_dprintf
.type __bf_dprintf, %function
__bf_dprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4320]
    cbz x17, 1f
    br x17
1:  mov x0, #540
    bl __bf_resolve_and_call
    br x0
.symver __bf_dprintf, dprintf@@LIBC

.balign 4
.globl __bf_memfd_create
.type __bf_memfd_create, %function
__bf_memfd_create:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4328]
    cbz x17, 1f
    br x17
1:  mov x0, #541
    bl __bf_resolve_and_call
    br x0
.symver __bf_memfd_create, memfd_create@@LIBC_R

.balign 4
.globl __bf_android_get_exported_namespace
.type __bf_android_get_exported_namespace, %function
__bf_android_get_exported_namespace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4336]
    cbz x17, 1f
    br x17
1:  mov x0, #542
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_get_exported_namespace, android_get_exported_namespace@@LIBDL_ANDROID

.balign 4
.globl __bf_readv
.type __bf_readv, %function
__bf_readv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4344]
    cbz x17, 1f
    br x17
1:  mov x0, #543
    bl __bf_resolve_and_call
    br x0
.symver __bf_readv, readv@@LIBC

.balign 4
.globl __bf_mkdtemp
.type __bf_mkdtemp, %function
__bf_mkdtemp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4352]
    cbz x17, 1f
    br x17
1:  mov x0, #544
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkdtemp, mkdtemp@@LIBC

.balign 4
.globl __bf_pthread_gettid_np
.type __bf_pthread_gettid_np, %function
__bf_pthread_gettid_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4360]
    cbz x17, 1f
    br x17
1:  mov x0, #545
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_gettid_np, pthread_gettid_np@@LIBC

.balign 4
.globl __bf_isnanf
.type __bf_isnanf, %function
__bf_isnanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4368]
    cbz x17, 1f
    br x17
1:  mov x0, #546
    bl __bf_resolve_and_call
    br x0
.symver __bf_isnanf, isnanf@@LIBC

.balign 4
.globl __bf_sched_setaffinity
.type __bf_sched_setaffinity, %function
__bf_sched_setaffinity:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4376]
    cbz x17, 1f
    br x17
1:  mov x0, #547
    bl __bf_resolve_and_call
    br x0
.symver __bf_sched_setaffinity, sched_setaffinity@@LIBC

.balign 4
.globl __bf_pthread_getname_np
.type __bf_pthread_getname_np, %function
__bf_pthread_getname_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4384]
    cbz x17, 1f
    br x17
1:  mov x0, #548
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_getname_np, pthread_getname_np@@LIBC_O

.balign 4
.globl __bf_nftw
.type __bf_nftw, %function
__bf_nftw:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4392]
    cbz x17, 1f
    br x17
1:  mov x0, #549
    bl __bf_resolve_and_call
    br x0
.symver __bf_nftw, nftw@@LIBC

.balign 4
.globl __bf___pwrite_chk
.type __bf___pwrite_chk, %function
__bf___pwrite_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4400]
    cbz x17, 1f
    br x17
1:  mov x0, #550
    bl __bf_resolve_and_call
    br x0
.symver __bf___pwrite_chk, __pwrite_chk@@LIBC_N

.balign 4
.globl __bf_basename
.type __bf_basename, %function
__bf_basename:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4408]
    cbz x17, 1f
    br x17
1:  mov x0, #551
    bl __bf_resolve_and_call
    br x0
.symver __bf_basename, basename@@LIBC

.balign 4
.globl __bf___system_property_wait
.type __bf___system_property_wait, %function
__bf___system_property_wait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4416]
    cbz x17, 1f
    br x17
1:  mov x0, #552
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_wait, __system_property_wait@@LIBC_O

.balign 4
.globl __bf_inet_ntoa
.type __bf_inet_ntoa, %function
__bf_inet_ntoa:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4424]
    cbz x17, 1f
    br x17
1:  mov x0, #553
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_ntoa, inet_ntoa@@LIBC

.balign 4
.globl __bf_pipe2
.type __bf_pipe2, %function
__bf_pipe2:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4432]
    cbz x17, 1f
    br x17
1:  mov x0, #554
    bl __bf_resolve_and_call
    br x0
.symver __bf_pipe2, pipe2@@LIBC

.balign 4
.globl __bf_hypot
.type __bf_hypot, %function
__bf_hypot:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4440]
    cbz x17, 1f
    br x17
1:  mov x0, #555
    bl __bf_resolve_and_call
    br x0
.symver __bf_hypot, hypot@@LIBC

.balign 4
.globl __bf_faccessat
.type __bf_faccessat, %function
__bf_faccessat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4448]
    cbz x17, 1f
    br x17
1:  mov x0, #556
    bl __bf_resolve_and_call
    br x0
.symver __bf_faccessat, faccessat@@LIBC

.balign 4
.globl __bf_fmemopen
.type __bf_fmemopen, %function
__bf_fmemopen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4456]
    cbz x17, 1f
    br x17
1:  mov x0, #557
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmemopen, fmemopen@@LIBC

.balign 4
.globl __bf_dirname
.type __bf_dirname, %function
__bf_dirname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4464]
    cbz x17, 1f
    br x17
1:  mov x0, #558
    bl __bf_resolve_and_call
    br x0
.symver __bf_dirname, dirname@@LIBC

.balign 4
.globl __bf_memccpy
.type __bf_memccpy, %function
__bf_memccpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4472]
    cbz x17, 1f
    br x17
1:  mov x0, #559
    bl __bf_resolve_and_call
    br x0
.symver __bf_memccpy, memccpy@@LIBC

.balign 4
.globl __bf_tsearch
.type __bf_tsearch, %function
__bf_tsearch:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4480]
    cbz x17, 1f
    br x17
1:  mov x0, #560
    bl __bf_resolve_and_call
    br x0
.symver __bf_tsearch, tsearch@@LIBC

.balign 4
.globl __bf_tfind
.type __bf_tfind, %function
__bf_tfind:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4488]
    cbz x17, 1f
    br x17
1:  mov x0, #561
    bl __bf_resolve_and_call
    br x0
.symver __bf_tfind, tfind@@LIBC

.balign 4
.globl __bf_tdelete
.type __bf_tdelete, %function
__bf_tdelete:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4496]
    cbz x17, 1f
    br x17
1:  mov x0, #562
    bl __bf_resolve_and_call
    br x0
.symver __bf_tdelete, tdelete@@LIBC

.balign 4
.globl __bf_tdestroy
.type __bf_tdestroy, %function
__bf_tdestroy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4504]
    cbz x17, 1f
    br x17
1:  mov x0, #563
    bl __bf_resolve_and_call
    br x0
.symver __bf_tdestroy, tdestroy@@LIBC

.balign 4
.globl __bf_android_get_device_api_level
.type __bf_android_get_device_api_level, %function
__bf_android_get_device_api_level:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4512]
    cbz x17, 1f
    br x17
1:  mov x0, #564
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_get_device_api_level, android_get_device_api_level@@LIBC_Q

.balign 4
.globl __bf_chown
.type __bf_chown, %function
__bf_chown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4520]
    cbz x17, 1f
    br x17
1:  mov x0, #565
    bl __bf_resolve_and_call
    br x0
.symver __bf_chown, chown@@LIBC

.balign 4
.globl __bf_perror
.type __bf_perror, %function
__bf_perror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4528]
    cbz x17, 1f
    br x17
1:  mov x0, #566
    bl __bf_resolve_and_call
    br x0
.symver __bf_perror, perror@@LIBC

.balign 4
.globl __bf_clone
.type __bf_clone, %function
__bf_clone:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4536]
    cbz x17, 1f
    br x17
1:  mov x0, #567
    bl __bf_resolve_and_call
    br x0
.symver __bf_clone, clone@@LIBC

.balign 4
.globl __bf_getpwnam_r
.type __bf_getpwnam_r, %function
__bf_getpwnam_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4544]
    cbz x17, 1f
    br x17
1:  mov x0, #568
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpwnam_r, getpwnam_r@@LIBC

.balign 4
.globl __bf_getgrnam_r
.type __bf_getgrnam_r, %function
__bf_getgrnam_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4552]
    cbz x17, 1f
    br x17
1:  mov x0, #569
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgrnam_r, getgrnam_r@@LIBC_N

.balign 4
.globl __bf_signalfd
.type __bf_signalfd, %function
__bf_signalfd:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4560]
    cbz x17, 1f
    br x17
1:  mov x0, #570
    bl __bf_resolve_and_call
    br x0
.symver __bf_signalfd, signalfd@@LIBC

.balign 4
.globl __bf_sigdelset
.type __bf_sigdelset, %function
__bf_sigdelset:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4568]
    cbz x17, 1f
    br x17
1:  mov x0, #571
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigdelset, sigdelset@@LIBC

.balign 4
.globl __bf_sigismember
.type __bf_sigismember, %function
__bf_sigismember:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4576]
    cbz x17, 1f
    br x17
1:  mov x0, #572
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigismember, sigismember@@LIBC

.balign 4
.globl __bf_mkdirat
.type __bf_mkdirat, %function
__bf_mkdirat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4584]
    cbz x17, 1f
    br x17
1:  mov x0, #573
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkdirat, mkdirat@@LIBC

.balign 4
.globl __bf_openat
.type __bf_openat, %function
__bf_openat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4592]
    cbz x17, 1f
    br x17
1:  mov x0, #574
    bl __bf_resolve_and_call
    br x0
.symver __bf_openat, openat@@LIBC

.balign 4
.globl __bf_waitid
.type __bf_waitid, %function
__bf_waitid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4600]
    cbz x17, 1f
    br x17
1:  mov x0, #575
    bl __bf_resolve_and_call
    br x0
.symver __bf_waitid, waitid@@LIBC

.balign 4
.globl __bf_strtold
.type __bf_strtold, %function
__bf_strtold:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4608]
    cbz x17, 1f
    br x17
1:  mov x0, #576
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtold, strtold@@LIBC

.balign 4
.globl __bf_wcstol
.type __bf_wcstol, %function
__bf_wcstol:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4616]
    cbz x17, 1f
    br x17
1:  mov x0, #577
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstol, wcstol@@LIBC

.balign 4
.globl __bf_wcstoul
.type __bf_wcstoul, %function
__bf_wcstoul:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4624]
    cbz x17, 1f
    br x17
1:  mov x0, #578
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstoul, wcstoul@@LIBC

.balign 4
.globl __bf_wcstoll
.type __bf_wcstoll, %function
__bf_wcstoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4632]
    cbz x17, 1f
    br x17
1:  mov x0, #579
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstoll, wcstoll@@LIBC

.balign 4
.globl __bf_wcstoull
.type __bf_wcstoull, %function
__bf_wcstoull:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4640]
    cbz x17, 1f
    br x17
1:  mov x0, #580
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstoull, wcstoull@@LIBC

.balign 4
.globl __bf_wcstof
.type __bf_wcstof, %function
__bf_wcstof:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4648]
    cbz x17, 1f
    br x17
1:  mov x0, #581
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstof, wcstof@@LIBC

.balign 4
.globl __bf_wcstod
.type __bf_wcstod, %function
__bf_wcstod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4656]
    cbz x17, 1f
    br x17
1:  mov x0, #582
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstod, wcstod@@LIBC

.balign 4
.globl __bf_wcstold
.type __bf_wcstold, %function
__bf_wcstold:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4664]
    cbz x17, 1f
    br x17
1:  mov x0, #583
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcstold, wcstold@@LIBC

.balign 4
.globl __bf_swprintf
.type __bf_swprintf, %function
__bf_swprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4672]
    cbz x17, 1f
    br x17
1:  mov x0, #584
    bl __bf_resolve_and_call
    br x0
.symver __bf_swprintf, swprintf@@LIBC

.balign 4
.globl __bf_setlocale
.type __bf_setlocale, %function
__bf_setlocale:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4680]
    cbz x17, 1f
    br x17
1:  mov x0, #585
    bl __bf_resolve_and_call
    br x0
.symver __bf_setlocale, setlocale@@LIBC

.balign 4
.globl __bf_link
.type __bf_link, %function
__bf_link:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4688]
    cbz x17, 1f
    br x17
1:  mov x0, #586
    bl __bf_resolve_and_call
    br x0
.symver __bf_link, link@@LIBC

.balign 4
.globl __bf_pathconf
.type __bf_pathconf, %function
__bf_pathconf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4696]
    cbz x17, 1f
    br x17
1:  mov x0, #587
    bl __bf_resolve_and_call
    br x0
.symver __bf_pathconf, pathconf@@LIBC

.balign 4
.globl __bf_chdir
.type __bf_chdir, %function
__bf_chdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4704]
    cbz x17, 1f
    br x17
1:  mov x0, #588
    bl __bf_resolve_and_call
    br x0
.symver __bf_chdir, chdir@@LIBC

.balign 4
.globl __bf_fchmodat
.type __bf_fchmodat, %function
__bf_fchmodat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4712]
    cbz x17, 1f
    br x17
1:  mov x0, #589
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchmodat, fchmodat@@LIBC

.balign 4
.globl __bf_truncate
.type __bf_truncate, %function
__bf_truncate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4720]
    cbz x17, 1f
    br x17
1:  mov x0, #590
    bl __bf_resolve_and_call
    br x0
.symver __bf_truncate, truncate@@LIBC

.balign 4
.globl __bf_sendfile
.type __bf_sendfile, %function
__bf_sendfile:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4728]
    cbz x17, 1f
    br x17
1:  mov x0, #591
    bl __bf_resolve_and_call
    br x0
.symver __bf_sendfile, sendfile@@LIBC

.balign 4
.globl __bf_fdopendir
.type __bf_fdopendir, %function
__bf_fdopendir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4736]
    cbz x17, 1f
    br x17
1:  mov x0, #592
    bl __bf_resolve_and_call
    br x0
.symver __bf_fdopendir, fdopendir@@LIBC

.balign 4
.globl __bf_unlinkat
.type __bf_unlinkat, %function
__bf_unlinkat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4744]
    cbz x17, 1f
    br x17
1:  mov x0, #593
    bl __bf_resolve_and_call
    br x0
.symver __bf_unlinkat, unlinkat@@LIBC

.balign 4
.globl __bf_fgetxattr
.type __bf_fgetxattr, %function
__bf_fgetxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4752]
    cbz x17, 1f
    br x17
1:  mov x0, #594
    bl __bf_resolve_and_call
    br x0
.symver __bf_fgetxattr, fgetxattr@@LIBC

.balign 4
.globl __bf_getxattr
.type __bf_getxattr, %function
__bf_getxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4760]
    cbz x17, 1f
    br x17
1:  mov x0, #595
    bl __bf_resolve_and_call
    br x0
.symver __bf_getxattr, getxattr@@LIBC

.balign 4
.globl __bf_fremovexattr
.type __bf_fremovexattr, %function
__bf_fremovexattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4768]
    cbz x17, 1f
    br x17
1:  mov x0, #596
    bl __bf_resolve_and_call
    br x0
.symver __bf_fremovexattr, fremovexattr@@LIBC

.balign 4
.globl __bf_fsetxattr
.type __bf_fsetxattr, %function
__bf_fsetxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4776]
    cbz x17, 1f
    br x17
1:  mov x0, #597
    bl __bf_resolve_and_call
    br x0
.symver __bf_fsetxattr, fsetxattr@@LIBC

.balign 4
.globl __bf_removexattr
.type __bf_removexattr, %function
__bf_removexattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4784]
    cbz x17, 1f
    br x17
1:  mov x0, #598
    bl __bf_resolve_and_call
    br x0
.symver __bf_removexattr, removexattr@@LIBC

.balign 4
.globl __bf_setxattr
.type __bf_setxattr, %function
__bf_setxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4792]
    cbz x17, 1f
    br x17
1:  mov x0, #599
    bl __bf_resolve_and_call
    br x0
.symver __bf_setxattr, setxattr@@LIBC

.balign 4
.globl __bf_chroot
.type __bf_chroot, %function
__bf_chroot:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4800]
    cbz x17, 1f
    br x17
1:  mov x0, #600
    bl __bf_resolve_and_call
    br x0
.symver __bf_chroot, chroot@@LIBC

.balign 4
.globl __bf___pwrite64_chk
.type __bf___pwrite64_chk, %function
__bf___pwrite64_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4808]
    cbz x17, 1f
    br x17
1:  mov x0, #601
    bl __bf_resolve_and_call
    br x0
.symver __bf___pwrite64_chk, __pwrite64_chk@@LIBC_N

.balign 4
.globl __bf_fstat64
.type __bf_fstat64, %function
__bf_fstat64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4816]
    cbz x17, 1f
    br x17
1:  mov x0, #602
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstat64, fstat64@@LIBC

.balign 4
.globl __bf_creat
.type __bf_creat, %function
__bf_creat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4824]
    cbz x17, 1f
    br x17
1:  mov x0, #603
    bl __bf_resolve_and_call
    br x0
.symver __bf_creat, creat@@LIBC

.balign 4
.globl __bf_stat64
.type __bf_stat64, %function
__bf_stat64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4832]
    cbz x17, 1f
    br x17
1:  mov x0, #604
    bl __bf_resolve_and_call
    br x0
.symver __bf_stat64, stat64@@LIBC

.balign 4
.globl __bf_iswspace
.type __bf_iswspace, %function
__bf_iswspace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4840]
    cbz x17, 1f
    br x17
1:  mov x0, #605
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswspace, iswspace@@LIBC

.balign 4
.globl __bf_wcschr
.type __bf_wcschr, %function
__bf_wcschr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4848]
    cbz x17, 1f
    br x17
1:  mov x0, #606
    bl __bf_resolve_and_call
    br x0
.symver __bf_wcschr, wcschr@@LIBC

.balign 4
.globl __bf_timegm
.type __bf_timegm, %function
__bf_timegm:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4856]
    cbz x17, 1f
    br x17
1:  mov x0, #607
    bl __bf_resolve_and_call
    br x0
.symver __bf_timegm, timegm@@LIBC

.balign 4
.globl __bf_personality
.type __bf_personality, %function
__bf_personality:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4864]
    cbz x17, 1f
    br x17
1:  mov x0, #608
    bl __bf_resolve_and_call
    br x0
.symver __bf_personality, personality@@LIBC

.balign 4
.globl __bf_ppoll
.type __bf_ppoll, %function
__bf_ppoll:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4872]
    cbz x17, 1f
    br x17
1:  mov x0, #609
    bl __bf_resolve_and_call
    br x0
.symver __bf_ppoll, ppoll@@LIBC

.balign 4
.globl __bf_logb
.type __bf_logb, %function
__bf_logb:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4880]
    cbz x17, 1f
    br x17
1:  mov x0, #610
    bl __bf_resolve_and_call
    br x0
.symver __bf_logb, logb@@LIBC

.balign 4
.globl __bf_scalbn
.type __bf_scalbn, %function
__bf_scalbn:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4888]
    cbz x17, 1f
    br x17
1:  mov x0, #611
    bl __bf_resolve_and_call
    br x0
.symver __bf_scalbn, scalbn@@LIBC

.balign 4
.globl __bf_scalbnf
.type __bf_scalbnf, %function
__bf_scalbnf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4896]
    cbz x17, 1f
    br x17
1:  mov x0, #612
    bl __bf_resolve_and_call
    br x0
.symver __bf_scalbnf, scalbnf@@LIBC

.balign 4
.globl __bf_logbl
.type __bf_logbl, %function
__bf_logbl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4904]
    cbz x17, 1f
    br x17
1:  mov x0, #613
    bl __bf_resolve_and_call
    br x0
.symver __bf_logbl, logbl@@LIBC

.balign 4
.globl __bf_scalbnl
.type __bf_scalbnl, %function
__bf_scalbnl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4912]
    cbz x17, 1f
    br x17
1:  mov x0, #614
    bl __bf_resolve_and_call
    br x0
.symver __bf_scalbnl, scalbnl@@LIBC

.balign 4
.globl __bf_fmaxl
.type __bf_fmaxl, %function
__bf_fmaxl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4920]
    cbz x17, 1f
    br x17
1:  mov x0, #615
    bl __bf_resolve_and_call
    br x0
.symver __bf_fmaxl, fmaxl@@LIBC

.balign 4
.globl __bf_putc
.type __bf_putc, %function
__bf_putc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4928]
    cbz x17, 1f
    br x17
1:  mov x0, #616
    bl __bf_resolve_and_call
    br x0
.symver __bf_putc, putc@@LIBC

.balign 4
.globl __bf_rewinddir
.type __bf_rewinddir, %function
__bf_rewinddir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4936]
    cbz x17, 1f
    br x17
1:  mov x0, #617
    bl __bf_resolve_and_call
    br x0
.symver __bf_rewinddir, rewinddir@@LIBC

.balign 4
.globl __bf_lockf
.type __bf_lockf, %function
__bf_lockf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4944]
    cbz x17, 1f
    br x17
1:  mov x0, #618
    bl __bf_resolve_and_call
    br x0
.symver __bf_lockf, lockf@@LIBC_N

.balign 4
.globl __bf___umask_chk
.type __bf___umask_chk, %function
__bf___umask_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4952]
    cbz x17, 1f
    br x17
1:  mov x0, #619
    bl __bf_resolve_and_call
    br x0
.symver __bf___umask_chk, __umask_chk@@LIBC

.balign 4
.globl __bf_getservbyname
.type __bf_getservbyname, %function
__bf_getservbyname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4960]
    cbz x17, 1f
    br x17
1:  mov x0, #620
    bl __bf_resolve_and_call
    br x0
.symver __bf_getservbyname, getservbyname@@LIBC

.balign 4
.globl __bf_regcomp
.type __bf_regcomp, %function
__bf_regcomp:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4968]
    cbz x17, 1f
    br x17
1:  mov x0, #621
    bl __bf_resolve_and_call
    br x0
.symver __bf_regcomp, regcomp@@LIBC

.balign 4
.globl __bf_regerror
.type __bf_regerror, %function
__bf_regerror:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4976]
    cbz x17, 1f
    br x17
1:  mov x0, #622
    bl __bf_resolve_and_call
    br x0
.symver __bf_regerror, regerror@@LIBC

.balign 4
.globl __bf_regexec
.type __bf_regexec, %function
__bf_regexec:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4984]
    cbz x17, 1f
    br x17
1:  mov x0, #623
    bl __bf_resolve_and_call
    br x0
.symver __bf_regexec, regexec@@LIBC

.balign 4
.globl __bf_regfree
.type __bf_regfree, %function
__bf_regfree:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #4992]
    cbz x17, 1f
    br x17
1:  mov x0, #624
    bl __bf_resolve_and_call
    br x0
.symver __bf_regfree, regfree@@LIBC

.balign 4
.globl __bf___fwrite_chk
.type __bf___fwrite_chk, %function
__bf___fwrite_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5000]
    cbz x17, 1f
    br x17
1:  mov x0, #625
    bl __bf_resolve_and_call
    br x0
.symver __bf___fwrite_chk, __fwrite_chk@@LIBC_N

.balign 4
.globl __bf_getifaddrs
.type __bf_getifaddrs, %function
__bf_getifaddrs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5008]
    cbz x17, 1f
    br x17
1:  mov x0, #626
    bl __bf_resolve_and_call
    br x0
.symver __bf_getifaddrs, getifaddrs@@LIBC_N

.balign 4
.globl __bf_freeifaddrs
.type __bf_freeifaddrs, %function
__bf_freeifaddrs:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5016]
    cbz x17, 1f
    br x17
1:  mov x0, #627
    bl __bf_resolve_and_call
    br x0
.symver __bf_freeifaddrs, freeifaddrs@@LIBC_N

.balign 4
.globl __bf_getpwuid_r
.type __bf_getpwuid_r, %function
__bf_getpwuid_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5024]
    cbz x17, 1f
    br x17
1:  mov x0, #628
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpwuid_r, getpwuid_r@@LIBC

.balign 4
.globl __bf_android_fdsan_get_owner_tag
.type __bf_android_fdsan_get_owner_tag, %function
__bf_android_fdsan_get_owner_tag:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5032]
    cbz x17, 1f
    br x17
1:  mov x0, #629
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_fdsan_get_owner_tag, android_fdsan_get_owner_tag@@LIBC_Q

.balign 4
.globl __bf___system_property_foreach
.type __bf___system_property_foreach, %function
__bf___system_property_foreach:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5040]
    cbz x17, 1f
    br x17
1:  mov x0, #630
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_property_foreach, __system_property_foreach@@LIBC

.balign 4
.globl __bf_strtoimax
.type __bf_strtoimax, %function
__bf_strtoimax:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5048]
    cbz x17, 1f
    br x17
1:  mov x0, #631
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtoimax, strtoimax@@LIBC

.balign 4
.globl __bf_fstatat
.type __bf_fstatat, %function
__bf_fstatat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5056]
    cbz x17, 1f
    br x17
1:  mov x0, #632
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstatat, fstatat@@LIBC

.balign 4
.globl __bf_readdir_r
.type __bf_readdir_r, %function
__bf_readdir_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5064]
    cbz x17, 1f
    br x17
1:  mov x0, #633
    bl __bf_resolve_and_call
    br x0
.symver __bf_readdir_r, readdir_r@@LIBC

.balign 4
.globl __bf_mknod
.type __bf_mknod, %function
__bf_mknod:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5072]
    cbz x17, 1f
    br x17
1:  mov x0, #634
    bl __bf_resolve_and_call
    br x0
.symver __bf_mknod, mknod@@LIBC

.balign 4
.globl __bf_open_memstream
.type __bf_open_memstream, %function
__bf_open_memstream:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5080]
    cbz x17, 1f
    br x17
1:  mov x0, #635
    bl __bf_resolve_and_call
    br x0
.symver __bf_open_memstream, open_memstream@@LIBC

.balign 4
.globl __bf_vfork
.type __bf_vfork, %function
__bf_vfork:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5088]
    cbz x17, 1f
    br x17
1:  mov x0, #636
    bl __bf_resolve_and_call
    br x0
.symver __bf_vfork, vfork@@LIBC

.balign 4
.globl __bf_getprotobynumber
.type __bf_getprotobynumber, %function
__bf_getprotobynumber:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5096]
    cbz x17, 1f
    br x17
1:  mov x0, #637
    bl __bf_resolve_and_call
    br x0
.symver __bf_getprotobynumber, getprotobynumber@@LIBC

.balign 4
.globl __bf___fgets_chk
.type __bf___fgets_chk, %function
__bf___fgets_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5104]
    cbz x17, 1f
    br x17
1:  mov x0, #638
    bl __bf_resolve_and_call
    br x0
.symver __bf___fgets_chk, __fgets_chk@@LIBC

.balign 4
.globl __bf_strtok
.type __bf_strtok, %function
__bf_strtok:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5112]
    cbz x17, 1f
    br x17
1:  mov x0, #639
    bl __bf_resolve_and_call
    br x0
.symver __bf_strtok, strtok@@LIBC

.balign 4
.globl __bf_ctime
.type __bf_ctime, %function
__bf_ctime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5120]
    cbz x17, 1f
    br x17
1:  mov x0, #640
    bl __bf_resolve_and_call
    br x0
.symver __bf_ctime, ctime@@LIBC

.balign 4
.globl __bf_scandir
.type __bf_scandir, %function
__bf_scandir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5128]
    cbz x17, 1f
    br x17
1:  mov x0, #641
    bl __bf_resolve_and_call
    br x0
.symver __bf_scandir, scandir@@LIBC

.balign 4
.globl __bf_alphasort
.type __bf_alphasort, %function
__bf_alphasort:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5136]
    cbz x17, 1f
    br x17
1:  mov x0, #642
    bl __bf_resolve_and_call
    br x0
.symver __bf_alphasort, alphasort@@LIBC

.balign 4
.globl __bf_rewind
.type __bf_rewind, %function
__bf_rewind:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5144]
    cbz x17, 1f
    br x17
1:  mov x0, #643
    bl __bf_resolve_and_call
    br x0
.symver __bf_rewind, rewind@@LIBC

.balign 4
.globl __bf_hasmntopt
.type __bf_hasmntopt, %function
__bf_hasmntopt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5152]
    cbz x17, 1f
    br x17
1:  mov x0, #644
    bl __bf_resolve_and_call
    br x0
.symver __bf_hasmntopt, hasmntopt@@LIBC_O

.balign 4
.globl __bf___pread64_chk
.type __bf___pread64_chk, %function
__bf___pread64_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5160]
    cbz x17, 1f
    br x17
1:  mov x0, #645
    bl __bf_resolve_and_call
    br x0
.symver __bf___pread64_chk, __pread64_chk@@LIBC

.balign 4
.globl __bf_posix_fadvise
.type __bf_posix_fadvise, %function
__bf_posix_fadvise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5168]
    cbz x17, 1f
    br x17
1:  mov x0, #646
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_fadvise, posix_fadvise@@LIBC

.balign 4
.globl __bf_ftruncate64
.type __bf_ftruncate64, %function
__bf_ftruncate64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5176]
    cbz x17, 1f
    br x17
1:  mov x0, #647
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftruncate64, ftruncate64@@LIBC

.balign 4
.globl __bf_process_vm_readv
.type __bf_process_vm_readv, %function
__bf_process_vm_readv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5184]
    cbz x17, 1f
    br x17
1:  mov x0, #648
    bl __bf_resolve_and_call
    br x0
.symver __bf_process_vm_readv, process_vm_readv@@LIBC

.balign 4
.globl __bf_umount
.type __bf_umount, %function
__bf_umount:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5192]
    cbz x17, 1f
    br x17
1:  mov x0, #649
    bl __bf_resolve_and_call
    br x0
.symver __bf_umount, umount@@LIBC

.balign 4
.globl __bf_swapon
.type __bf_swapon, %function
__bf_swapon:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5200]
    cbz x17, 1f
    br x17
1:  mov x0, #650
    bl __bf_resolve_and_call
    br x0
.symver __bf_swapon, swapon@@LIBC

.balign 4
.globl __bf_system
.type __bf_system, %function
__bf_system:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5208]
    cbz x17, 1f
    br x17
1:  mov x0, #651
    bl __bf_resolve_and_call
    br x0
.symver __bf_system, system@@LIBC

.balign 4
.globl __bf_fstatfs64
.type __bf_fstatfs64, %function
__bf_fstatfs64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5216]
    cbz x17, 1f
    br x17
1:  mov x0, #652
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstatfs64, fstatfs64@@LIBC

.balign 4
.globl __bf_sync
.type __bf_sync, %function
__bf_sync:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5224]
    cbz x17, 1f
    br x17
1:  mov x0, #653
    bl __bf_resolve_and_call
    br x0
.symver __bf_sync, sync@@LIBC

.balign 4
.globl __bf_posix_openpt
.type __bf_posix_openpt, %function
__bf_posix_openpt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5232]
    cbz x17, 1f
    br x17
1:  mov x0, #654
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_openpt, posix_openpt@@LIBC

.balign 4
.globl __bf_grantpt
.type __bf_grantpt, %function
__bf_grantpt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5240]
    cbz x17, 1f
    br x17
1:  mov x0, #655
    bl __bf_resolve_and_call
    br x0
.symver __bf_grantpt, grantpt@@LIBC

.balign 4
.globl __bf_unlockpt
.type __bf_unlockpt, %function
__bf_unlockpt:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5248]
    cbz x17, 1f
    br x17
1:  mov x0, #656
    bl __bf_resolve_and_call
    br x0
.symver __bf_unlockpt, unlockpt@@LIBC

.balign 4
.globl __bf_ptsname_r
.type __bf_ptsname_r, %function
__bf_ptsname_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5256]
    cbz x17, 1f
    br x17
1:  mov x0, #657
    bl __bf_resolve_and_call
    br x0
.symver __bf_ptsname_r, ptsname_r@@LIBC

.balign 4
.globl __bf_setsid
.type __bf_setsid, %function
__bf_setsid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5264]
    cbz x17, 1f
    br x17
1:  mov x0, #658
    bl __bf_resolve_and_call
    br x0
.symver __bf_setsid, setsid@@LIBC

.balign 4
.globl __bf_android_link_namespaces
.type __bf_android_link_namespaces, %function
__bf_android_link_namespaces:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5272]
    cbz x17, 1f
    br x17
1:  mov x0, #659
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_link_namespaces, android_link_namespaces@@LIBDL_ANDROID

.balign 4
.globl __bf_android_create_namespace
.type __bf_android_create_namespace, %function
__bf_android_create_namespace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5280]
    cbz x17, 1f
    br x17
1:  mov x0, #660
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_create_namespace, android_create_namespace@@LIBDL_ANDROID

.balign 4
.globl __bf_eventfd_read
.type __bf_eventfd_read, %function
__bf_eventfd_read:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5288]
    cbz x17, 1f
    br x17
1:  mov x0, #661
    bl __bf_resolve_and_call
    br x0
.symver __bf_eventfd_read, eventfd_read@@LIBC

.balign 4
.globl __bf_sem_clockwait
.type __bf_sem_clockwait, %function
__bf_sem_clockwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5296]
    cbz x17, 1f
    br x17
1:  mov x0, #662
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_clockwait, sem_clockwait@@LIBC_R

.balign 4
.globl __bf_asinh
.type __bf_asinh, %function
__bf_asinh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5304]
    cbz x17, 1f
    br x17
1:  mov x0, #663
    bl __bf_resolve_and_call
    br x0
.symver __bf_asinh, asinh@@LIBC

.balign 4
.globl __bf_acosh
.type __bf_acosh, %function
__bf_acosh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5312]
    cbz x17, 1f
    br x17
1:  mov x0, #664
    bl __bf_resolve_and_call
    br x0
.symver __bf_acosh, acosh@@LIBC

.balign 4
.globl __bf_atanh
.type __bf_atanh, %function
__bf_atanh:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5320]
    cbz x17, 1f
    br x17
1:  mov x0, #665
    bl __bf_resolve_and_call
    br x0
.symver __bf_atanh, atanh@@LIBC

.balign 4
.globl __bf_remainder
.type __bf_remainder, %function
__bf_remainder:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5328]
    cbz x17, 1f
    br x17
1:  mov x0, #666
    bl __bf_resolve_and_call
    br x0
.symver __bf_remainder, remainder@@LIBC

.balign 4
.globl __bf_getrandom
.type __bf_getrandom, %function
__bf_getrandom:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5336]
    cbz x17, 1f
    br x17
1:  mov x0, #667
    bl __bf_resolve_and_call
    br x0
.symver __bf_getrandom, getrandom@@LIBC_P

.balign 4
.globl __bf_erf
.type __bf_erf, %function
__bf_erf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5344]
    cbz x17, 1f
    br x17
1:  mov x0, #668
    bl __bf_resolve_and_call
    br x0
.symver __bf_erf, erf@@LIBC

.balign 4
.globl __bf_asctime
.type __bf_asctime, %function
__bf_asctime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5352]
    cbz x17, 1f
    br x17
1:  mov x0, #669
    bl __bf_resolve_and_call
    br x0
.symver __bf_asctime, asctime@@LIBC

.balign 4
.globl __bf_getprotobyname
.type __bf_getprotobyname, %function
__bf_getprotobyname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5360]
    cbz x17, 1f
    br x17
1:  mov x0, #670
    bl __bf_resolve_and_call
    br x0
.symver __bf_getprotobyname, getprotobyname@@LIBC

.balign 4
.globl __bf_setns
.type __bf_setns, %function
__bf_setns:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5368]
    cbz x17, 1f
    br x17
1:  mov x0, #671
    bl __bf_resolve_and_call
    br x0
.symver __bf_setns, setns@@LIBC

.balign 4
.globl __bf___gnu_basename
.type __bf___gnu_basename, %function
__bf___gnu_basename:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5376]
    cbz x17, 1f
    br x17
1:  mov x0, #672
    bl __bf_resolve_and_call
    br x0
.symver __bf___gnu_basename, __gnu_basename@@LIBC

.balign 4
.globl __bf_fgetc
.type __bf_fgetc, %function
__bf_fgetc:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5384]
    cbz x17, 1f
    br x17
1:  mov x0, #673
    bl __bf_resolve_and_call
    br x0
.symver __bf_fgetc, fgetc@@LIBC

.balign 4
.globl __bf_glob
.type __bf_glob, %function
__bf_glob:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5392]
    cbz x17, 1f
    br x17
1:  mov x0, #674
    bl __bf_resolve_and_call
    br x0
.symver __bf_glob, glob@@LIBC_P

.balign 4
.globl __bf_globfree
.type __bf_globfree, %function
__bf_globfree:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5400]
    cbz x17, 1f
    br x17
1:  mov x0, #675
    bl __bf_resolve_and_call
    br x0
.symver __bf_globfree, globfree@@LIBC_P

.balign 4
.globl __bf_getgroups
.type __bf_getgroups, %function
__bf_getgroups:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5408]
    cbz x17, 1f
    br x17
1:  mov x0, #676
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgroups, getgroups@@LIBC

.balign 4
.globl __bf_getopt_long_only
.type __bf_getopt_long_only, %function
__bf_getopt_long_only:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5416]
    cbz x17, 1f
    br x17
1:  mov x0, #677
    bl __bf_resolve_and_call
    br x0
.symver __bf_getopt_long_only, getopt_long_only@@LIBC

.balign 4
.globl __bf_inet_aton
.type __bf_inet_aton, %function
__bf_inet_aton:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5424]
    cbz x17, 1f
    br x17
1:  mov x0, #678
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_aton, inet_aton@@LIBC

.balign 4
.globl __bf_strcasestr
.type __bf_strcasestr, %function
__bf_strcasestr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5432]
    cbz x17, 1f
    br x17
1:  mov x0, #679
    bl __bf_resolve_and_call
    br x0
.symver __bf_strcasestr, strcasestr@@LIBC

.balign 4
.globl __bf_inet_addr
.type __bf_inet_addr, %function
__bf_inet_addr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5440]
    cbz x17, 1f
    br x17
1:  mov x0, #680
    bl __bf_resolve_and_call
    br x0
.symver __bf_inet_addr, inet_addr@@LIBC

.balign 4
.globl __bf_pthread_setschedprio
.type __bf_pthread_setschedprio, %function
__bf_pthread_setschedprio:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5448]
    cbz x17, 1f
    br x17
1:  mov x0, #681
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_setschedprio, pthread_setschedprio@@LIBC_P

.balign 4
.globl __bf_pwrite64
.type __bf_pwrite64, %function
__bf_pwrite64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5456]
    cbz x17, 1f
    br x17
1:  mov x0, #682
    bl __bf_resolve_and_call
    br x0
.symver __bf_pwrite64, pwrite64@@LIBC

.balign 4
.globl __bf_mmap64
.type __bf_mmap64, %function
__bf_mmap64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5464]
    cbz x17, 1f
    br x17
1:  mov x0, #683
    bl __bf_resolve_and_call
    br x0
.symver __bf_mmap64, mmap64@@LIBC

.balign 4
.globl __bf_munlock
.type __bf_munlock, %function
__bf_munlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5472]
    cbz x17, 1f
    br x17
1:  mov x0, #684
    bl __bf_resolve_and_call
    br x0
.symver __bf_munlock, munlock@@LIBC

.balign 4
.globl __bf_malloc_backtrace
.type __bf_malloc_backtrace, %function
__bf_malloc_backtrace:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5480]
    cbz x17, 1f
    br x17
1:  mov x0, #685
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_backtrace, malloc_backtrace@@LIBC_Q

.balign 4
.globl __bf_malloc_disable
.type __bf_malloc_disable, %function
__bf_malloc_disable:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5488]
    cbz x17, 1f
    br x17
1:  mov x0, #686
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_disable, malloc_disable@@LIBC_Q

.balign 4
.globl __bf_malloc_enable
.type __bf_malloc_enable, %function
__bf_malloc_enable:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5496]
    cbz x17, 1f
    br x17
1:  mov x0, #687
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_enable, malloc_enable@@LIBC_Q

.balign 4
.globl __bf_malloc_iterate
.type __bf_malloc_iterate, %function
__bf_malloc_iterate:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5504]
    cbz x17, 1f
    br x17
1:  mov x0, #688
    bl __bf_resolve_and_call
    br x0
.symver __bf_malloc_iterate, malloc_iterate@@LIBC_Q

.balign 4
.globl __bf_sethostname
.type __bf_sethostname, %function
__bf_sethostname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5512]
    cbz x17, 1f
    br x17
1:  mov x0, #689
    bl __bf_resolve_and_call
    br x0
.symver __bf_sethostname, sethostname@@LIBC

.balign 4
.globl __bf_mknodat
.type __bf_mknodat, %function
__bf_mknodat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5520]
    cbz x17, 1f
    br x17
1:  mov x0, #690
    bl __bf_resolve_and_call
    br x0
.symver __bf_mknodat, mknodat@@LIBC

.balign 4
.globl __bf_symlinkat
.type __bf_symlinkat, %function
__bf_symlinkat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5528]
    cbz x17, 1f
    br x17
1:  mov x0, #691
    bl __bf_resolve_and_call
    br x0
.symver __bf_symlinkat, symlinkat@@LIBC

.balign 4
.globl __bf_fchdir
.type __bf_fchdir, %function
__bf_fchdir:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5536]
    cbz x17, 1f
    br x17
1:  mov x0, #692
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchdir, fchdir@@LIBC

.balign 4
.globl __bf_initgroups
.type __bf_initgroups, %function
__bf_initgroups:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5544]
    cbz x17, 1f
    br x17
1:  mov x0, #693
    bl __bf_resolve_and_call
    br x0
.symver __bf_initgroups, initgroups@@LIBC

.balign 4
.globl __bf_sigtimedwait
.type __bf_sigtimedwait, %function
__bf_sigtimedwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5552]
    cbz x17, 1f
    br x17
1:  mov x0, #694
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigtimedwait, sigtimedwait@@LIBC

.balign 4
.globl __bf___libc_current_sigrtmax
.type __bf___libc_current_sigrtmax, %function
__bf___libc_current_sigrtmax:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5560]
    cbz x17, 1f
    br x17
1:  mov x0, #695
    bl __bf_resolve_and_call
    br x0
.symver __bf___libc_current_sigrtmax, __libc_current_sigrtmax@@LIBC

.balign 4
.globl __bf_fexecve
.type __bf_fexecve, %function
__bf_fexecve:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5568]
    cbz x17, 1f
    br x17
1:  mov x0, #696
    bl __bf_resolve_and_call
    br x0
.symver __bf_fexecve, fexecve@@LIBC_P

.balign 4
.globl __bf___libc_current_sigrtmin
.type __bf___libc_current_sigrtmin, %function
__bf___libc_current_sigrtmin:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5576]
    cbz x17, 1f
    br x17
1:  mov x0, #697
    bl __bf_resolve_and_call
    br x0
.symver __bf___libc_current_sigrtmin, __libc_current_sigrtmin@@LIBC

.balign 4
.globl __bf_prlimit
.type __bf_prlimit, %function
__bf_prlimit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5584]
    cbz x17, 1f
    br x17
1:  mov x0, #698
    bl __bf_resolve_and_call
    br x0
.symver __bf_prlimit, prlimit@@LIBC

.balign 4
.globl __bf_vsyslog
.type __bf_vsyslog, %function
__bf_vsyslog:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5592]
    cbz x17, 1f
    br x17
1:  mov x0, #699
    bl __bf_resolve_and_call
    br x0
.symver __bf_vsyslog, vsyslog@@LIBC

.balign 4
.globl __bf_vdprintf
.type __bf_vdprintf, %function
__bf_vdprintf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5600]
    cbz x17, 1f
    br x17
1:  mov x0, #700
    bl __bf_resolve_and_call
    br x0
.symver __bf_vdprintf, vdprintf@@LIBC

.balign 4
.globl __bf_posix_madvise
.type __bf_posix_madvise, %function
__bf_posix_madvise:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5608]
    cbz x17, 1f
    br x17
1:  mov x0, #701
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_madvise, posix_madvise@@LIBC

.balign 4
.globl __bf_android_set_application_target_sdk_version
.type __bf_android_set_application_target_sdk_version, %function
__bf_android_set_application_target_sdk_version:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5616]
    cbz x17, 1f
    br x17
1:  mov x0, #702
    bl __bf_resolve_and_call
    br x0
.symver __bf_android_set_application_target_sdk_version, android_set_application_target_sdk_version@@LIBDL_ANDROID

.balign 4
.globl __bf_res_mkquery
.type __bf_res_mkquery, %function
__bf_res_mkquery:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5624]
    cbz x17, 1f
    br x17
1:  mov x0, #703
    bl __bf_resolve_and_call
    br x0
.symver __bf_res_mkquery, res_mkquery@@LIBC

.balign 4
.globl __bf___b64_ntop
.type __bf___b64_ntop, %function
__bf___b64_ntop:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5632]
    cbz x17, 1f
    br x17
1:  mov x0, #704
    bl __bf_resolve_and_call
    br x0
.symver __bf___b64_ntop, __b64_ntop@@LIBC

.balign 4
.globl __bf_vfscanf
.type __bf_vfscanf, %function
__bf_vfscanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5640]
    cbz x17, 1f
    br x17
1:  mov x0, #705
    bl __bf_resolve_and_call
    br x0
.symver __bf_vfscanf, vfscanf@@LIBC

.balign 4
.globl __bf_clock_getcpuclockid
.type __bf_clock_getcpuclockid, %function
__bf_clock_getcpuclockid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5648]
    cbz x17, 1f
    br x17
1:  mov x0, #706
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_getcpuclockid, clock_getcpuclockid@@LIBC

.balign 4
.globl __bf_clock_getres
.type __bf_clock_getres, %function
__bf_clock_getres:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5656]
    cbz x17, 1f
    br x17
1:  mov x0, #707
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_getres, clock_getres@@LIBC

.balign 4
.globl __bf_clock_settime
.type __bf_clock_settime, %function
__bf_clock_settime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5664]
    cbz x17, 1f
    br x17
1:  mov x0, #708
    bl __bf_resolve_and_call
    br x0
.symver __bf_clock_settime, clock_settime@@LIBC

.balign 4
.globl __bf_getgrgid_r
.type __bf_getgrgid_r, %function
__bf_getgrgid_r:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5672]
    cbz x17, 1f
    br x17
1:  mov x0, #709
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgrgid_r, getgrgid_r@@LIBC_N

.balign 4
.globl __bf_sigwait
.type __bf_sigwait, %function
__bf_sigwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5680]
    cbz x17, 1f
    br x17
1:  mov x0, #710
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigwait, sigwait@@LIBC

.balign 4
.globl __bf_sigsuspend
.type __bf_sigsuspend, %function
__bf_sigsuspend:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5688]
    cbz x17, 1f
    br x17
1:  mov x0, #711
    bl __bf_resolve_and_call
    br x0
.symver __bf_sigsuspend, sigsuspend@@LIBC

.balign 4
.globl __bf_getgrouplist
.type __bf_getgrouplist, %function
__bf_getgrouplist:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5696]
    cbz x17, 1f
    br x17
1:  mov x0, #712
    bl __bf_resolve_and_call
    br x0
.symver __bf_getgrouplist, getgrouplist@@LIBC

.balign 4
.globl __bf_daemon
.type __bf_daemon, %function
__bf_daemon:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5704]
    cbz x17, 1f
    br x17
1:  mov x0, #713
    bl __bf_resolve_and_call
    br x0
.symver __bf_daemon, daemon@@LIBC

.balign 4
.globl __bf_setfsgid
.type __bf_setfsgid, %function
__bf_setfsgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5712]
    cbz x17, 1f
    br x17
1:  mov x0, #714
    bl __bf_resolve_and_call
    br x0
.symver __bf_setfsgid, setfsgid@@LIBC

.balign 4
.globl __bf_setfsuid
.type __bf_setfsuid, %function
__bf_setfsuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5720]
    cbz x17, 1f
    br x17
1:  mov x0, #715
    bl __bf_resolve_and_call
    br x0
.symver __bf_setfsuid, setfsuid@@LIBC

.balign 4
.globl __bf_process_vm_writev
.type __bf_process_vm_writev, %function
__bf_process_vm_writev:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5728]
    cbz x17, 1f
    br x17
1:  mov x0, #716
    bl __bf_resolve_and_call
    br x0
.symver __bf_process_vm_writev, process_vm_writev@@LIBC

.balign 4
.globl __bf_munlockall
.type __bf_munlockall, %function
__bf_munlockall:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5736]
    cbz x17, 1f
    br x17
1:  mov x0, #717
    bl __bf_resolve_and_call
    br x0
.symver __bf_munlockall, munlockall@@LIBC

.balign 4
.globl __bf_mlockall
.type __bf_mlockall, %function
__bf_mlockall:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5744]
    cbz x17, 1f
    br x17
1:  mov x0, #718
    bl __bf_resolve_and_call
    br x0
.symver __bf_mlockall, mlockall@@LIBC

.balign 4
.globl __bf_umask
.type __bf_umask, %function
__bf_umask:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5752]
    cbz x17, 1f
    br x17
1:  mov x0, #719
    bl __bf_resolve_and_call
    br x0
.symver __bf_umask, umask@@LIBC

.balign 4
.globl __bf_recv
.type __bf_recv, %function
__bf_recv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5760]
    cbz x17, 1f
    br x17
1:  mov x0, #720
    bl __bf_resolve_and_call
    br x0
.symver __bf_recv, recv@@LIBC

.balign 4
.globl __bf_send
.type __bf_send, %function
__bf_send:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5768]
    cbz x17, 1f
    br x17
1:  mov x0, #721
    bl __bf_resolve_and_call
    br x0
.symver __bf_send, send@@LIBC

.balign 4
.globl __bf_cfsetspeed
.type __bf_cfsetspeed, %function
__bf_cfsetspeed:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5776]
    cbz x17, 1f
    br x17
1:  mov x0, #722
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfsetspeed, cfsetspeed@@LIBC

.balign 4
.globl __bf_cfgetispeed
.type __bf_cfgetispeed, %function
__bf_cfgetispeed:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5784]
    cbz x17, 1f
    br x17
1:  mov x0, #723
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfgetispeed, cfgetispeed@@LIBC

.balign 4
.globl __bf_cfgetospeed
.type __bf_cfgetospeed, %function
__bf_cfgetospeed:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5792]
    cbz x17, 1f
    br x17
1:  mov x0, #724
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfgetospeed, cfgetospeed@@LIBC

.balign 4
.globl __bf_cfsetispeed
.type __bf_cfsetispeed, %function
__bf_cfsetispeed:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5800]
    cbz x17, 1f
    br x17
1:  mov x0, #725
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfsetispeed, cfsetispeed@@LIBC

.balign 4
.globl __bf_cfsetospeed
.type __bf_cfsetospeed, %function
__bf_cfsetospeed:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5808]
    cbz x17, 1f
    br x17
1:  mov x0, #726
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfsetospeed, cfsetospeed@@LIBC

.balign 4
.globl __bf_cfmakeraw
.type __bf_cfmakeraw, %function
__bf_cfmakeraw:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5816]
    cbz x17, 1f
    br x17
1:  mov x0, #727
    bl __bf_resolve_and_call
    br x0
.symver __bf_cfmakeraw, cfmakeraw@@LIBC

.balign 4
.globl __bf_getnetbyname
.type __bf_getnetbyname, %function
__bf_getnetbyname:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5824]
    cbz x17, 1f
    br x17
1:  mov x0, #728
    bl __bf_resolve_and_call
    br x0
.symver __bf_getnetbyname, getnetbyname@@LIBC

.balign 4
.globl __bf_iswdigit
.type __bf_iswdigit, %function
__bf_iswdigit:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5832]
    cbz x17, 1f
    br x17
1:  mov x0, #729
    bl __bf_resolve_and_call
    br x0
.symver __bf_iswdigit, iswdigit@@LIBC

.balign 4
.globl __bf_wait4
.type __bf_wait4, %function
__bf_wait4:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5840]
    cbz x17, 1f
    br x17
1:  mov x0, #730
    bl __bf_resolve_and_call
    br x0
.symver __bf_wait4, wait4@@LIBC

.balign 4
.globl __bf_getpgid
.type __bf_getpgid, %function
__bf_getpgid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5848]
    cbz x17, 1f
    br x17
1:  mov x0, #731
    bl __bf_resolve_and_call
    br x0
.symver __bf_getpgid, getpgid@@LIBC

.balign 4
.globl __bf_lchown
.type __bf_lchown, %function
__bf_lchown:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5856]
    cbz x17, 1f
    br x17
1:  mov x0, #732
    bl __bf_resolve_and_call
    br x0
.symver __bf_lchown, lchown@@LIBC

.balign 4
.globl __bf_nanf
.type __bf_nanf, %function
__bf_nanf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5864]
    cbz x17, 1f
    br x17
1:  mov x0, #733
    bl __bf_resolve_and_call
    br x0
.symver __bf_nanf, nanf@@LIBC

.balign 4
.globl __bf_strptime
.type __bf_strptime, %function
__bf_strptime:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5872]
    cbz x17, 1f
    br x17
1:  mov x0, #734
    bl __bf_resolve_and_call
    br x0
.symver __bf_strptime, strptime@@LIBC

.balign 4
.globl __bf_sem_timedwait
.type __bf_sem_timedwait, %function
__bf_sem_timedwait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5880]
    cbz x17, 1f
    br x17
1:  mov x0, #735
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_timedwait, sem_timedwait@@LIBC

.balign 4
.globl __bf_frexpl
.type __bf_frexpl, %function
__bf_frexpl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5888]
    cbz x17, 1f
    br x17
1:  mov x0, #736
    bl __bf_resolve_and_call
    br x0
.symver __bf_frexpl, frexpl@@LIBC

.balign 4
.globl __bf_ldexpl
.type __bf_ldexpl, %function
__bf_ldexpl:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5896]
    cbz x17, 1f
    br x17
1:  mov x0, #737
    bl __bf_resolve_and_call
    br x0
.symver __bf_ldexpl, ldexpl@@LIBC

.balign 4
.globl __bf___recvfrom_chk
.type __bf___recvfrom_chk, %function
__bf___recvfrom_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5904]
    cbz x17, 1f
    br x17
1:  mov x0, #738
    bl __bf_resolve_and_call
    br x0
.symver __bf___recvfrom_chk, __recvfrom_chk@@LIBC

.balign 4
.globl __bf_readdir64
.type __bf_readdir64, %function
__bf_readdir64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5912]
    cbz x17, 1f
    br x17
1:  mov x0, #739
    bl __bf_resolve_and_call
    br x0
.symver __bf_readdir64, readdir64@@LIBC

.balign 4
.globl __bf_readlinkat
.type __bf_readlinkat, %function
__bf_readlinkat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5920]
    cbz x17, 1f
    br x17
1:  mov x0, #740
    bl __bf_resolve_and_call
    br x0
.symver __bf_readlinkat, readlinkat@@LIBC

.balign 4
.globl __bf_flistxattr
.type __bf_flistxattr, %function
__bf_flistxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5928]
    cbz x17, 1f
    br x17
1:  mov x0, #741
    bl __bf_resolve_and_call
    br x0
.symver __bf_flistxattr, flistxattr@@LIBC

.balign 4
.globl __bf_llistxattr
.type __bf_llistxattr, %function
__bf_llistxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5936]
    cbz x17, 1f
    br x17
1:  mov x0, #742
    bl __bf_resolve_and_call
    br x0
.symver __bf_llistxattr, llistxattr@@LIBC

.balign 4
.globl __bf_lremovexattr
.type __bf_lremovexattr, %function
__bf_lremovexattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5944]
    cbz x17, 1f
    br x17
1:  mov x0, #743
    bl __bf_resolve_and_call
    br x0
.symver __bf_lremovexattr, lremovexattr@@LIBC

.balign 4
.globl __bf_linkat
.type __bf_linkat, %function
__bf_linkat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5952]
    cbz x17, 1f
    br x17
1:  mov x0, #744
    bl __bf_resolve_and_call
    br x0
.symver __bf_linkat, linkat@@LIBC

.balign 4
.globl __bf_openat64
.type __bf_openat64, %function
__bf_openat64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5960]
    cbz x17, 1f
    br x17
1:  mov x0, #745
    bl __bf_resolve_and_call
    br x0
.symver __bf_openat64, openat64@@LIBC

.balign 4
.globl __bf_fstatat64
.type __bf_fstatat64, %function
__bf_fstatat64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5968]
    cbz x17, 1f
    br x17
1:  mov x0, #746
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstatat64, fstatat64@@LIBC

.balign 4
.globl __bf_fchownat
.type __bf_fchownat, %function
__bf_fchownat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5976]
    cbz x17, 1f
    br x17
1:  mov x0, #747
    bl __bf_resolve_and_call
    br x0
.symver __bf_fchownat, fchownat@@LIBC

.balign 4
.globl __bf_posix_fadvise64
.type __bf_posix_fadvise64, %function
__bf_posix_fadvise64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5984]
    cbz x17, 1f
    br x17
1:  mov x0, #748
    bl __bf_resolve_and_call
    br x0
.symver __bf_posix_fadvise64, posix_fadvise64@@LIBC

.balign 4
.globl __bf_fstatvfs64
.type __bf_fstatvfs64, %function
__bf_fstatvfs64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #5992]
    cbz x17, 1f
    br x17
1:  mov x0, #749
    bl __bf_resolve_and_call
    br x0
.symver __bf_fstatvfs64, fstatvfs64@@LIBC

.balign 4
.globl __bf_renameat
.type __bf_renameat, %function
__bf_renameat:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6000]
    cbz x17, 1f
    br x17
1:  mov x0, #750
    bl __bf_resolve_and_call
    br x0
.symver __bf_renameat, renameat@@LIBC

.balign 4
.globl __bf_fallocate64
.type __bf_fallocate64, %function
__bf_fallocate64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6008]
    cbz x17, 1f
    br x17
1:  mov x0, #751
    bl __bf_resolve_and_call
    br x0
.symver __bf_fallocate64, fallocate64@@LIBC

.balign 4
.globl __bf_lgetxattr
.type __bf_lgetxattr, %function
__bf_lgetxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6016]
    cbz x17, 1f
    br x17
1:  mov x0, #752
    bl __bf_resolve_and_call
    br x0
.symver __bf_lgetxattr, lgetxattr@@LIBC

.balign 4
.globl __bf_listxattr
.type __bf_listxattr, %function
__bf_listxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6024]
    cbz x17, 1f
    br x17
1:  mov x0, #753
    bl __bf_resolve_and_call
    br x0
.symver __bf_listxattr, listxattr@@LIBC

.balign 4
.globl __bf_lsetxattr
.type __bf_lsetxattr, %function
__bf_lsetxattr:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6032]
    cbz x17, 1f
    br x17
1:  mov x0, #754
    bl __bf_resolve_and_call
    br x0
.symver __bf_lsetxattr, lsetxattr@@LIBC

.balign 4
.globl __bf_pause
.type __bf_pause, %function
__bf_pause:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6040]
    cbz x17, 1f
    br x17
1:  mov x0, #755
    bl __bf_resolve_and_call
    br x0
.symver __bf_pause, pause@@LIBC

.balign 4
.globl __bf___stpcpy_chk
.type __bf___stpcpy_chk, %function
__bf___stpcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6048]
    cbz x17, 1f
    br x17
1:  mov x0, #756
    bl __bf_resolve_and_call
    br x0
.symver __bf___stpcpy_chk, __stpcpy_chk@@LIBC

.balign 4
.globl __bf___fsetlocking
.type __bf___fsetlocking, %function
__bf___fsetlocking:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6056]
    cbz x17, 1f
    br x17
1:  mov x0, #757
    bl __bf_resolve_and_call
    br x0
.symver __bf___fsetlocking, __fsetlocking@@LIBC

.balign 4
.globl __bf___mempcpy_chk
.type __bf___mempcpy_chk, %function
__bf___mempcpy_chk:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6064]
    cbz x17, 1f
    br x17
1:  mov x0, #758
    bl __bf_resolve_and_call
    br x0
.symver __bf___mempcpy_chk, __mempcpy_chk@@LIBC_R

.balign 4
.globl __bf_reallocarray
.type __bf_reallocarray, %function
__bf_reallocarray:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6072]
    cbz x17, 1f
    br x17
1:  mov x0, #759
    bl __bf_resolve_and_call
    br x0
.symver __bf_reallocarray, reallocarray@@LIBC_Q

.balign 4
.globl __bf___system_properties_init
.type __bf___system_properties_init, %function
__bf___system_properties_init:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6080]
    cbz x17, 1f
    br x17
1:  mov x0, #760
    bl __bf_resolve_and_call
    br x0
.symver __bf___system_properties_init, __system_properties_init@@LIBC_Q

.balign 4
.globl __bf_fts_open
.type __bf_fts_open, %function
__bf_fts_open:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6088]
    cbz x17, 1f
    br x17
1:  mov x0, #761
    bl __bf_resolve_and_call
    br x0
.symver __bf_fts_open, fts_open@@LIBC

.balign 4
.globl __bf_fts_read
.type __bf_fts_read, %function
__bf_fts_read:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6096]
    cbz x17, 1f
    br x17
1:  mov x0, #762
    bl __bf_resolve_and_call
    br x0
.symver __bf_fts_read, fts_read@@LIBC

.balign 4
.globl __bf_fts_set
.type __bf_fts_set, %function
__bf_fts_set:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6104]
    cbz x17, 1f
    br x17
1:  mov x0, #763
    bl __bf_resolve_and_call
    br x0
.symver __bf_fts_set, fts_set@@LIBC

.balign 4
.globl __bf_fts_close
.type __bf_fts_close, %function
__bf_fts_close:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6112]
    cbz x17, 1f
    br x17
1:  mov x0, #764
    bl __bf_resolve_and_call
    br x0
.symver __bf_fts_close, fts_close@@LIBC

.balign 4
.globl __bf_stpcpy
.type __bf_stpcpy, %function
__bf_stpcpy:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6120]
    cbz x17, 1f
    br x17
1:  mov x0, #765
    bl __bf_resolve_and_call
    br x0
.symver __bf_stpcpy, stpcpy@@LIBC

.balign 4
.globl __bf_popen
.type __bf_popen, %function
__bf_popen:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6128]
    cbz x17, 1f
    br x17
1:  mov x0, #766
    bl __bf_resolve_and_call
    br x0
.symver __bf_popen, popen@@LIBC

.balign 4
.globl __bf_pclose
.type __bf_pclose, %function
__bf_pclose:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6136]
    cbz x17, 1f
    br x17
1:  mov x0, #767
    bl __bf_resolve_and_call
    br x0
.symver __bf_pclose, pclose@@LIBC

.balign 4
.globl __bf_pwritev
.type __bf_pwritev, %function
__bf_pwritev:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6144]
    cbz x17, 1f
    br x17
1:  mov x0, #768
    bl __bf_resolve_and_call
    br x0
.symver __bf_pwritev, pwritev@@LIBC_N

.balign 4
.globl __bf_fpathconf
.type __bf_fpathconf, %function
__bf_fpathconf:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6152]
    cbz x17, 1f
    br x17
1:  mov x0, #769
    bl __bf_resolve_and_call
    br x0
.symver __bf_fpathconf, fpathconf@@LIBC

.balign 4
.globl __bf_memmem
.type __bf_memmem, %function
__bf_memmem:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6160]
    cbz x17, 1f
    br x17
1:  mov x0, #770
    bl __bf_resolve_and_call
    br x0
.symver __bf_memmem, memmem@@LIBC

.balign 4
.globl __bf_strchrnul
.type __bf_strchrnul, %function
__bf_strchrnul:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6168]
    cbz x17, 1f
    br x17
1:  mov x0, #771
    bl __bf_resolve_and_call
    br x0
.symver __bf_strchrnul, strchrnul@@LIBC_N

.balign 4
.globl __bf_open64
.type __bf_open64, %function
__bf_open64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6176]
    cbz x17, 1f
    br x17
1:  mov x0, #772
    bl __bf_resolve_and_call
    br x0
.symver __bf_open64, open64@@LIBC

.balign 4
.globl __bf_mkfifo
.type __bf_mkfifo, %function
__bf_mkfifo:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6184]
    cbz x17, 1f
    br x17
1:  mov x0, #773
    bl __bf_resolve_and_call
    br x0
.symver __bf_mkfifo, mkfifo@@LIBC

.balign 4
.globl __bf_preadv
.type __bf_preadv, %function
__bf_preadv:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6192]
    cbz x17, 1f
    br x17
1:  mov x0, #774
    bl __bf_resolve_and_call
    br x0
.symver __bf_preadv, preadv@@LIBC_N

.balign 4
.globl __bf_splice
.type __bf_splice, %function
__bf_splice:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6200]
    cbz x17, 1f
    br x17
1:  mov x0, #775
    bl __bf_resolve_and_call
    br x0
.symver __bf_splice, splice@@LIBC

.balign 4
.globl __bf_copy_file_range
.type __bf_copy_file_range, %function
__bf_copy_file_range:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6208]
    cbz x17, 1f
    br x17
1:  mov x0, #776
    bl __bf_resolve_and_call
    br x0
.symver __bf_copy_file_range, copy_file_range@@LIBC_U

.balign 4
.globl __bf_fseeko64
.type __bf_fseeko64, %function
__bf_fseeko64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6216]
    cbz x17, 1f
    br x17
1:  mov x0, #777
    bl __bf_resolve_and_call
    br x0
.symver __bf_fseeko64, fseeko64@@LIBC_N

.balign 4
.globl __bf_ftello64
.type __bf_ftello64, %function
__bf_ftello64:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6224]
    cbz x17, 1f
    br x17
1:  mov x0, #778
    bl __bf_resolve_and_call
    br x0
.symver __bf_ftello64, ftello64@@LIBC_N

.balign 4
.globl __bf_sem_trywait
.type __bf_sem_trywait, %function
__bf_sem_trywait:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6232]
    cbz x17, 1f
    br x17
1:  mov x0, #779
    bl __bf_resolve_and_call
    br x0
.symver __bf_sem_trywait, sem_trywait@@LIBC

.balign 4
.globl __bf_pthread_mutex_timedlock
.type __bf_pthread_mutex_timedlock, %function
__bf_pthread_mutex_timedlock:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6240]
    cbz x17, 1f
    br x17
1:  mov x0, #780
    bl __bf_resolve_and_call
    br x0
.symver __bf_pthread_mutex_timedlock, pthread_mutex_timedlock@@LIBC

.balign 4
.globl __bf_strerrorname_np
.type __bf_strerrorname_np, %function
__bf_strerrorname_np:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6248]
    cbz x17, 1f
    br x17
1:  mov x0, #781
    bl __bf_resolve_and_call
    br x0
.symver __bf_strerrorname_np, strerrorname_np@@LIBC_V

.balign 4
.globl __bf_clone3
.type __bf_clone3, %function
__bf_clone3:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6256]
    cbz x17, 1f
    br x17
1:  mov x0, #782
    bl __bf_resolve_and_call
    br x0
.symver __bf_clone3, clone3@@LIBC

.balign 4
.globl __bf_setregid
.type __bf_setregid, %function
__bf_setregid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6264]
    cbz x17, 1f
    br x17
1:  mov x0, #783
    bl __bf_resolve_and_call
    br x0
.symver __bf_setregid, setregid@@LIBC

.balign 4
.globl __bf_setreuid
.type __bf_setreuid, %function
__bf_setreuid:
    adrp x16, :got:__bf_tramp_table
    ldr x16, [x16, #:got_lo12:__bf_tramp_table]
    ldr x17, [x16, #6272]
    cbz x17, 1f
    br x17
1:  mov x0, #784
    bl __bf_resolve_and_call
    br x0
.symver __bf_setreuid, setreuid@@LIBC

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

.balign 8
.globl __bf_data_signgam
.type __bf_data_signgam, %object
.size __bf_data_signgam, 8
__bf_data_signgam:
    .dword 0
.symver __bf_data_signgam, signgam@@LIBC
