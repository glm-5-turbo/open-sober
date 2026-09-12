#!/bin/bash
# SH24: reproducible artifact for --renderframe-drawprobe — the engine's OWN
# real geometry wrapper 0x5b35288 dispatches a real glDrawElements through the
# GLES bridge (slots 9/10 seeded). Proves the geometry draw path is
# bridge-functional (analogous to SH17/22 for the clear path).
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh24-drawprobe.txt
rm -f "$LOG"
timeout 60 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=4000 JIT_TRACE=1 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe \
  --kicker 0x106863af8 > "$LOG" 2>&1
echo "EXIT=$?"
echo "=== geometry draw reached through bridge? ==="
grep -E "seedgles\] slot (9|10)|hostcall@gl(DrawElements|DrawArrays|BindBuffer)|geometry wrapper|post-draw swap" "$LOG" | head
echo "=== should show hostcall@glDrawElements pc=0x7f0000002a38 x0=0x4 (GL_TRIANGLES) ==="
grep -c "hostcall@glDrawElements" "$LOG"