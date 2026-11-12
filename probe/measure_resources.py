"""Measured RAM/CPU baseline for the Windows build (no third-party deps).

Samples the real process with Win32 ``GetProcessMemoryInfo`` and
``GetProcessTimes`` — no Task Manager screenshots, no estimates. Two
scenarios are measured: idle after a deterministic demo login, and active
playback of the bundled test stream.

Usage:
    python probe/measure_resources.py [--seconds 12]

Output: probe/out/resources.json + a short console summary.
"""

import ctypes
import hashlib
import json
import os
import statistics
import subprocess
import sys
import time
from ctypes import wintypes
from datetime import datetime, timezone

import shot_ui

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "probe", "out")
EXE = os.environ.get("DADO_EXE") or os.path.join(ROOT, "dist", "Dado-Player.exe")

psapi = ctypes.windll.psapi
kernel32 = ctypes.windll.kernel32
PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_VM_READ = 0x0010



class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wintypes.DWORD), ("dwHighDateTime", wintypes.DWORD)]


def ft_value(ft):
    return (ft.dwHighDateTime << 32) | ft.dwLowDateTime


def cpu_seconds(handle):
    creation, exit_t, kernel_t, user_t = [FILETIME() for _ in range(4)]
    ok = kernel32.GetProcessTimes(
        handle,
        ctypes.byref(creation),
        ctypes.byref(exit_t),
        ctypes.byref(kernel_t),
        ctypes.byref(user_t),
    )
    if not ok:
        return 0.0
    # FILETIME is in 100 ns units.
    return (ft_value(kernel_t) + ft_value(user_t)) / 10_000_000.0


def memory(handle):
    pmc = PROCESS_MEMORY_COUNTERS()
    pmc.cb = ctypes.sizeof(pmc)
    if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(pmc), pmc.cb):
        return {}
    return {
        "working_set_mb": round(pmc.WorkingSetSize / 1024 / 1024, 2),
        "peak_working_set_mb": round(pmc.PeakWorkingSetSize / 1024 / 1024, 2),
        "private_commit_mb": round(pmc.PagefileUsage / 1024 / 1024, 2),
        "peak_commit_mb": round(pmc.PeakPagefileUsage / 1024 / 1024, 2),
    }


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def open_process(pid):
    return kernel32.OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, False, pid
    )


