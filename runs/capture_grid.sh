#!/bin/bash
# SH34 artifact: --renderframe-quad --renderframe-grid <N> — scale the coherent
# renderer onto a REAL larger mesh (NxN grid of textured quads, (4*N*N) verts +
# (6*N*N) idx) through the engine's OWN geometry wrapper, one distinct texel color
# per cell read back at the real interpolated vertex UV. Proves primitive-setup +
# the draw wrapper render real mesh topology (many verts/indices), not just the
# single 4-vert quad (SH25-33).
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh34-grid.txt
rm -f "$LOG"
N="${N:-4}"
timeout 55 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle \
  --renderframe-quad --renderframe-grid "$N" --kicker 0x106863af8 > "$LOG" 2>&1
EXIT=$?
echo "EXIT=$EXIT"
echo "=== mesh upload (expect NxN grid, 4*N*N verts, 6*N*N idx) ==="
grep -E "renderframe-quad\] vbo=" "$LOG"
echo "=== geometry wrapper result ==="
grep -E "geometry wrapper 0x5b35288 returned" "$LOG"
echo "=== per-cell readbacks (each must match its expected r/g texel) ==="
grep -E "cell\(" "$LOG"
echo "=== distinct cell-center colors observed (reference: count of readback lines with RGBA) ==="
grep -cE "cell\(.*RGBA" "$LOG"