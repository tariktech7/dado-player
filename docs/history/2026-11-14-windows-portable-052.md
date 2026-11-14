# Windows portable 0.5.2 — 2026-11-14

Implementation source: 1425110ccdaee037a4cf966a8cd3b7d676110967.
Release: https://github.com/tariktech7/dado-player/releases/tag/windows-portable-052
Portable updater tag: portable-v0.5.2.
EXE: 54,962,885 bytes.
SHA-256: 48531548f7088ee0a339d43db8b01a77c11307cce1ba4f36f9b4f83c3b975f15.

## Diagnosis and corrections

The supplied Windows-10 photo reports __std_parallel_algorithms_hw_threads while loading d3dcompiler_47.dll. PE inspection confirmed that the packaged Windows-11 DLL imports that symbol from msvcp_win.dll. Both copied OS DLLs are now excluded, and local originals preserved under dist/legacy-system-dlls. Windows supplies its matching components; 9 application-local files remain.

The full startup test exposed an older x64 launcher ABI error: GdiplusStartup wrote a pointer-sized token through a u32 pointer. Token storage and shutdown now use usize. A real GDI+ test protects neighboring memory.

Cached runtime files are checked by CRC even when their lengths match. Launcher and updater recognize Authenticode certificate data after the package footer. Optional signing signs the inner application after icon edits and the complete portable after packaging, with SHA-256 and timestamp verification.

## Validation and limits

39 application tests, 7 launcher tests and 2 Python runtime-policy checks passed. Root Clippy with warnings denied and both format checks passed. An additional launcher Clippy run reports pre-existing Win32 acronym, doc-format and style lints; it is not an acceptance gate.

Fresh-folder smoke test passed on Windows 11 Pro x64 build 26300: startup around 4.8 seconds, 9 extracted files, video confirmed through mpv log, clean exit 0. Process modules confirmed d3dcompiler_47.dll and msvcp_win.dll loaded from System32, with neither extracted beside the application. No physical Windows-10/VM test was available.

GitHub QA runners still refuse startup because account payments/spending limits are blocked (Rust QA run 43580471671). Checks above ran locally.

The EXE is NotSigned. No publicly trusted publisher certificate was present. The unrelated self-signed certificate was not used. No trusted-root certificate or security-policy exception was installed. Public signing requires a verified publisher identity; SmartScreen reputation is separate. Microsoft Store distribution is an alternative to download warnings. Reference: https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation

Android build 7 and iOS build 4 are unchanged assets retained for mobile updater compatibility.
