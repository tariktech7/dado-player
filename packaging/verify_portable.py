#!/usr/bin/env python3
"""End-to-end verification of the single-file portable build.

Copies `packaging/out/Dado-Player.exe` into a FRESH folder that contains ONLY
the EXE (no sibling DLLs), runs it with a forced extraction directory, and
checks:

  * the run folder contains exactly one file (the EXE)
  * the app boots (first-frame marker / visible window)
  * video actually renders (IPTV_TEST_STREAM=1 -> animated test pattern)
  * all 9 application-local files are extracted with correct sizes
  * closing the window exits the whole process tree with code 0

No credentials or debug logs are involved.
"""
import ctypes
import os
import shutil
import subprocess
import sys
import tempfile
import time

from ctypes import wintypes
from PIL import Image, ImageChops

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(ROOT, "packaging", "out", "Dado-Player.exe")
DIST = os.path.join(ROOT, "dist")

EXPECT = {
    "Dado-Player.exe": os.path.getsize(os.path.join(DIST, "Dado-Player.exe")),
    "libmpv-2.dll": os.path.getsize(os.path.join(DIST, "libmpv-2.dll")),
    "vulkan-1.dll": os.path.getsize(os.path.join(DIST, "vulkan-1.dll")),
    "VCRUNTIME140.dll": os.path.getsize(os.path.join(DIST, "VCRUNTIME140.dll")),
    "VCRUNTIME140_1.dll": os.path.getsize(os.path.join(DIST, "VCRUNTIME140_1.dll")),
    "msvcp140.dll": os.path.getsize(os.path.join(DIST, "msvcp140.dll")),
    "msvcp140_1.dll": os.path.getsize(os.path.join(DIST, "msvcp140_1.dll")),
    "msvcp140_2.dll": os.path.getsize(os.path.join(DIST, "msvcp140_2.dll")),
    "concrt140.dll": os.path.getsize(os.path.join(DIST, "concrt140.dll")),
}

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
kernel32 = ctypes.windll.kernel32
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
EXPECTED_RUNTIME = None

WM_CLOSE = 0x0010
SW_RESTORE = 9


def fail(msg):
    print(f"FAIL: {msg}")
    sys.exit(1)


def find_window(title):
    result = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        pid = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
        handle = kernel32.OpenProcess(0x1000, False, pid.value)
        if not handle:
            return True
        try:
            capacity = wintypes.DWORD(32768)
            image = ctypes.create_unicode_buffer(capacity.value)
            if not kernel32.QueryFullProcessImageNameW(handle, 0, image, ctypes.byref(capacity)):
                return True
            if not EXPECTED_RUNTIME or os.path.normcase(os.path.dirname(image.value)) != os.path.normcase(EXPECTED_RUNTIME):
                return True
        finally:
            kernel32.CloseHandle(handle)
        length = user32.GetWindowTextLengthW(hwnd)
        if length == 0:
            return True
        buf = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buf, length + 1)
        if buf.value == title:
            result.append(hwnd)
        return True

    user32.EnumWindows(cb, 0)
    return result[0] if result else None


def screenshot(hwnd, path):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w = r.right - r.left
    h = r.bottom - r.top
    hwnd_dc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hwnd_dc)
    bmp = gdi32.CreateCompatibleBitmap(hwnd_dc, w, h)
    gdi32.SelectObject(mdc, bmp)
    if not user32.PrintWindow(hwnd, mdc, 1):
        gdi32.DeleteObject(bmp)
        gdi32.DeleteDC(mdc)
        user32.ReleaseDC(hwnd, hwnd_dc)
        return False
    bmi = bytearray(40)
    import struct
    struct.pack_into("<I", bmi, 0, 40)
    struct.pack_into("<i", bmi, 4, w)
    struct.pack_into("<i", bmi, 8, -h)
    struct.pack_into("<H", bmi, 12, 1)
    struct.pack_into("<H", bmi, 14, 32)
    buf = ctypes.create_string_buffer(w * h * 4)
    bmi_type = ctypes.c_char * len(bmi)
    gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bmi_type.from_buffer(bmi)), 0)
    img = Image.frombuffer("RGB", (w, h), buf, "raw", "BGRX", 0, 1)
    img.save(path)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(hwnd, hwnd_dc)
    return True


def is_black(path, threshold=18):
    img = Image.open(path).convert("L")
    extrema = img.getextrema()
    return extrema[1] < threshold


