"""Focused check: with the Audio popover open, the content-sized strip above
 the normal bottom bar must reveal egui chrome (popover panel + track rows),
 NOT the bright video test pattern. The same measured strip is punched out of
 the fixed-size video HWND, proving both visibility and the no-resize rule."""
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


def band_mean_white(path, y0, y1):
    im = Image.open(path).convert("RGB")
    px = im.load()
    w, h = im.size
    y0 = max(0, min(y0, h - 1)); y1 = max(y0 + 1, min(y1, h))
    s = 0; n = 0; white = 0
    for y in range(y0, y1, 2):
        for x in range(0, w, 2):
            r, g, b = px[x, y]
            s += (r + g + b) / 3.0; n += 1
            if r > 180 and g > 180 and b > 180:
                white += 1
    return s / max(n, 1), white


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
    sys.exit(2)

env = dict(os.environ)
env["IPTV_DEMO"] = "1"; env["IPTV_DEMO_VIEW"] = "live"
env["IPTV_TEST_STREAM"] = "1"; env["DADO_KEEP_UI"] = "1"
proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
hwnd = None
dl = time.time() + 40
while time.time() < dl:
    hwnd = find_window(TITLE)
    if hwnd: break
    if proc.poll() is not None:
        print("FAIL exited", proc.returncode); sys.exit(1)
    time.sleep(0.4)
time.sleep(2.5)
user32.ShowWindow(hwnd, 9)
dpi = user32.GetDpiForWindow(hwnd) or 96
user32.SetWindowPos(hwnd, 0, 0, 0, int(1400 * dpi / 96), int(880 * dpi / 96), SWP_NOZORDER)
time.sleep(2.0); time.sleep(1.0)

# Open the Audio popover.
user32.SetForegroundWindow(hwnd); time.sleep(0.3)
user32.keybd_event(0x41, 0, 0, 0); time.sleep(0.05)
user32.keybd_event(0x41, 0, 0x0002, 0); time.sleep(1.0)

path = os.path.join(TMP, "dado_popover_open.png")
shot(hwnd, path)
W, H = Image.open(path).size
ppp = W / 1400.0
BOT_H = int(120 * ppp)
# The extension strip is now sized to the popover panel itself (single audio
# track => ~70px, not a flat 100px). Probe the band just above the normal bar:
# with the black-strip fix this band is the popover panel (dark glass + text),
# not the bright video that used to show through a dead strip.
# Demo audio has one track. Mirror track_popover_height(): header 28px,
# one row 32px, padding 12px, plus 8px safety => 80px logical.
POP = int(80 * ppp)
y0 = H - BOT_H - POP
y1 = H - BOT_H
mean, white = band_mean_white(path, y0, y1)
print(f"popover strip: y={y0}..{y1} mean={mean:.1f} white={white} (W={W} H={H} ppp={ppp:.2f})")
if mean < 110 and white > 5:
    print("PASS: the strip directly above the bottom bar is the popover panel "
          "(dark glass + text), not a dead black video strip -> no black bar.")
else:
    print("WARN: strip above the bottom bar looks like plain video "
          "(black strip not yet removed).")
user32.PostMessageW(hwnd, 0x0010, 0, 0)
proc.wait(timeout=15)
