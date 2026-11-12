"""End-to-end regression for the Home ("Startseite") selection bug.

Reported behaviour: clicking a film / Live-TV item on the landing screen
appeared to do nothing until the user switched to another tab. A screenshot
alone cannot distinguish "state changed but the frame was not repainted" from
"state never changed", so this harness uses both:

  1. ``DADO_QA_STATE_LOG`` (QA-only hook in src/app.rs) records every state
     transition with a millisecond timestamp, straight from the eframe update
     boundary. The log must gain a new line within 1.2 s of the click.
  2. A before/after ``PrintWindow`` capture must actually differ, proving the
     new state was painted too.

Three independent launches are used, one per interaction, because each action
navigates away from Home:

    details  -> hero "Details" button must switch to view=movies
    poster   -> first film poster must open the player (player=true)
    live     -> first Live-TV rail card must open the player (player=true)

No tab is switched by the harness. Any success therefore proves the action
reacted on the *same* screen, without the workaround the user had to use.

Output: probe/out/home_click.json (gitignored)
Usage:  python probe/verify_home_click.py
"""

import ctypes
import json
import os
import subprocess
import sys
import time
from ctypes import wintypes
from datetime import datetime, timezone

from PIL import Image, ImageChops

import shot_ui

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "probe", "out")
EXE = os.environ.get("DADO_EXE") or os.path.join(ROOT, "dist", "Dado-Player.exe")
STATE_LOG = os.path.join(OUT, "home_click_state.log")
user32 = ctypes.windll.user32

# Fractions of the *whole window* (GetWindowRect), measured from the
# deterministic 1400x880 @144-DPI Home capture in shot_matrix.py. They are
# stable because the layout is fixed at that size.
CASES = [
    ("details", (0.222, 0.286), "view=movies"),
    ("poster", (0.215, 0.720), "player=true"),
    ("live", (0.250, 0.535), "player=true"),
]


def click_at_window_fraction(hwnd, fx, fy):
    r = wintypes.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    x, y = int(r.left + w * fx), int(r.top + h * fy)
    user32.SetCursorPos(x, y)
    time.sleep(0.12)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    time.sleep(0.06)
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP
    return (x, y, w, h)


def read_log():
    try:
        with open(STATE_LOG, "r", encoding="utf-8") as f:
            return [line.rstrip("\n") for line in f if line.strip()]
    except OSError:
        return []


def frame_diff(path_a, path_b):
    with Image.open(path_a).convert("RGB") as a, Image.open(path_b).convert("RGB") as b:
        if a.size != b.size:
            return {"different": True, "reason": "size changed"}
        d = ImageChops.difference(a, b).resize((160, 100)).convert("L")
        changed = sum(1 for px in d.getdata() if px > 16)
        return {"changed_sample_px": changed, "different": changed > 150}


def run_case(name, frac, expected):
    try:
        os.remove(STATE_LOG)
    except OSError:
        pass

    env = dict(os.environ)
    for key in (
        "IPTV_DEMO_VIEW", "IPTV_TEST_STREAM", "DADO_KEEP_UI", "IPTV_DEMO_EMPTY",
        "IPTV_DEMO_CATALOG_ISSUES", "IPTV_DEMO_DETAIL_ERROR", "IPTV_DEMO_LOADING",
        "IPTV_MOBILE", "IPTV_TV", "IPTV_DARK", "IPTV_DEMO_PALETTE",
    ):
        env.pop(key, None)
    env["IPTV_DEMO"] = "1"
    env["IPTV_DEMO_VIEW"] = "home"
    env["DADO_QA_STATE_LOG"] = STATE_LOG

    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    hwnd = None
    deadline = time.time() + 40
    while time.time() < deadline:
        hwnd = shot_ui.find_window(shot_ui.TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            return {"name": name, "ok": False, "error": "early exit %s" % proc.returncode}
        time.sleep(0.3)
    if not hwnd:
        proc.kill()
        return {"name": name, "ok": False, "error": "no window within 40s"}

    time.sleep(2.2)
    user32.ShowWindow(hwnd, 9)
    dpi = user32.GetDpiForWindow(hwnd) or 96
    user32.SetWindowPos(hwnd, 0, 0, 0, int(1400 * dpi / 96), int(880 * dpi / 96),
                        shot_ui.SWP_NOZORDER)
    time.sleep(1.0)
    # An unfocused window swallows the first synthetic click as activation.
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.4)

    before_path = os.path.join(OUT, "shots", "home_click_%s_before.png" % name)
    after_path = os.path.join(OUT, "shots", "home_click_%s_after.png" % name)
    shot_ui.shot(hwnd, before_path)
    lines_before = len(read_log())

    clicked = click_at_window_fraction(hwnd, *frac)
    deadline = time.time() + 1.2
    new_lines = []
    while time.time() < deadline:
        lines = read_log()[lines_before:]
        if any(expected in line for line in lines):
            new_lines = lines
            break
        time.sleep(0.05)

    time.sleep(0.25)
    shot_ui.shot(hwnd, after_path)
    diff = frame_diff(before_path, after_path)

    user32.PostMessageW(hwnd, 0x0010, 0, 0)  # WM_CLOSE
    try:
        rc = proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        rc = proc.returncode

    return {
        "name": name,
        "expected_state": expected,
        "click_fraction": list(frac),
        "click_screen_xy_and_window": clicked,
        "state_lines_after_click": new_lines or read_log()[lines_before:],
        "state_reacted_immediately": bool(new_lines),
        "pixels_changed": diff,
        "exit_code": rc,
        "ok": bool(new_lines) and diff["different"] and rc in (0, None),
    }


def main():
    if not os.path.isfile(EXE):
        print("EXE nicht gefunden:", EXE)
        return 2
    os.makedirs(os.path.join(OUT, "shots"), exist_ok=True)
    results = []
    for name, frac, expected in CASES:
        print("[%s] clicking %s ..." % (name, frac), flush=True)
        rec = run_case(name, frac, expected)
        results.append(rec)
        print("       state=%s pixels=%s ok=%s" % (
            rec.get("state_reacted_immediately"),
            rec.get("pixels_changed"),
            rec.get("ok")), flush=True)

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "exe": os.path.relpath(EXE, ROOT).replace("\\", "/"),
        "purpose": "Prove Home selections react without switching tabs first.",
        "passed": sum(1 for r in results if r.get("ok")),
        "total": len(results),
        "cases": results,
    }
    path = os.path.join(OUT, "home_click.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)
    print("Ergebnis: %d/%d -> %s" % (report["passed"], report["total"], path))
    return 0 if report["passed"] == report["total"] else 1


if __name__ == "__main__":
    sys.exit(main())
