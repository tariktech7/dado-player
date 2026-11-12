"""Reproducible Dado Player screenshot matrix (Windows).

Unlike ``shot_ui.py`` (a quick ad-hoc capture set), this script produces a
*versioned* QA artefact: one PNG per case plus a machine-readable manifest with
the exact environment, window size, EXE hash, screenshot hash, exit code and a
pixel-sanity verdict. That makes a visual regression comparable between builds
instead of relying on somebody remembering what a screenshot looked like.

Usage:
    python probe/shot_matrix.py            # full matrix
    python probe/shot_matrix.py --quick   # 6 representative cases (smoke test)

Output (gitignored):
    probe/out/shots/<case>.png
    probe/out/manifest.json
    probe/out/QA-MATRIX.md
"""

import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone

from PIL import Image, ImageStat

import shot_ui

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "probe", "out")
SHOTS = os.path.join(OUT, "shots")
EXE = os.environ.get("DADO_EXE") or os.path.join(ROOT, "dist", "Dado-Player.exe")

DESK = (1400, 880)
COMPACT = (1024, 700)
PHONE = (411, 914)
PHONE_LANDSCAPE = (914, 411)

D = {"IPTV_DEMO": "1"}
M = {"IPTV_DEMO": "1", "IPTV_MOBILE": "1"}


def case(cid, env, size=DESK, scroll=False, note=""):
    return {
        "id": cid,
        "env": env,
        "size": list(size),
        "scroll": scroll,
        "note": note,
    }


CASES = [
    # ---- desktop: theme matrix on the landing screen -----------------------
    case("desk_home", dict(D, IPTV_DEMO_VIEW="home"), note="Landing / default theme"),
    case("desk_home_dark", dict(D, IPTV_DEMO_VIEW="home", IPTV_DARK="true"), note="Forced dark"),
    case("desk_home_light", dict(D, IPTV_DEMO_VIEW="home", IPTV_DARK="false"), note="Forced light"),
    case("desk_compact", dict(D, IPTV_DEMO_VIEW="home"), COMPACT, note="Compact desktop window"),
    # ---- desktop: every menu ----------------------------------------------
    case("desk_movies", dict(D, IPTV_DEMO_VIEW="movies")),
    case("desk_series", dict(D, IPTV_DEMO_VIEW="series"), scroll=True),
    case("desk_settings", dict(D, IPTV_DEMO_VIEW="settings")),
    case("desk_favorites", dict(D, IPTV_DEMO_VIEW="favorites"), scroll=True),
    case("desk_search", dict(D, IPTV_DEMO_VIEW="search"), scroll=True),
    case("desk_continue", dict(D, IPTV_DEMO_VIEW="continue"), scroll=True),
    case("desk_palette", dict(D, IPTV_DEMO_PALETTE="1"), note="Command palette"),
    # ---- deterministic error / empty / loading / retry states -------------
    case("desk_empty", dict(D, IPTV_DEMO_VIEW="home", IPTV_DEMO_EMPTY="1"), note="Empty catalogue"),
    case("desk_catalog_issues", dict(D, IPTV_DEMO_VIEW="home", IPTV_DEMO_CATALOG_ISSUES="1"), note="Partial catalogue failure + refresh banner"),
    case("desk_loading", dict(D, IPTV_DEMO_VIEW="home", IPTV_DEMO_LOADING="1"), note="Catalogue in flight"),
    case("desk_detail_error", dict(D, IPTV_DEMO_VIEW="series", IPTV_DEMO_DETAIL_ERROR="1"), note="Episode fetch failure + retry"),
    case("mobile_detail_error", dict(M, IPTV_DEMO_VIEW="series", IPTV_DEMO_DETAIL_ERROR="1"), PHONE, note="Mobile episode failure + retry"),
    # ---- login (no demo state) --------------------------------------------
    case("login", {}, note="First-run login, no profile"),
    case("mobile_login", {"IPTV_MOBILE": "1"}, PHONE, note="Phone login"),
    # ---- phone portrait / landscape ---------------------------------------
    case("mobile_home", dict(M, IPTV_DEMO_VIEW="home"), PHONE, scroll=True),
    case("mobile_home_landscape", dict(M, IPTV_DEMO_VIEW="home"), PHONE_LANDSCAPE, scroll=True),
    case("mobile_movies", dict(M, IPTV_DEMO_VIEW="movies"), PHONE, scroll=True),
    case("mobile_series", dict(M, IPTV_DEMO_VIEW="series"), PHONE, scroll=True),
    # ---- 10-foot TV layout -------------------------------------------------
    case("tv_home", dict(D, IPTV_DEMO_VIEW="home", IPTV_TV="1"), (1600, 900), note="TV / 10-foot layout"),
    # ---- player ------------------------------------------------------------
    case("player", dict(D, IPTV_TEST_STREAM="1", DADO_KEEP_UI="1"), note="Playback chrome visible"),
]

