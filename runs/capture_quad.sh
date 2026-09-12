#!/bin/bash
# SH30: reproducible artifact for --renderframe-triangle --renderframe-quad — a
# REAL TWO-ATTRIB TEXTURED QUAD rendered through the engine's OWN geometry wrapper.
# The coherent renderer holds TWO primitives (-> two vertex attribs): prim0=vb0
# offset0 format[3]{size4,GL_FLOAT} attrib0=aPos; prim1=vb0 offset16 format[1]
# {size2,GL_FLOAT} attrib1=aUV. The interleaved VBO is [pos.xyzw, uv.xy] x4
# (stride 24). The fragment shader samples a 2x2 checkerboard at the REAL
# interpolated vertex UV (not gl_FragCoord) -> 4 quadrants read the 4 texel colors.
#
# Requires: cargo build -p arm64jit --example elfjit, real libroblox.so at
# ~/.cache/open-sober/robbox/libroblox.so, Xvfb + ffmpeg.
set -u
cd "$(dirname "$0")/.."
LOG=/home/hermes-worker/runs/sh30-quad.txt
PNG=/home/hermes-worker/runs/sh30-quad.png
RAW=/home/hermes-worker/runs/sh30-quad.rgb
rm -f "$LOG" "$PNG" "$RAW"
timeout 90 env JIT_DRIVE_LIFECYCLE=1 RENDERINIT_WARMUP_MS=5000 \
  ./target/debug/examples/elfjit ~/.cache/open-sober/robbox/libroblox.so 0x2173ff4 \
  --jni --startapp 0x258b144 --renderinit 0x105b3a280 --renderthunk --renderframe \
  --renderframe-drive --renderframe-seedgles --renderframe-drawprobe --renderframe-triangle --renderframe-quad \
  --kicker 0x106863af8 > "$LOG" 2>&1 &
PID=$!
for i in $(seq 1 120); do
  if grep -q "renderframe-quad.*swap returned Ok" "$LOG" 2>/dev/null; then break; fi
  if ! kill -0 "$PID" 2>/dev/null; then break; fi
  sleep 0.5
done
sleep 2
DISPNUM=$(grep -oE "on :[0-9]+" "$LOG" | head -1 | tr -d 'on :')
echo "capturing display :${DISPNUM:-none} (post-quad-draw)"
if [ -n "$DISPNUM" ]; then
  ffmpeg -y -loglevel error -f x11grab -video_size 1280x720 -i ":$DISPNUM" \
    -frames:v 1 -f rawvideo -pix_fmt rgb24 - 2>/dev/null > "$RAW"
  echo "RAW_BYTES=$(stat -c%s "$RAW" 2>/dev/null || echo 0)"
  ffmpeg -y -loglevel error -f rawvideo -pix_fmt rgb24 -s 1280x720 -i "$RAW" "$PNG"
fi
wait $PID
echo "EXIT=$?"
echo "=== two-attrib textured quad + readbacks (expect BL red/BR green/TR white/TL blue) ==="
grep -E "renderframe-quad" "$LOG" | grep -vE "^  \[t=|VECLD" | tail -8
echo "=== pixel analysis (expect clear-blue bg + 4-color checkered quad interior) ==="
if [ -s "$RAW" ]; then
  python3 -c "
d=open('$RAW','rb').read()
from collections import Counter
c=Counter()
for j in range(0,len(d)-2,3):
    r,g,b=d[j],d[j+1],d[j+2]
    if r>150 and g<90 and b<90: c['RED']+=1
    elif g>150 and r<90 and b<90: c['GREEN']+=1
    elif b>150 and r<90 and g<90: c['BLUE']+=1
    elif r>150 and g>150 and b>150: c['WHITE']+=1
    elif r<40 and g<40 and b<110: c['BG(0,0,76)']+=1
    else: c['other']+=1
print('total px=%d'%(len(d)//3))
for k,v in c.most_common(): print('  %s x%d'%(k,v))
"
fi