#!/usr/bin/env python3
"""Generate a tiny raw RGB24 "video" for the Android video-output check.

The Dado Player's Android libmpv build has no libavdevice input devices, so
`av://lavfi:testsrc` fails. We instead feed it a raw RGB24 byte stream that the
`rawvideo` demuxer (present in libavformat.so) decodes with no container.

Frame layout: 192x144, 3 bytes/pixel (R,G,B), 60 frames at 10 fps = 6 s.
A per-frame global colour shift acts as a visible frame counter so two
screenshots taken seconds apart are guaranteed to differ (proving live frames,
not a frozen one).
"""
import os

W, H, FPS, N = 192, 144, 10, 60
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "test.raw")

# Build a static base gradient once, then shift its channels per frame.
base = bytearray()
for y in range(H):
    for x in range(W):
        base += bytes(((x * 4) & 0xFF, (y * 3) & 0xFF, 128))

frame_size = W * H * 3
with open(OUT, "wb") as f:
    for i in range(N):
        # Shift each channel by a different per-frame amount.
        r_off, g_off, b_off = (i * 6) & 0xFF, (i * 4) & 0xFF, (i * 9) & 0xFF
        frame = bytearray(frame_size)
        idx = 0
        for p in range(0, len(base), 3):
            frame[idx] = (base[p] + r_off) & 0xFF
            frame[idx + 1] = (base[p + 1] + g_off) & 0xFF
            frame[idx + 2] = (base[p + 2] + b_off) & 0xFF
            idx += 3
        f.write(frame)

print(f"wrote {OUT}: {os.path.getsize(OUT)} bytes "
      f"({W}x{H} rgb24, {N} frames @ {FPS} fps = {N / FPS:.1f}s)")
