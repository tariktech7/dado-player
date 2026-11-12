"""UI screenshot harness for the Windows build.

Renders the demo library in several states and writes PNGs to %TEMP%.

Two traps this script exists to avoid:
  * python.exe is DPI-unaware by default, so GetWindowRect/PrintWindow would
    report virtual coordinates and capture only the top-left crop of a scaled
    window. Hence SetProcessDpiAwareness(2) + physical sizing via GetDpiForWindow.
  * Synthetic key presses are unreliable when the window is not focused, so the
    palette is opened through the IPTV_DEMO_PALETTE hook instead of Ctrl+K.

Usage:  python probe/shot_ui.py
Output: %TEMP%/dado_{design,palette,palette_query,settings,movies}.png
"""

import ctypes, os, subprocess, sys, time, struct
from ctypes import wintypes
from PIL import Image

try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)  # per-monitor DPI aware
except Exception:
    user32_dummy = None

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

ROOT = r"C:\Users\Tarik\WorkBuddy AI\IPTV App"
EXE = os.path.join(ROOT, "dist", "Dado-Player.exe")
TMP = os.environ["TEMP"]
TITLE = "Dado-Player"
SWP_NOZORDER = 0x0004


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


def shot(hwnd, path):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    if w <= 0 or h <= 0:
        return False
    hdc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    gdi32.SelectObject(mdc, bmp)
    user32.PrintWindow(hwnd, mdc, 2)
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
    return True


def scroll_to_bottom(hwnd, steps=200):
    """Send mouse-wheel-down events so the page scrolls to its end before the
    shot. egui reads the wheel from the window under the cursor, so we park the
    cursor in the window centre first. Overscroll is clamped, so a high step
    count reliably reaches the true bottom for every page length."""
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    user32.SetCursorPos((r.left + r.right) // 2, (r.top + r.bottom) // 2)
    MOUSEEVENTF_WHEEL = 0x0800
    for _ in range(steps):
        user32.mouse_event(MOUSEEVENTF_WHEEL, 0, 0, -120, 0)
        time.sleep(0.01)
    time.sleep(0.8)


def run(tag, extra_env, view="live", size=(1400, 880), demo=True, scroll=False):
    env = dict(os.environ)
    if demo:
        env["IPTV_DEMO"] = "1"
        env["IPTV_DEMO_VIEW"] = view
    else:
        env.pop("IPTV_DEMO", None)
    env.update(extra_env)
    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    hwnd = None
    deadline = time.time() + 40
    while time.time() < deadline:
        hwnd = find_window(TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            print("exited early", proc.returncode)
            return
        time.sleep(0.4)
    if not hwnd:
        print("no window")
        proc.kill()
        return
    time.sleep(3.2)
    user32.ShowWindow(hwnd, 9)
    dpi = user32.GetDpiForWindow(hwnd) or 96
    sx = int(size[0] * dpi / 96)
    sy = int(size[1] * dpi / 96)
    user32.SetWindowPos(hwnd, 0, 0, 0, sx, sy, SWP_NOZORDER)
    time.sleep(0.9)
    if scroll:
        scroll_to_bottom(hwnd)
    shot(hwnd, os.path.join(TMP, "dado_%s.png" % tag))
    time.sleep(0.2)
    user32.PostMessageW(hwnd, 0x0010, 0, 0)
    try:
        proc.wait(timeout=15)
    except Exception:
        proc.kill()
    print(tag, "rc=", proc.returncode)


def main():
    run("design", {})
    run("palette", {"IPTV_DEMO_PALETTE": "1"})
    run("palette_query", {"IPTV_DEMO_PALETTE": "1", "IPTV_DEMO_PALETTE_Q": "fil"})
    run("settings", {}, view="settings")
    run("movies", {}, view="movies")
    run("series", {}, view="series", scroll=True)
    run("continue", {}, view="continue", scroll=True)
    run("favorites", {}, view="favorites", scroll=True)
    run("search", {}, view="search", scroll=True)
    # Login: no IPTV_DEMO → falls to the login screen.
    run("login", {}, demo=False)
    # Mobile views: phone-sized window + IPTV_MOBILE=1 forces the touch UI.
    run("mobile_login", {"IPTV_MOBILE": "1"}, size=(411, 914), demo=False)
    run("mobile_home", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "home"}, size=(411, 914), scroll=True)
    run("mobile_movies", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "movies"}, size=(411, 914), scroll=True)
    run("mobile_series", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "series"}, size=(411, 914), scroll=True)
    run("mobile_favorites", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "favorites"}, size=(411, 914), scroll=True)
    run("mobile_search", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "search"}, size=(411, 914), scroll=True)
    run("mobile_continue", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "continue"}, size=(411, 914), scroll=True)
    run("mobile_settings", {"IPTV_MOBILE": "1", "IPTV_DEMO_VIEW": "settings"}, size=(411, 914), scroll=True)
    run("player", {"IPTV_TEST_STREAM": "1", "DADO_KEEP_UI": "1"}, view="live")
    print("done")


# Guard the batch: shot_matrix.py imports this module to reuse find_window/shot
# without accidentally re-running the full legacy capture set on import.
if __name__ == "__main__":
    main()

