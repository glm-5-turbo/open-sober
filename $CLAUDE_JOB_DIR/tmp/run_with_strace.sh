#!/bin/bash
# Run with strace to see what's blocking
ANDROID_ROOT=~/.cache/open-sober/android-env
SYSROOT="$ANDROID_ROOT"

cd /tmp

timeout 30 strace -f -e trace=process,futex,clock_nanosleep,nanosleep,write \
  /home/code-agent/.cache/open-sober/qemu-patched \
  -L "$SYSROOT" \
  -E LD_LIBRARY_PATH=/system/lib64 \
  -E LD_PRELOAD=/system/lib64/libbionic_shim.so \
  -E ROBLOX_LIB=/system/lib64/libroblox.so \
  "$SYSROOT/jni_shim" \
  2>&1 | head -500