def process_tree_handles(root_pid):
    """Return query handles for the app process and live descendants.

    WMIC is absent on current Windows images, so use the built-in CIM query
    first and retain WMIC only as a compatibility fallback. The renderer is
    discovered after startup and included in the aggregate resource numbers.
    """
    command = (
        "$ErrorActionPreference='SilentlyContinue'; "
        "Get-CimInstance Win32_Process | "
        "Select-Object ProcessId,ParentProcessId | ConvertTo-Json -Compress"
    )
    query = subprocess.run(
        ["powershell", "-NoProfile", "-Command", command],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if not (query.stdout or "").strip():
        query = subprocess.run(
            ["wmic", "process", "get", "ProcessId,ParentProcessId", "/format:csv"],
            capture_output=True,
            text=True,
            encoding="mbcs",
            errors="replace",
            check=False,
        )
    children = {}
    try:
        rows = json.loads(query.stdout)
        if isinstance(rows, dict):
            rows = [rows]
        for row in rows or []:
            pid = int(row.get("ProcessId"))
            parent = int(row.get("ParentProcessId"))
            children.setdefault(parent, []).append(pid)
    except (ValueError, TypeError, json.JSONDecodeError):
        for line in query.stdout.splitlines():
            parts = [part.strip() for part in line.split(",")]
            if len(parts) != 3 or parts[0].lower() == "node":
                continue
            try:
                pid, parent = int(parts[1]), int(parts[2])
            except ValueError:
                continue
            children.setdefault(parent, []).append(pid)
    result = {root_pid: open_process(root_pid)}
    pending = [root_pid]
    while pending:
        parent = pending.pop()
        for pid in children.get(parent, []):
            if pid not in result:
                result[pid] = open_process(pid)
                pending.append(pid)
    return {pid: handle for pid, handle in result.items() if handle}


def aggregate_memory(handles):
    snapshots = [memory(handle) for handle in handles.values()]
    snapshots = [s for s in snapshots if s]
    if not snapshots:
        return {}
    return {
        key: round(sum(s[key] for s in snapshots), 2)
        for key in (
            "working_set_mb",
            "peak_working_set_mb",
            "private_commit_mb",
            "peak_commit_mb",
        )
    }


def aggregate_cpu(handles):
    return sum(cpu_seconds(handle) for handle in handles.values())


def close_handles(handles):
    for handle in handles.values():
        kernel32.CloseHandle(handle)


def scenario(name, extra_env, seconds, hooks=("IPTV_DEMO", "IPTV_DEMO_VIEW", "IPTV_TEST_STREAM", "IPTV_TEST_URL", "DADO_KEEP_UI")):
    # Discard unrelated inherited QA settings (for example a stuck detail-error
    # hook); each scenario must represent exactly its declared state.
    env = dict(os.environ)
    for key in hooks:
        env.pop(key, None)
    env["IPTV_DEMO"] = "1"
    env.update(extra_env)

    started = time.monotonic()
    proc = subprocess.Popen([EXE], cwd=os.path.dirname(EXE), env=env)
    handles = process_tree_handles(proc.pid)
    if not handles:
        proc.kill()
        proc.wait(timeout=5)
        return {"name": name, "ok": False, "error": "unable to query app process memory"}
    hwnd = None
    deadline = started + 40
    while time.monotonic() < deadline:
        hwnd = shot_ui.find_window(shot_ui.TITLE)
        if hwnd:
            break
        if proc.poll() is not None:
            close_handles(handles)
            return {"name": name, "ok": False, "error": "early exit %s" % proc.returncode}
        time.sleep(0.05)
    cold_start_s = round(time.monotonic() - started, 2)

    samples = []
    prev_cpu = aggregate_cpu(handles)
    prev_t = time.monotonic()
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        time.sleep(0.5)
        now = time.monotonic()
        cur_cpu = aggregate_cpu(handles)
        wall = max(now - prev_t, 1e-6)
        cpu_pct = round(100.0 * (cur_cpu - prev_cpu) / wall, 2)
        prev_cpu, prev_t = cur_cpu, now
        row = {"t_s": round(now - started, 1), "cpu_pct_of_one_core": cpu_pct}
        row.update(aggregate_memory(handles))
        samples.append(row)

    close_handles(handles)
    shot_ui.user32.PostMessageW(hwnd or 0, 0x0010, 0, 0) if hwnd else None
    try:
        rc = proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        rc = proc.returncode

    ws = [s["working_set_mb"] for s in samples if "working_set_mb" in s]
    cpu = [s["cpu_pct_of_one_core"] for s in samples]
    peak = max((s.get("peak_working_set_mb", 0) for s in samples), default=0)
    steady = [s["cpu_pct_of_one_core"] for s in samples[6:]] or cpu
    return {
        "name": name,
        "ok": rc in (0, None) and bool(samples),
        "exit_code": rc,
        "cold_start_to_window_s": cold_start_s,
        "sample_interval_s": 0.5,
        "samples": len(samples),
        "working_set_start_mb": ws[0] if ws else None,
        "working_set_peak_mb": max(ws) if ws else None,
        "working_set_steady_mb": round(statistics.median(ws[-6:]), 2) if ws else None,
        "peak_working_set_reported_mb": peak,
        "private_commit_peak_mb": max((s.get("private_commit_mb", 0) for s in samples), default=0),
        "cpu_mean_pct_one_core": round(statistics.mean(steady), 2) if steady else None,
        "cpu_peak_pct_one_core": max(cpu) if cpu else None,
        "core_count": os.cpu_count(),
        "env": extra_env,
        "trace": samples,
    }


def main():
    seconds = 12.0
    if "--seconds" in sys.argv:
        seconds = float(sys.argv[sys.argv.index("--seconds") + 1])
    # The sampling and screenshot helpers target the Dado title, so concurrent
    # desktop sessions would silently attach metrics to the wrong PID/window.
    active = subprocess.run(
        ["tasklist", "/FI", "IMAGENAME eq Dado-Player.exe", "/FO", "CSV", "/NH"],
        capture_output=True,
        text=True,
        encoding="mbcs",
        errors="replace",
        check=False,
    )
    if "Dado-Player.exe" in (active.stdout or ""):
        print("Dado-Player.exe already running; close it and retry to keep measurements isolated.")
        return 2
    if not os.path.isfile(EXE):
        print("EXE nicht gefunden:", EXE)
        return 2
    os.makedirs(OUT, exist_ok=True)

    idle = scenario("idle_demo", {"IPTV_DEMO_VIEW": "home"}, seconds)
    playback = scenario(
        "playback_test_stream",
        {"IPTV_TEST_STREAM": "1", "DADO_KEEP_UI": "1"},
        seconds,
    )
    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "exe": os.path.relpath(EXE, ROOT).replace("\\", "/"),
        "exe_sha256": sha256(EXE),
        "duration_per_scenario_s": seconds,
        "note": "Working set and CPU aggregate the app process plus live descendants; CPU is percent of one logical core.",
        "scenarios": [idle, playback],
    }
    path = os.path.join(OUT, "resources.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    for s in (idle, playback):
        print(
            "%-22s ok=%s cold=%ss WS peak=%sMB steady=%sMB CPU mean=%s%% peak=%s%%"
            % (
                s["name"],
                s.get("ok"),
                s.get("cold_start_to_window_s"),
                s.get("working_set_peak_mb"),
                s.get("working_set_steady_mb"),
                s.get("cpu_mean_pct_one_core"),
                s.get("cpu_peak_pct_one_core"),
            )
        )
    print("->", path)
    return 0 if idle.get("ok") and playback.get("ok") else 1


if __name__ == "__main__":
    sys.exit(main())
