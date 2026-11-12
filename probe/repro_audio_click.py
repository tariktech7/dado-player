"""Reproduction harness: click the player's Audio button and measure fallout.

The player chrome draws a native mpv child HWND above egui. When a track
popover is open, player.rs used to call `position_video()` twice per frame with
two *different* rects (normal video rect + a shrunken one). `last_video_rect`
then flipped every frame and the child HWND was resized twice per frame -
a SetWindowPos/repaint storm ("System dreht durch").

This script boots the player with the local test stream, clicks the Audio
button at its computed client coordinates, then measures:
  * process CPU time consumed over a 3s window (before vs after the click),
  * whether the process is still alive and responds to WM_CLOSE,
  * two screenshots a short interval apart (diff => flicker / repaint churn).

Usage: python probe/repro_audio_click.py
"""
import ctypes
import os
import subprocess
import sys
import time
from ctypes import wintypes

from PIL import Image, ImageChops

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
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wintypes.DWORD), ("dwHighDateTime", wintypes.DWORD)]


kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(FILETIME)] * 4
kernel32.GetProcessTimes.restype = wintypes.BOOL
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]


def cpu_seconds(pid):
    h = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)
    if not h:
        return None
    a, b, c, d = (FILETIME() for _ in range(4))
    ok = kernel32.GetProcessTimes(h, ctypes.byref(a), ctypes.byref(b),
                                  ctypes.byref(c), ctypes.byref(d))
    kernel32.CloseHandle(h)
    if not ok:
        return None
    def val(ft):
        return (ft.dwHighDateTime << 32) | ft.dwLowDateTime
    return (val(c) + val(d)) / 1e7


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


def client_size(hwnd):
    r = wintypes.RECT()
    user32.GetClientRect(hwnd, ctypes.byref(r))
    return r.right - r.left, r.bottom - r.top


def client_to_screen(hwnd, x, y):
    pt = wintypes.POINT(int(x), int(y))
    user32.ClientToScreen(hwnd, ctypes.byref(pt))
    return pt.x, pt.y


def shot(hwnd, path):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    hdc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mdc, bmp)
    user32.PrintWindow(hwnd, mdc, 2)
    bmi = bytearray(40)
    import struct
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


def click(hwnd, x, y):
    sx, sy = client_to_screen(hwnd, x, y)
    user32.SetCursorPos(sx, sy)
    time.sleep(0.12)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    time.sleep(0.06)
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP


def click_audio_button(hwnd):
    """Hit the actual button with Win32 click delivery.

    Use Accessibility/UI Automation element discovery if available through the
    button geometry, otherwise derive its current center from the client area.
    This avoids relying on the shortcut as the sole assertion for the visual
    button the user reported clicking.
    """
    cw, ch = client_size(hwnd)
    ppp = (user32.GetDpiForWindow(hwnd) or 96) / 96.0
    x, y = audio_button_center(cw / ppp, ch / ppp)
    click(hwnd, x * ppp, y * ppp)


def audio_button_center(cw, ch):
    """Mirror the geometry in src/ui/player.rs (stream-info/audio cluster)."""
    pad = 16.0
    line_h = 16.0
    info_h = line_h * 2.0 + 12.0
    btns_h = 32.0
    gap = 8.0
    panel_w = 276.0
    panel_h = info_h + gap + btns_h
    panel_left = cw - pad - panel_w
    panel_top = ch - pad - panel_h
    by = panel_top + info_h + gap
    btn_w = (panel_w - 24.0 - 8.0) / 2.0
    return panel_left + 12.0 + btn_w / 2.0, by + btns_h / 2.0


