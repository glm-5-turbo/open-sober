#!/usr/bin/env bash
# Build version bridges for libc.so, libm.so, libdl.so
# These provide LIBC_* version definitions that GSI libraries expect.
#
# Usage: build_bridges.sh <sysroot_lib64_dir>
#
# The sysroot lib64 directory must already contain:
#   libc_glibc.so  (copy of ARM64 glibc libc.so.6)
#   libm_glibc.so  (copy of ARM64 glibc libm.so.6)
#   libdl.so.2     (ARM64 glibc libdl, or empty for nostdlib)

set -euo pipefail

SYSROOT="${1:?Usage: $0 <sysroot_lib64_dir>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VER_SCRIPT="$SCRIPT_DIR/bridge_version.ver"

if [ ! -f "$VER_SCRIPT" ]; then
    echo "Error: bridge_version.ver not found alongside $0"
    exit 1
fi

CROSS="aarch64-linux-gnu-gcc"
CROSS_CC="${CROSS}"

build_libc() {
    echo "  libc.so ..."
    # Need to copy/rename glibc libc so we can link against it
    if [ ! -f "$SYSROOT/libc_glibc.so" ]; then
        if [ -f "$SYSROOT/libglibc.so" ]; then
            cp "$SYSROOT/libglibc.so" "$SYSROOT/libc_glibc.so"
        fi
    fi

    if [ -f "$SYSROOT/libc_glibc.so" ]; then
        # Use --whole-archive to pull all glibc symbols into libc.so
        # and tag them with the LIBC version via the version script.
        $CROSS_CC -shared -fPIC -o "$SYSROOT/libc.so" \
            "$SCRIPT_DIR/bridge_libc.c" \
            -Wl,--version-script,"$VER_SCRIPT" \
            -Wl,-soname,libc.so \
            -Wl,--whole-archive "$SYSROOT/libc_glibc.so" \
            -Wl,--no-whole-archive \
            -lc -lm -ldl 2>&1
    else
        echo "  WARNING: libc_glibc.so not found, building nostdlib"
        $CROSS_CC -shared -fPIC -o "$SYSROOT/libc.so" \
            "$SCRIPT_DIR/bridge_libc.c" \
            -Wl,--version-script,"$VER_SCRIPT" \
            -Wl,-soname,libc.so \
            -nostdlib 2>&1
    fi
}

build_libm() {
    echo "  libm.so ..."
    if [ ! -f "$SYSROOT/libm_glibc.so" ]; then
        if [ -f "$SYSROOT/libm.so.6" ]; then
            cp "$SYSROOT/libm.so.6" "$SYSROOT/libm_glibc.so"
        fi
    fi

    if [ -f "$SYSROOT/libm_glibc.so" ]; then
        $CROSS_CC -shared -fPIC -o "$SYSROOT/libm.so" \
            "$SCRIPT_DIR/bridge_libm.c" \
            -Wl,--version-script,"$VER_SCRIPT" \
            -Wl,-soname,libm.so \
            -L "$SYSROOT" -lm_glibc 2>&1
    else
        echo "  WARNING: libm_glibc.so not found, building nostdlib"
        $CROSS_CC -shared -fPIC -o "$SYSROOT/libm.so" \
            "$SCRIPT_DIR/bridge_libm.c" \
            -Wl,--version-script,"$VER_SCRIPT" \
            -Wl,-soname,libm.so \
            -nostdlib 2>&1
    fi
}

build_libdl() {
    echo "  libdl.so ..."
    $CROSS_CC -shared -fPIC -o "$SYSROOT/libdl.so" \
        "$SCRIPT_DIR/bridge_libdl.c" \
        -Wl,--version-script,"$VER_SCRIPT" \
        -Wl,-soname,libdl.so \
        -nostdlib 2>&1
}

echo "Building version bridges in: $SYSROOT"
build_libc
build_libm
build_libdl
echo "Done. Verify with: readelf -V $SYSROOT/libc.so | grep LIBC"