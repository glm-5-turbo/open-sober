// Minimal JNI shim to load libroblox.so and call JNI_OnLoad.
// This bypasses the need for a full Android Java runtime.
// Compile: gcc -shared -fPIC -o libjni_shim.so jni_shim.c -ldl

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>

// Minimal JNI types
typedef int jint;
typedef void* jobject;
typedef void* jclass;
typedef void* jstring;
typedef void* jmethodID;
typedef void* jfieldID;
typedef void* jvalue;
typedef unsigned char jboolean;
typedef short jchar;

#define JNI_TRUE 1
#define JNI_FALSE 0
#define JNI_OK 0
#define JNI_ERR (-1)
#define JNI_VERSION_1_6 0x00010006
#define JNI_COMMIT 1
#define JNI_ABORT 2

// JNI function table (minimal - just what Roblox needs)
typedef struct JavaVM_ JavaVM;

typedef struct JNINativeInterface_ {
    void* reserved0[4];
    jint (*GetVersion)(JavaVM* vm);
    jclass (*FindClass)(JavaVM* env, const char* name);
    // ... many more functions
    void* reserved[200]; // placeholder for the rest
} JNINativeInterface;

typedef struct JNIEnv_ {
    const struct JNINativeInterface_* functions;
    void* reserved[4];
} JNIEnv;

typedef struct JavaVMOption_ {
    char* optionString;
    void* extraInfo;
} JavaVMOption;

typedef struct JavaVMInitArgs_ {
    jint version;
    jint nOptions;
    JavaVMOption* options;
    jboolean ignoreUnrecognized;
} JavaVMInitArgs;

typedef struct JavaVM_ {
    const struct JNIInvokeInterface_* functions;
    void* reserved[4];
} JavaVM;

typedef struct JNIInvokeInterface_ {
    void* reserved0[3];
    jint (*DestroyJavaVM)(JavaVM*);
    jint (*AttachCurrentThread)(JavaVM*, JNIEnv**, void*);
    jint (*DetachCurrentThread)(JavaVM*);
    jint (*GetEnv)(JavaVM*, void**, jint);
    jint (*AttachCurrentThreadAsDaemon)(JavaVM*, JNIEnv**, void*);
} JNIInvokeInterface;

// Thread-local JNIEnv
static __thread JNIEnv tls_env;
static JavaVM g_vm;
static JNIInvokeInterface g_vm_functions;

// Stub FindClass - returns NULL (game will need to handle)
static jclass stub_FindClass(JNIEnv* env, const char* name) {
    fprintf(stderr, "[jni_shim] FindClass: %s (returning NULL)\n", name);
    return NULL;
}

// Stub GetVersion
static jint stub_GetVersion(JNIEnv* env) {
    return JNI_VERSION_1_6;
}

// Stub RegisterNatives
static jint stub_RegisterNatives(JNIEnv* env, jclass clazz, const void* methods, jint nMethods) {
    fprintf(stderr, "[jni_shim] RegisterNatives: %d methods\n", nMethods);
    return JNI_OK;
}

// Initialize the JNI function table with stubs
static JNINativeInterface g_env_functions = {0};

__attribute__((visibility("default")))
__attribute__((constructor))
static void init_jni_functions() {
    // Set up the minimal function table
    g_env_functions.reserved0[0] = NULL; // reserved
    g_env_functions.reserved0[1] = NULL;
    g_env_functions.reserved0[2] = NULL;
    g_env_functions.reserved0[3] = NULL;
    g_env_functions.GetVersion = (void*)stub_GetVersion;
    g_env_functions.FindClass = (void*)stub_FindClass;
    // ... rest remain NULL (stubs)

    // Set up VM
    g_vm_functions.DestroyJavaVM = NULL;
    g_vm_functions.AttachCurrentThread = NULL;
    g_vm_functions.DetachCurrentThread = NULL;
    g_vm_functions.GetEnv = NULL;
    g_vm_functions.AttachCurrentThreadAsDaemon = NULL;
    g_vm.functions = &g_vm_functions;
}

// JNI_OnLoad - called when libroblox.so is loaded
// This is the entry point the game expects
jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    fprintf(stderr, "[jni_shim] JNI_OnLoad called\n");
    // Return the JNI version we support
    return JNI_VERSION_1_6;
}

// Main function - loads libroblox.so and calls JNI_OnLoad
int main(int argc, char** argv) {
    const char* lib_path = getenv("ROBLOX_LIB");
    if (!lib_path) lib_path = "libroblox.so";

    fprintf(stderr, "[jni_shim] Loading %s...\n", lib_path);

    void* handle = dlopen(lib_path, RTLD_NOW | RTLD_GLOBAL);
    if (!handle) {
        fprintf(stderr, "[jni_shim] Failed to load %s: %s\n", lib_path, dlerror());
        return 1;
    }

    fprintf(stderr, "[jni_shim] Loaded %s successfully\n", lib_path);

    // Call JNI_OnLoad
    typedef jint (*jni_onload_t)(JavaVM*, void*);
    jni_onload_t jni_onload = (jni_onload_t)dlsym(handle, "JNI_OnLoad");
    if (jni_onload) {
        jint version = jni_onload(&g_vm, NULL);
        fprintf(stderr, "[jni_shim] JNI_OnLoad returned version 0x%x\n", version);
    } else {
        fprintf(stderr, "[jni_shim] No JNI_OnLoad symbol found in %s\n", lib_path);
    }

    // Keep running until signal
    fprintf(stderr, "[jni_shim] Entering main loop...\n");

    // Call the game's main if it exists
    typedef int (*main_func_t)(int, char**);
    main_func_t game_main = (main_func_t)dlsym(handle, "main");
    if (game_main) {
        fprintf(stderr, "[jni_shim] Calling game main()\n");
        return game_main(argc, argv);
    }

    // Otherwise just wait
    while (1) sleep(1);

    dlclose(handle);
    return 0;
}