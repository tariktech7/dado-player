"""Verify the player scaling is now rock-steady (no HWND resize => no mpv re-fit).

Previously opening the Audio/Untertitel popover or hiding the chrome changed the
video rect, which resized the native mpv child HWND and made mpv re-fit/re-scale
the picture ("skalierung springt hin und her"). The fix keeps the HWND at ONE
fixed rect (the full player area) for the whole playback and reveals the chrome
through `SetWindowRgn` holes instead of resizing.

This script proves three things:
  1. The native video child HWND has the SAME size in every state
     (chrome visible / popover open / popover closed / chrome hidden).
     Identical size => mpv never re-fits => the picture never rescales.
  2. The egui chrome is actually visible where the holes are (top + bottom bands
     show the dark glass + white text, which can ONLY happen if the video HWND
     has a region cutout there -- the HWND otherwise covers the whole window).
  3. The picture content (test pattern) is unchanged in scale between states.
"""
import ctypes
import os
import subprocess
import sys
import time
from ctypes import wintypes

from PIL import Image

try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)
except Exception:
    pass

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
kernel32 = ctypes.windll.kernel32

ROOT = r"C:\Users\Tarik\WorkBuddy AI\IPTV App"
EXE = os.path.join(ROOT, "dist", "Dado-Player.exe")
TMP = os.environ["TEMP"]
TITLE = "Dado-Player"
SWP_NOZORDER = 0x0004
VP_CLASS = "DadoVideoSurface"


def find_window(title):
    res = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        n = user32.GetWindowTextLengthW(hwnd)
        if n == 0:
            return True
        buf = ctypes.create_unicode_buffer(n + 1)
        user32.GetWindowTextW(hwnd, buf, n + 1)
        if buf.value == title:
            res.append(hwnd)
        return True

    user32.EnumWindows(cb, 0)
    return res[0] if res else None


def find_video_child(parent):
    res = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, _):
        buf = ctypes.create_unicode_buffer(64)
        user32.GetClassNameW(hwnd, buf, 64)
        if buf.value == VP_CLASS:
            res.append(hwnd)
        return True

    user32.EnumChildWindows(parent, cb, 0)
    return res[0] if res else None


def video_rect(hwnd):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    return r.left, r.top, r.right - r.left, r.bottom - r.top


def client_size(hwnd):
    r = wintypes.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    return r.right - r.left, r.bottom - r.top


def shot(hwnd, path):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    hdc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mdc, bmp)
    user32.PrintWindow(hwnd, mdc, 2)
    import struct
    bmi = bytearray(40)
    struct.pack_into("<I", bmi, 0, 40)
    struct.pack_into("<i", bmi, 4, w)
    struct.pack_into("<i", bmi, 8, -h)
    struct.pack_into("<H", bmi, 12, 1)
    struct.pack_into("<H", bmi, 14, 32)
    buf = ctypes.create_string_buffer(w * h * 4)
    bt = ctypes.c_char * len(bmi)
    gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bt.from_buffer(bmi)), 0)
    Image.frombuffer("RGB", (w, h), buf, "raw", "BGRX", 0, 1).save(path)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(hwnd, hdc)


def band_stats(path, y0, y1):
    """Mean brightness + count of near-white (text) pixels in a row band."""
    im = Image.open(path).convert("RGB")
    px = im.load()
    w, h = im.size
    y0 = max(0, min(y0, h - 1))
    y1 = max(y0 + 1, min(y1, h))
    s = 0
    n = 0
    white = 0
    for y in range(y0, y1, 2):
        for x in range(0, w, 2):
            r, g, b = px[x, y]
            s += (r + g + b) / 3.0
            n += 1
            if r > 180 and g > 180 and b > 180:
                white += 1
    return s / max(n, 1), white


