#!/usr/bin/env python3
"""
Generate test.avi: an uncompressed RGB24 AVI that mpv can open with NO
per-file loadfile options. The geometry (192x144, rgb24, 10fps) lives in the
AVI strf BITMAPINFOHEADER, so the demuxer reads it from the container instead
of needing `?query` or `loadfile` options (both of which this libmpv Android
build rejects).

A moving gradient + a per-frame blue tint acts as a frame counter, so two
screen captures a few seconds apart prove *live* rendering rather than a frozen
black frame.

W*3 = 192*3 = 576 bytes/row, already a multiple of 4 -> no row padding needed.
"""
import struct

W, H = 192, 144
FPS = 10
FRAMES = 300  # 30.0 s clip -> stays in playback during both captures


def bmp_header() -> bytes:
    bi = struct.pack(
        "<IiiHHIIiiII",
        40,          # biSize
        W,           # biWidth
        -H,          # biHeight (negative => top-down, keeps frames upright)
        1,           # biPlanes
        24,          # biBitCount
        0,           # biCompression = BI_RGB
        W * H * 3,   # biSizeImage
        0,           # biXPelsPerMeter
        0,           # biYPelsPerMeter
        0,           # biClrUsed
        0,           # biClrImportant
    )
    return bi


def make_frame(i: int) -> bytes:
    """RGB24 frame: horizontal red ramp, vertical green ramp, blue = frame tint."""
    tint = (i * 4) % 256
    row = bytearray(W * 3)
    out = bytearray()
    for y in range(H):
        g = int(y / (H - 1) * 255)
        for x in range(W):
            r = int(x / (W - 1) * 255)
            o = x * 3
            row[o] = r
            row[o + 1] = g
            row[o + 2] = tint
        out += row
    return bytes(out)


def chunk(fourcc: str, data: bytes) -> bytes:
    pad = b"" if len(data) % 2 == 0 else b"\x00"
    return fourcc.encode() + struct.pack("<I", len(data)) + data + pad


def list_chunk(ltype: str, body: bytes) -> bytes:
    # AVI LIST chunk: fourcc "LIST", then <4-byte list type>, then payload.
    return b"LIST" + struct.pack("<I", len(body) + 4) + ltype.encode() + body


def main() -> None:
    frames = [make_frame(i) for i in range(FRAMES)]
    bih = bmp_header()

    # AVI main header (AVIMAINHEADER): 14 DWORDS
    avih = struct.pack(
        "<IIIIIIIIIIIIII",
        1_000_000 // FPS,   # dwMicroSecPerFrame
        0,                   # dwMaxBytesPerSec
        0,                   # dwPaddingGranularity
        0x10,                # dwFlags (has index)
        FRAMES,              # dwTotalFrames
        0,                   # dwInitialFrames
        1,                   # dwStreams
        W * H * 3,           # dwSuggestedBufferSize
        W, H,                # dwWidth, dwHeight
        0, 0, 0, 0,          # dwReserved[4]
    )

    # stream header (AVISTREAMHEADER): 2 FOURCC + 10 DWORD + RECT(4 SHORT)
    strh = struct.pack(
        "<4s4sIIIIIIIIIIHHHH",
        b"vids",             # fccType
        b"DIB ",             # fccHandler
        0,                   # dwFlags
        0,                   # dwPriority
        0,                   # dwInitialFrames
        1,                   # dwScale
        FPS,                 # dwRate
        0,                   # dwStart
        FRAMES,              # dwLength
        W * H * 3,           # dwSuggestedBufferSize
        (2 ** 32) - 1,       # dwQuality (-1)
        0,                   # dwSampleSize
        0, 0, 0, 0,          # rcFrame (left, top, right, bottom)
    )

    strl = list_chunk("strl", chunk("strh", strh) + chunk("strf", bih))

    hdrl = list_chunk("hdrl", chunk("avih", avih) + strl)

    # movi LIST with "00dc" chunks
    movi_body = b""
    idx = []
    offset = 0  # offset of chunk data relative to movi LIST payload start
    for f in frames:
        c = chunk("00dc", f)
        movi_body += c
        # idx1 entry: ckid, flags, offset (rel to movi payload), size
        idx.append(struct.pack("<4sIII", b"00dc", 0x10, offset + 4, len(f)))
        offset += len(c)
    movi = list_chunk("movi", movi_body)

    idx1 = chunk("idx1", b"".join(idx))

    body = hdrl + movi + idx1
    riff = b"RIFF" + struct.pack("<I", len(body)) + b"AVI " + body

    with open("test.avi", "wb") as fh:
        fh.write(riff)
    print(f"wrote test.avi: {len(riff)} bytes, {FRAMES} frames @ {FPS}fps, {W}x{H} rgb24")


if __name__ == "__main__":
    main()
