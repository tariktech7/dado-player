#!/usr/bin/env python3
"""Export the Dado Player logo (assets/icon.png) to a raw RGBA buffer that the
Windows single-file launcher can embed at compile time via include_bytes!().

We do this offline (no Rust image crates) so the launcher stays dependency-free
and builds without network access. The output is a flat RGBA, row-major,
top-down byte array sized W*H*4.
"""
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent          # packaging/.. = project root
SRC = ROOT / "assets" / "icon.png"
OUT = Path(__file__).resolve().parent / "launcher" / "logo.rgba"


def main() -> int:
    img = Image.open(SRC).convert("RGBA")
    img = img.resize((256, 256), Image.Resampling.LANCZOS)
    w, h = img.size
    px = img.tobytes()  # RGBA, row-major, top-down
    if len(px) != w * h * 4:
        print("ERROR: unexpected byte count", file=sys.stderr)
        return 1
    OUT.write_bytes(px)
    print(f"wrote {OUT} ({w}x{h}, {len(px)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