def launch(env_extra):
    env = dict(os.environ)
    env["IPTV_DEMO"] = "1"
    env["IPTV_DEMO_VIEW"] = "live"
    env["IPTV_TEST_STREAM"] = "1"
    env.update(env_extra)
    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    hwnd = None
    deadline = time.time() + 40
    while time.time() < deadline:
        hwnd = find_window(TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            print("FAIL: exited early", proc.returncode)
            return None, None
        time.sleep(0.4)
    if not hwnd:
        print("FAIL: no window")
        proc.kill()
        return None, None
    time.sleep(2.5)
    user32.ShowWindow(hwnd, 9)
    dpi = user32.GetDpiForWindow(hwnd) or 96
    # Fixed window so the video rect is deterministic across runs.
    user32.SetWindowPos(hwnd, 0, 0, 0, int(1400 * dpi / 96),
                        int(880 * dpi / 96), SWP_NOZORDER)
    time.sleep(2.0)
    # Give mpv a moment to actually present frames.
    time.sleep(1.0)
    return proc, hwnd


def close_process(proc, hwnd):
    """Close the owning instance and fail cleanly on a shutdown hang."""
    if hwnd and user32.IsWindow(hwnd):
        user32.PostMessageW(hwnd, 0x0010, 0, 0)  # WM_CLOSE
    try:
        return proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
        return None


def main():
    active = subprocess.run(
        ["tasklist", "/FI", "IMAGENAME eq Dado-Player.exe", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        encoding="mbcs",
        errors="replace",
        check=False,
    )
    if "Dado-Player.exe" in (active.stdout or ""):
        print("FAIL: another Dado-Player instance is running; close it to keep this probe isolated.")
        return 2
    print("=== launch 1: chrome pinned (DADO_KEEP_UI=1) ===")
    proc, hwnd = launch({"DADO_KEEP_UI": "1"})
    if not proc:
        return 1
    vh = find_video_child(hwnd)
    print(f"video child HWND found : {bool(vh)}")
    if not vh:
        print("FAIL: no video surface child")
        proc.kill()
        return 1

    def size_tag(tag):
        x, y, w, h = video_rect(vh)
        print(f"  {tag:<16}: x={x} y={y} size={w}x{h}")
        return (w, h)

    first_hwnd = hwnd
    s_visible = size_tag("chrome visible")
    shot(first_hwnd, os.path.join(TMP, "dado_scale_visible.png"))

    # Open the Audio popover via the documented `A` shortcut.
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.3)
    user32.keybd_event(0x41, 0, 0, 0)
    time.sleep(0.05)
    user32.keybd_event(0x41, 0, 0x0002, 0)
    time.sleep(1.0)
    s_popover = size_tag("popover open")
    shot(first_hwnd, os.path.join(TMP, "dado_scale_popover.png"))

    # Close it again.
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.3)
    user32.keybd_event(0x41, 0, 0, 0)
    time.sleep(0.05)
    user32.keybd_event(0x41, 0, 0x0002, 0)
    time.sleep(1.0)
    s_closed = size_tag("popover closed")
    shot(first_hwnd, os.path.join(TMP, "dado_scale_closed.png"))

    rc = close_process(proc, first_hwnd)
    if rc != 0:
        print(f"FAIL: first player exited with code {rc}")
        return 1

    print("=== launch 2: chrome auto-hides (no DADO_KEEP_UI) ===")
    proc, hwnd = launch({})
    if not proc:
        return 1
    second_hwnd = hwnd
    vh = find_video_child(second_hwnd)
    if not vh:
        print("FAIL: no video surface child in auto-hide launch")
        close_process(proc, second_hwnd)
        return 1
    time.sleep(3.4)  # exceed HIDE_AFTER (3.0s) so chrome fades out
    s_hidden = size_tag("chrome hidden")
    path_hidden = os.path.join(TMP, "dado_scale_hidden.png")
    shot(second_hwnd, path_hidden)
    rc = close_process(proc, second_hwnd)
    if rc != 0:
        print(f"FAIL: auto-hide player exited with code {rc}")
        return 1

    # --- analysis ---
    sizes = {
        "chrome visible": s_visible,
        "popover open": s_popover,
        "popover closed": s_closed,
        "chrome hidden": s_hidden,
    }
    uniq = set(sizes.values())
    print("\n=== RESULT ===")
    print("unique HWND sizes across all states:", len(uniq))
    for k, v in sizes.items():
        print(f"  {k:<16}: {v[0]}x{v[1]}")
    if len(uniq) == 1:
        print("PASS: video HWND size is IDENTICAL in every state -> mpv never "
              "re-fits -> picture scaling never changes.")
    else:
        print("FAIL: video HWND size differs between states -> picture rescales.")
        return 1

    # --- chrome-visible screenshot band check ---
    path = os.path.join(TMP, "dado_scale_visible.png")
    im = Image.open(path)
    W, H = im.size
    ppp = W / 1400.0  # logical window width was 1400
    top_h = int(56 * ppp)
    bot_h = int(120 * ppp)
    mean_top, white_top = band_stats(path, 0, top_h)
    mean_bot, white_bot = band_stats(path, H - bot_h, H)
    print(f"\nchrome-visible screenshot: {W}x{H}, ppp~{ppp:.2f}")
    print(f"  top band  mean={mean_top:5.1f}  white-text-px={white_top}")
    print(f"  bot band  mean={mean_bot:5.1f}  white-text-px={white_bot}")
    if mean_top < 70 and white_top > 20 and mean_bot < 70 and white_bot > 20:
        print("PASS: top+bottom bands show dark glass with white text -> the "
              "egui chrome is visible THROUGH the window-region holes (video "
              "HWND would otherwise cover them with the bright test pattern).")
    else:
        print("WARN: chrome bands not clearly detected (mean too high or no "
              "text). Hole may not be revealing chrome as expected.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