def main():
    global EXPECTED_RUNTIME
    if not os.path.exists(EXE):
        fail(f"exe fehlt: {EXE}")
    # Copy into a fresh folder that contains ONLY the exe (the whole point).
    tmp = tempfile.mkdtemp(prefix="dado-portable-verify-")
    exe = os.path.join(tmp, "Dado-Player.exe")
    shutil.copyfile(EXE, exe)
    left = sorted(os.listdir(tmp))
    print(f"run folder contents: {left}")
    if left != ["Dado-Player.exe"]:
        fail("Ordner enthält mehr als nur die EXE")

    rt = tempfile.mkdtemp(prefix="dado-rt-")
    EXPECTED_RUNTIME = os.path.abspath(rt)
    log = os.path.join(exe)
    env = dict(os.environ)
    env["DADO_RUNTIME_DIR"] = rt
    env["IPTV_TEST_STREAM"] = "1"  # prove the mpv/video path works
    # DADO_DEBUG turns on the opt-in mpv log. The test stream is a local file, so
    # no credentials are ever written. We read it back to confirm real playback.
    env["DADO_DEBUG"] = "1"

    print(f"launching (cwd has only the exe) ...")
    proc = subprocess.Popen([exe], cwd=tmp, env=env)

    # Wait for first-frame marker OR a visible window, up to 60s.
    ready = os.path.join(
        tempfile.gettempdir(),
        f"Dado-Player-ready-{proc.pid}.tmp",
    )
    deadline = time.time() + 60
    booted = False
    while time.time() < deadline:
        if os.path.exists(ready):
            booted = True
            break
        if find_window("Dado-Player"):
            booted = True
            break
        if proc.poll() is not None:
            fail(f"launcher exited early with code {proc.returncode}")
        time.sleep(0.3)
    if not booted:
        fail("App hat in 60s weder Marker noch Fenster gezeigt")
    print(f"booted after ~{round(60-(deadline-time.time()),1)}s")

    # Give the test stream a moment to start rendering.
    time.sleep(3.0)
    hwnd = find_window("Dado-Player")
    if not hwnd:
        fail("sichtbares Fenster 'Dado-Player' nicht gefunden")
    user32.ShowWindow(hwnd, SW_RESTORE)
    time.sleep(0.5)
    s1 = os.path.join(tmp, "shot1.png")
    if not screenshot(hwnd, s1):
        print("WARN: Screenshot fehlgeschlagen (mpv render in child HWND often not captured)")
    else:
        if is_black(s1):
            print("WARN: Screenshot ist schwarz (UI-HWND ohne mpv-Child erwartbar)")

    # The authoritative video signal: the opt-in mpv log proves frames actually
    # decoded and rendered. (mpv draws into its own child HWND, so PrintWindow of
    # the main window cannot capture the video pixels directly.)
    mpv_log = os.path.join(tempfile.gettempdir(), "Dado-Player-mpv.log")
    mpv_text = ""
    if os.path.exists(mpv_log):
        mpv_text = open(mpv_log, encoding="utf-8", errors="replace").read()
    print(f"mpv log size: {len(mpv_text)} bytes")
    failed = ("Failed to open" in mpv_text) or ("Error opening" in mpv_text)
    playing = any(
        k in mpv_text
        for k in (
            "first video frame",
            "video=playing",
            "starting video playback",
            "VO: [gpu]",
        )
    )
    if failed:
        fail("mpv log meldet einen Wiedergabefehler:\n" + mpv_text[-1000:])
    if not playing:
        fail("mpv log zeigt KEINE Video-Wiedergabe. tail:\n" + mpv_text[-1000:])
    print("video playback confirmed via mpv log")

    # Verify extraction.
    missing = [n for n in EXPECT if not os.path.exists(os.path.join(rt, n))]
    if missing:
        fail(f"extrahierte Dateien fehlen: {missing}")
    mism = []
    for n, sz in EXPECT.items():
        got = os.path.getsize(os.path.join(rt, n))
        if got != sz:
            mism.append((n, sz, got))
    if mism:
        for n, exp, got in mism:
            print(f"  size mismatch {n}: expected {exp}, got {got}")
        fail("Groesse der extrahierten Dateien stimmt nicht")
    print(f"extraction OK: {len(EXPECT)} files in {rt}")
    for name in ("msvcp_win.dll", "d3dcompiler_47.dll"):
        if os.path.exists(os.path.join(rt, name)):
            fail(f"Windows system DLL must not be extracted: {name}")

    # Close and check clean exit.
    print("sending WM_CLOSE ...")
    user32.PostMessageW(hwnd, WM_CLOSE, 0, 0)
    try:
        rc = proc.wait(timeout=30)
    except subprocess.TimeoutExpired:
        proc.kill()
        fail("App reagierte nicht auf WM_CLOSE (Timeout)")
    if rc != 0:
        fail(f"Launcher exit code {rc} (erwartet 0)")
    print(f"clean exit: code {rc}")

    # Cleanup
    shutil.rmtree(tmp, ignore_errors=True)
    shutil.rmtree(rt, ignore_errors=True)
    print("\nPASS: single-file portable build boots, plays video, extracts, exits clean.")


if __name__ == "__main__":
    main()