def popover_state(path):
    """Detect the audio popover's exact header in the top-right panel.

    `DADO_KEEP_UI=1` holds the chrome visible. Audio and subtitle popovers are
    mutually exclusive and draw a panel whose header is `Audio`; scan its
    expected anchor region for the word, rather than treating the whole
    bottom extension as if it were the old 100px-high cutout.
    """
    im = Image.open(path).convert("RGB")
    px = im.load()
    w, h = im.size
    ppp = w / 1400.0
    # `draw_track_popover` anchors the panel above the right-side info panel;
    # its header sits in the upper part of the bottom quarter.
    x0, x1 = int(w * 0.78), int(w * 0.99)
    y0, y1 = int(h * 0.68), int(h * 0.88)
    bright = []
    for y in range(y0, y1, 2):
        for x in range(x0, x1, 2):
            r, g, b = px[x, y]
            if min(r, g, b) > 145 and max(r, g, b) - min(r, g, b) < 80:
                bright.append((x, y))
    # Text glyph pixels form a sparse cluster; an open popover also paints its
    # own panel, so require a clear increase in neutral bright pixels.
    count = len(bright)
    return ("OPEN" if count > 180 else "CLOSED"), count


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
    env = dict(os.environ)
    env["IPTV_DEMO"] = "1"
    env["IPTV_DEMO_VIEW"] = "live"
    env["IPTV_TEST_STREAM"] = "1"
    env["DADO_KEEP_UI"] = "1"
    qa_log = os.path.join(TMP, "dado_audio_state.log")
    try:
        os.remove(qa_log)
    except FileNotFoundError:
        pass
    env["DADO_QA_STATE_LOG"] = qa_log
    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    hwnd = None
    deadline = time.time() + 40
    while time.time() < deadline:
        hwnd = find_window(TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            print("FAIL: exited early", proc.returncode)
            return 1
        time.sleep(0.4)
    if not hwnd:
        print("FAIL: no window")
        proc.kill()
        return 1

    time.sleep(2.5)
    user32.ShowWindow(hwnd, 9)
    dpi = user32.GetDpiForWindow(hwnd) or 96
    user32.SetWindowPos(hwnd, 0, 0, 0, int(1400 * dpi / 96),
                        int(880 * dpi / 96), SWP_NOZORDER)
    time.sleep(2.0)

    cw, ch = client_size(hwnd)  # physical pixels
    ppp = dpi / 96.0
    lw, lh = cw / ppp, ch / ppp  # egui logical points
    ax, ay = audio_button_center(lw, lh)
    print(f"client={cw}x{ch} physical  logical={lw:.0f}x{lh:.0f} "
          f"ppp={ppp:.2f}  audio-button=({ax:.0f},{ay:.0f}) logical")

    # An inactive window can swallow the first synthetic click (Windows sends
    # it as activation). Focus explicitly so the harness tests the UI code
    # path rather than focus rules.
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.5)

    # Baseline CPU before opening the popover.
    c0 = cpu_seconds(proc.pid)
    time.sleep(3.0)
    c1 = cpu_seconds(proc.pid)
    base = (c1 - c0) if (c0 is not None and c1 is not None) else -1
    print(f"CPU baseline (menu closed) : {base:.2f}s CPU / 3.0s wall")

    before = os.path.join(TMP, "dado_repro_before.png")
    shot(hwnd, before)

    with open(qa_log, encoding="utf-8") as f:
        baseline_log = f.read()
    click_audio_button(hwnd)
    time.sleep(1.0)
    with open(qa_log, encoding="utf-8") as f:
        click_log = f.read()[len(baseline_log):]
    p_click = os.path.join(TMP, "dado_repro_open_click.png")
    shot(hwnd, p_click)

    # Two frames a short interval apart: a resize/repaint storm makes the
    # chrome unstable across consecutive captures.
    p1 = os.path.join(TMP, "dado_repro_open1.png")
    p2 = os.path.join(TMP, "dado_repro_open2.png")
    shot(hwnd, p1)
    time.sleep(0.35)
    shot(hwnd, p2)

    # The documented `A` shortcut sets the very same state as the button.
    # If the mouse coordinates were off, this still toggles the popover, so we
    # can tell a coordinate miss apart from a broken UI code path.
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.2)
    user32.keybd_event(0x41, 0, 0, 0)          # VK_A down
    time.sleep(0.05)
    user32.keybd_event(0x41, 0, 0x0002, 0)    # KEYEVENTF_KEYUP
    time.sleep(1.0)
    p_key = os.path.join(TMP, "dado_repro_open_key.png")
    shot(hwnd, p_key)

    d = ImageChops.difference(Image.open(p1).convert("RGB"),
                              Image.open(p2).convert("RGB"))
    bbox = d.getbbox()
    changed = 0
    if bbox:
        small = d.crop(bbox).resize((min(400, bbox[2]-bbox[0]),
                                    min(300, bbox[3]-bbox[1])))
        changed = sum(1 for px in small.convert("L").getdata() if px > 24)
    print(f"frame-to-frame diff bbox   : {bbox}  (changed px sample: {changed})")

    for tag, path in [("before", before), ("after click", p_click),
                      ("frame 1", p1), ("frame 2", p2), ("after A key", p_key)]:
        state, n = popover_state(path)
        print(f"popover {tag:<12}: {state:<6} (band samples={n})")

    c2 = cpu_seconds(proc.pid)
    time.sleep(3.0)
    c3 = cpu_seconds(proc.pid)
    after = (c3 - c2) if (c2 is not None and c3 is not None) else -1
    print(f"CPU after click (menu open): {after:.2f}s CPU / 3.0s wall")
    alive = proc.poll() is None
    print(f"alive after click          : {alive}")
    user32.PostMessageW(hwnd, 0x0010, 0, 0)  # WM_CLOSE
    try:
        rc = proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
        print("FAIL: WM_CLOSE ignored (hung)")
        return 1
    print(f"WM_CLOSE exit code         : {rc}")
    opened_by_click = popover_state(p_click)[0] == "OPEN"
    opened_by_key = popover_state(p_key)[0] == "OPEN"
    stable_frames = changed < 1000
    qa_text = ""
    if os.path.isfile(qa_log):
        with open(qa_log, encoding="utf-8") as f:
            qa_text = f.read()
    click_state = "audio=true" in click_log
    lines = qa_text.splitlines()
    rects = [line.split("video_rect=", 1)[1] for line in lines if "video_rect=" in line]
    audio_idx = next((i for i, line in enumerate(lines) if "audio=true" in line), len(lines))
    rects_after_audio = [line for line in lines[audio_idx + 1:] if "video_rect=" in line]
    # One resize is expected while the harness establishes the final window
    # size; opening Audio must not trigger any additional video geometry.
    fixed_rect = len(rects) >= 2 and not rects_after_audio
    print(f"QA state log: click_audio={click_state} video_rect_values={len(set(rects))} post_audio_resizes={len(rects_after_audio)}")
    if not alive or not click_state or not opened_by_key or not stable_frames or not fixed_rect or rc != 0:
        print("FAIL: audio interaction did not produce a stable, logged popover")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