QUICK_IDS = {
    "desk_home",
    "desk_home_light",
    "desk_catalog_issues",
    "mobile_home",
    "login",
    "player",
}


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def pixel_sanity(path):
    """Catch black/blank captures without pretending to be a vision model."""
    with Image.open(path).convert("RGB") as img:
        small = img.resize((160, 100))
        stat = ImageStat.Stat(small)
        mean = sum(stat.mean) / 3.0
        colors = len(small.getcolors(maxcolors=160 * 100) or [])
        return {
            "width": img.width,
            "height": img.height,
            "mean_brightness": round(mean, 2),
            "distinct_colors": colors,
            "blank": mean < 3.0 or colors < 8,
        }


def capture(item, settle=2.4):
    env = dict(os.environ)
    for key in (
        "IPTV_DEMO", "IPTV_DEMO_VIEW", "IPTV_DEMO_PALETTE", "IPTV_DEMO_PALETTE_Q",
        "IPTV_DEMO_EMPTY", "IPTV_DEMO_CATALOG_ISSUES", "IPTV_DEMO_DETAIL_ERROR",
        "IPTV_DEMO_LOADING", "IPTV_DARK", "IPTV_MOBILE", "IPTV_TV",
        "IPTV_TEST_STREAM", "DADO_KEEP_UI",
    ):
        env.pop(key, None)
    env.update(item["env"])

    record = dict(item)
    record["captured_at"] = datetime.now(timezone.utc).isoformat()
    path = os.path.join(SHOTS, item["id"] + ".png")
    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    hwnd = None
    deadline = time.time() + 40
    while time.time() < deadline:
        hwnd = shot_ui.find_window(shot_ui.TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            record.update(ok=False, error="process exited before a window appeared", exit_code=proc.returncode)
            return record
        time.sleep(0.35)
    if not hwnd:
        proc.kill()
        record.update(ok=False, error="no window within 40s")
        return record

    time.sleep(settle)
    shot_ui.user32.ShowWindow(hwnd, 9)
    dpi = shot_ui.user32.GetDpiForWindow(hwnd) or 96
    w = int(item["size"][0] * dpi / 96)
    h = int(item["size"][1] * dpi / 96)
    shot_ui.user32.SetWindowPos(hwnd, 0, 0, 0, w, h, shot_ui.SWP_NOZORDER)
    time.sleep(0.8)
    if item["scroll"]:
        shot_ui.scroll_to_bottom(hwnd, steps=120)

    ok = shot_ui.shot(hwnd, path)
    shot_ui.user32.PostMessageW(hwnd, 0x0010, 0, 0)  # WM_CLOSE
    try:
        rc = proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        rc = proc.returncode

    if not ok or not os.path.exists(path):
        record.update(ok=False, error="screenshot capture failed", exit_code=rc)
        return record

    record.update(
        ok=True,
        exit_code=rc,
        screenshot=os.path.relpath(path, ROOT).replace("\\", "/"),
        screenshot_sha256=sha256(path),
        screenshot_bytes=os.path.getsize(path),
        pixels=pixel_sanity(path),
    )
    if record["pixels"]["blank"]:
        record["ok"] = False
        record["error"] = "blank / all-black screenshot"
    if rc not in (0, None):
        record["ok"] = False
        record["error"] = "non-zero exit code %s" % rc
    return record


def write_report(records, exe_hash):
    passed = [r for r in records if r.get("ok")]
    failed = [r for r in records if not r.get("ok")]
    manifest = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "exe": os.path.relpath(EXE, ROOT).replace("\\", "/"),
        "exe_sha256": exe_hash,
        "python": sys.version.split()[0],
        "case_count": len(records),
        "passed": len(passed),
        "failed": len(failed),
        "cases": records,
    }
    with open(os.path.join(OUT, "manifest.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    lines = [
        "# Dado Player — visuelle QA-Matrix",
        "",
        "Generiert: `%s`" % manifest["generated_at"],
        "",
        "EXE: `%s` (SHA-256 `%s`)" % (manifest["exe"], exe_hash[:16] + "…"),
        "",
        "Ergebnis: **%d/%d bestanden**." % (len(passed), len(records)),
        "",
        "| Case | Größe | Status | Helligkeit | Farben | Notiz |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for r in records:
        px = r.get("pixels") or {}
        lines.append(
            "| `%s` | %dx%d | %s | %s | %s | %s |"
            % (
                r["id"],
                r["size"][0],
                r["size"][1],
                "OK" if r.get("ok") else "FEHLER: " + str(r.get("error", "")),
                px.get("mean_brightness", "-"),
                px.get("distinct_colors", "-"),
                r.get("note", ""),
            )
        )
    lines += [
        "",
        "## Reproduktion",
        "",
        "```",
        "python probe/shot_matrix.py",
        "```",
        "",
        "Alle Fälle starten die reale EXE isoliert mit den im Manifest",
        "festgehaltenen Umgebungsvariablen. `blank` bedeutet: mittlere Helligkeit",
        "< 3 oder weniger als 8 verschiedene Farben — also ein leerer/schwarzer",
        "Capture, kein inhaltliches Urteil. Inhaltliche Prüfung bleibt manuell",
        "anhand der PNGs in `probe/out/shots/`.",
        "",
        "## Offene, nicht automatisiert prüfbare Punkte",
        "",
        "- Echter Provider (Xtream/M3U) mit gültigen Zugangsdaten",
        "- Physisches Android-Gerät, TV-Fernbedienung, D-Pad-Fokus",
        "- Zweiter Windows-PC ohne Visual-C++-Redistributable",
        "- Langzeit-Lauf / Speicher über mehrere Stunden",
        "",
    ]
    with open(os.path.join(OUT, "QA-MATRIX.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    return manifest


def main():
    # This harness locates windows by title. Running a second desktop process
    # in parallel can attach captures to the wrong instance and corrupt both
    # test results; fail fast instead of producing a misleading manifest.
    active = subprocess.run(
        ["tasklist", "/FI", "IMAGENAME eq Dado-Player.exe", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        encoding="mbcs",
        errors="replace",
        check=False,
    )
    if "Dado-Player.exe" in (active.stdout or ""):
        print("Dado-Player.exe already running; close it and retry to keep captures isolated.")
        return 2
    if not os.path.isfile(EXE):
        print("EXE nicht gefunden:", EXE)
        return 2
    os.makedirs(SHOTS, exist_ok=True)
    exe_hash = sha256(EXE)
    selected = [c for c in CASES if "--quick" not in sys.argv or c["id"] in QUICK_IDS]
    records = []
    for i, item in enumerate(selected, 1):
        print("[%d/%d] %s ..." % (i, len(selected), item["id"]), flush=True)
        rec = capture(item)
        records.append(rec)
        print("       %s" % ("OK" if rec.get("ok") else rec.get("error")), flush=True)
    manifest = write_report(records, exe_hash)
    print("Fertig: %d/%d bestanden -> probe/out/QA-MATRIX.md" % (manifest["passed"], manifest["case_count"]))
    return 0 if manifest["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
