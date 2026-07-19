// Wraps glibc symbols with LIBC version so libroblox.so can load
// Compile: aarch64-linux-gnu-gcc -shared -fPIC -o libbionic_shim.so bionic_shim.c \
//   -Wl,--version-script=bionic_shim.ver -nostartfiles -lc

// List of @LIBC versioned symbols libroblox.so needs
// Each one: __asm__(".symver glibc_foo,foo@@LIBC");

// Standard libc functions with LIBC version
__asm__(".symver abort,abort@@LIBC");
__asm__(".symver accept,accept@@LIBC");
__asm__(".symver accept4,accept4@@LIBC");
__asm__(".symver access,access@@LIBC");
__asm__(".symver acos,acos@@LIBC");
__asm__(".symver acosf,acosf@@LIBC");
__asm__(".symver free,free@@LIBC");
__asm__(".symver malloc,malloc@@LIBC");
__asm__(".symver calloc,calloc@@LIBC");
__asm__(".symver realloc,realloc@@LIBC");
__asm__(".symver memcpy,memcpy@@LIBC");
__asm__(".symver memset,memset@@LIBC");
__asm__(".symver strlen,strlen@@LIBC");
__asm__(".symver strcmp,strcmp@@LIBC");
__asm__(".symver strncmp,strncmp@@LIBC");
__asm__(".symver memmove,memmove@@LIBC");
__asm__(".symver memcmp,memcmp@@LIBC");
__asm__(".symver close,close@@LIBC");
__asm__(".symver read,read@@LIBC");
__asm__(".symver write,write@@LIBC");
__asm__(".symver open,open@@LIBC");
__asm__(".symver stat,stat@@LIBC");
__asm__(".symver fstat,fstat@@LIBC");
__asm__(".symver lseek,lseek@@LIBC");
__asm__(".symver mmap,mmap@@LIBC");
__asm__(".symver munmap,munmap@@LIBC");
__asm__(".symver mprotect,mprotect@@LIBC");
__asm__(".symver socket,socket@@LIBC");
__asm__(".symver connect,connect@@LIBC");
__asm__(".symver bind,bind@@LIBC");
__asm__(".symver listen,listen@@LIBC");
__asm__(".symver getaddrinfo,getaddrinfo@@LIBC");
__asm__(".symver freeaddrinfo,freeaddrinfo@@LIBC");
__asm__(".symver gettimeofday,gettimeofday@@LIBC");
__asm__(".symver clock_gettime,clock_gettime@@LIBC");
__asm__(".symver nanosleep,nanosleep@@LIBC");
__asm__(".symver usleep,usleep@@LIBC");
__asm__(".symver dlopen,dlopen@@LIBC");
__asm__(".symver dlsym,dlsym@@LIBC");
__asm__(".symver dlclose,dlclose@@LIBC");
__asm__(".symver dlerror,dlerror@@LIBC");
__asm__(".symver pthread_create,pthread_create@@LIBC");
__asm__(".symver pthread_exit,pthread_exit@@LIBC");
__asm__(".symver pthread_join,pthread_join@@LIBC");
__asm__(".symver pthread_mutex_lock,pthread_mutex_lock@@LIBC");
__asm__(".symver pthread_mutex_unlock,pthread_mutex_unlock@@LIBC");
__asm__(".symver pthread_mutex_init,pthread_mutex_init@@LIBC");
__asm__(".symver pthread_mutex_destroy,pthread_mutex_destroy@@LIBC");
__asm__(".symver pthread_cond_wait,pthread_cond_wait@@LIBC");
__asm__(".symver pthread_cond_signal,pthread_cond_signal@@LIBC");
__asm__(".symver pthread_cond_broadcast,pthread_cond_broadcast@@LIBC");
__asm__(".symver pthread_once,pthread_once@@LIBC");
__asm__(".symver pthread_key_create,pthread_key_create@@LIBC");
__asm__(".symver pthread_getspecific,pthread_getspecific@@LIBC");
__asm__(".symver pthread_setspecific,pthread_setspecific@@LIBC");
// Add more as needed - run the auto-loop to find them all

// Weak stubs to satisfy the linker
void __attribute__((weak)) abort(void) { extern void _exit(int); _exit(1); }
void __attribute__((weak)) _exit(int s) { for(;;) asm volatile(""); }
