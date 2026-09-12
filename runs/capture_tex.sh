#!/bin/bash
# SH26: reproducible artifact for --renderframe-triangle --renderframe-tex — the
# engine's OWN coherent geometry draw path renders a REAL TEXTURED triangle: the
# textured fragment shader samples a 2x2 RGBA checkerboard (RED/GREEN/BLUE/WHITE)
# via a UV derived from gl_FragCoord. All texture/uniform/shader calls
# (glGenTextures/glBindTexture/glActiveTexture/glTexImage2D/glTexParameteri/
# glGetUniformLocation/glUniform1i) dispatch through the GLES bridge. Three
# on-triangle quadrant probes must read back three DIFFERENT texel colors
# (WHITE/GREEN/RED) which no constant shader can produce.
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh26-tex.txt
PNG=/home/hermes-worker/runs/sh26-tex.png
RAW=/home/hermes-worker/runs/sh26-tex.rgb
rm -f "$LOG" "$PNG" "$RAW"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle --renderframe-tex \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 120); do
  if grep -q "post-draw swap returned Ok" "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
sleep 2
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none} (post-draw)"
if [ -n "$DISPNUM" ]; then
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -i ":$DISPNUM" \
    -frames:v 1 -f rawvideo -pix_fmt rgb24 - 2>/dev/null > "$RAW"
  echo "RAW_BYTES=$(stat -c%s "$RAW" 2>/dev/null || echo 0)"
  if command -v ffmpeg >/dev/null 2>&1; then
    ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb24 -s 1280x720 -i "$RAW" "$PNG"
  fi
fi
wait $PID
echo "EXIT=$?"
echo "=== textured draw + readbacks (expect WHITE/GREEN/RED distinct) ==="
grep -E "renderframe-tex|renderframe-triangle.*(readback|compile_status)|hostcall@gl(TexImage2D|BindTexture|ActiveTexture|Uniform1i|GetUniformLocation|DrawElements)" "$LOG" | grep -vE "^  \[t=|VECLD" | tail -25
echo "=== pixel analysis (expect multi-color checkered triangle: red/green/blue/white) ==="
if [ -s "$RAW" ]; then
  python3 -c "
import sys
d=open('$RAW','rb').read()
W,H=1280,720
from collections import Counter
c=Counter()
for j in range(0,len(d)-2,3):
    c[(d[j],d[j+1],d[j+2])]+=1
print('total px=%d'%(len(d)//3))
for col,cnt in c.most_common(6):
    print('  RGB%d,%d,%d x%d'%(col[0],col[1],col[2],cnt))
"
fi