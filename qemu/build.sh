#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Build the Open Sober SMC-patched QEMU user-mode emulator.
#
# Why: Roblox's ARM64 Android binary self-modifies code pages (canary
# patching, JIT regions).  Stock QEMU uses chained (goto_tb) TBs whose
# native jump targets go stale after a page is translated once and then
# rewritten, causing host SIGSEGV.  This build:
#   1. Forces CF_NO_GOTO_TB on every TB (never emit goto_tb jumps), and
#   2. NOPs out tb_set_jmp_target (never patch target addresses late).
#
# Usage:
#   ./build.sh                 # build into ./build, install to ./out
#   ./build.sh /path/to/qemu-10.2.1.tar.xz   # use a local tarball
#
# Outputs:
#   ./out/qemu-aarch64         # the patched user-mode emulator

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QEMU_VERSION="10.2.1"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/open-sober-qemu.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# Fetch the tarball if not provided
if [ "$#" -ge 1 ] && [ -f "$1" ]; then
    TARBALL="$(realpath "$1")"
else
    TARBALL="$WORK/qemu-$QEMU_VERSION.tar.xz"
    echo ">> Downloading QEMU $QEMU_VERSION..."
    curl -fSL -o "$TARBALL" "https://download.qemu.org/qemu-$QEMU_VERSION.tar.xz"
fi

echo ">> Extracting..."
tar -xf "$TARBALL" -C "$WORK"
SRC="$WORK/qemu-$QEMU_VERSION"

echo ">> Applying Open Sober SMC patches..."
patch -p1 -d "$SRC" < "$SCRIPT_DIR/patches/0001-cpu-exec-common-force-CF_NO_GOTO_TB.patch"
patch -p1 -d "$SRC" < "$SCRIPT_DIR/patches/0002-cpu-exec-noop-tb_set_jmp_target.patch"

echo ">> Configuring (aarch64 linux-user only)..."
mkdir -p "$WORK/build"
(
    cd "$WORK/build"
    "$SRC/configure" \
        --target-list=aarch64-linux-user \
        --disable-system --disable-tools --disable-docs \
        --disable-guest-agent --without-default-features
)

echo ">> Building..."
ninja -C "$WORK/build" -j"$(nproc)" qemu-aarch64

echo ">> Installing to $SCRIPT_DIR/out/qemu-aarch64"
mkdir -p "$SCRIPT_DIR/out"
cp "$WORK/build/qemu-aarch64" "$SCRIPT_DIR/out/qemu-aarch64"
chmod +x "$SCRIPT_DIR/out/qemu-aarch64"

echo ">> Done:"
"$SCRIPT_DIR/out/qemu-aarch64" --version