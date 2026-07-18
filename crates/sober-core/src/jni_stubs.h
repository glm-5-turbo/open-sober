// Auto-generated stubs for Android/Bionic symbols needed by libroblox.so
// These provide minimal implementations so the library can load under QEMU.

#ifndef JNI_STUBS_H
#define JNI_STUBS_H

#include <stddef.h>
#include <stdint.h>

// Android asset manager stubs
typedef void AAssetManager;
typedef void AAsset;
AAsset* AAssetManager_open(AAssetManager* mgr, const char* filename, int mode) { return NULL; }
void AAsset_close(AAsset* asset) {}
const void* AAsset_getBuffer(AAsset* asset) { return NULL; }
off_t AAsset_getLength(AAsset* asset) { return 0; }
AAssetManager* AAssetManager_fromJava(void* env, void* instance) { return NULL; }

// Android native window stubs
typedef void ANativeWindow;
void ANativeWindow_release(ANativeWindow* win) {}
ANativeWindow* ANativeWindow_fromSurface(void* env, void* surface) { return NULL; }

// Android logging
int __android_log_print(int prio, const char* tag, const char* fmt, ...) { return 0; }

// Additional Bionic C++/libc symbols not in ARM64 glibc
int __register_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void), void *dso_handle) { return 0; }
int __cxa_guard_acquire(void *g) { return 0; }
void __cxa_guard_release(void *g) {}

#endif // JNI_STUBS_H