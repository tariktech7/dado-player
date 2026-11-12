# Dado Player — iOS host

A thin Swift/UIKit shell around the Rust core. There is no Xcode project and no
SwiftUI: `main.swift` runs top-level code, calls `dado_app_set_storage_dir` +
`dado_app_start`, and Rust/winit takes over `UIApplicationMain` from there. Swift
only does three things:

1. provide the app entry point and a `UIApplicationDelegate` for UIKit,
2. own the `AVPlayerLayer` video surface and report its state over the C ABI,
3. push the environment (safe area, keyboard/IME inset, dark mode, scale) to Rust.

## ABI direction

`Sources/DadoHost/include/DadoBridge.h` is the single source of truth and is
**immutable** — do not edit it without changing `src/platform/ios/bridge.rs` in
the same commit.

| Prefix | Implemented in | Called by | Where it lives |
|---|---|---|---|
| `dado_app_*` | Rust | Swift | `main.swift`, `RootViewController.swift` |
| `dado_host_*` | Swift (`@_cdecl`) | Rust | `HostBridge.swift` |

Strings are NUL-terminated UTF-8 and are copied by the callee. Geometry is in
UIKit points; Rust converts using the reported `scale`.

Symbol counts, so the `nm` check below has a reference: **3** `dado_app_*`
(`start`, `set_storage_dir`, `set_env`) and **16** `dado_host_*`.

Video placement flows Rust → Swift: `dado_host_video_frame` and
`dado_host_video_visible` are implemented in `HostBridge.swift` and called from
`src/platform/ios/video_surface.rs`. The header has no `dado_app_video_*`
functions — an earlier draft had them in the wrong direction and they were
removed.

## SwiftPM layout

```
Package.swift
Sources/
  DadoHost/
    module.modulemap        module DadoBridge { header "include/DadoBridge.h" }
    include/DadoBridge.h    immutable ABI header
  DadoPlayer/               executable target
    main.swift              top-level entry (no @main, no SwiftUI)
    AppDelegate.swift       delegate only; winit owns the window
    RootViewController.swift env push (safe area, IME, dark, scale)
    VideoView.swift         AVPlayerLayer-backed surface
    HostBridge.swift        every dado_host_* symbol
Vendor/                     libdadoplayer.a lands here at build time (git-ignored)
xtool.yml
```

`DadoBridge` is a `.systemLibrary` target because SwiftPM has no bridging-header
support outside Xcode. System-library targets take no `publicHeadersPath`
parameter: headers come from `include/` and the module map from the target root,
which is the layout above. If SwiftPM ever fails to resolve the module, move
`module.modulemap` into `include/` and change the header line to
`header "DadoBridge.h"`.

## Build flow

Rust is not built by SwiftPM. `probe/build_ios.sh` is expected to:

1. `cargo build --target aarch64-apple-ios --release` (staticlib, not cdylib),
2. copy the result to `ios/Vendor/libdadoplayer.a`.

`Package.swift` then links it with `.unsafeFlags(["-LVendor", "-ldadoplayer"])`.
Note that `-L` is relative to the linker's working directory (the package root
when xtool drives the build).

`probe/build_ios.sh` does exactly this, plus a preflight that refuses to run
without an Apple SDK and an `nm` check of the ABI. Build it from the project
root:

Then, from this directory:

```bash
bash ../probe/build_ios.sh            # Rust staticlib + ABI check
bash ../probe/build_ios.sh --deploy   # ... then xtool dev (sign + install)
```

`xtool setup` is the one-time prerequisite (Darwin SDK + Apple sign-in).
`xtool new` is not needed — the package already exists. `xtool.yml` only sets
`version` and `bundleID`; the schema has no `displayName` key (the home-screen
name comes from an Info.plist referenced via `infoPath`, which we do not ship
yet). Verify any extra key against `xtool help` before adding it.

## Status: not compiled

This host has **never been compiled on this machine** — there is no Apple SDK,
no Swift toolchain for iOS and no `libdadoplayer.a` here, so nothing in this
directory has been type-checked against a real SDK.

First validation step, in this order:

1. `bash probe/build_ios.sh` → produces `ios/Vendor/libdadoplayer.a`.
2. The script's own `nm` gate: the three `dado_app_*` symbols must be **defined**
   (T) and the sixteen `dado_host_*` symbols must be **undefined** (U), which
   proves the two halves agree.
3. `xtool dev` → expect the first real compile. The likely first failures are
   (a) further frameworks the Rust static library pulls in (CoreAudio, AudioToolbox,
   Foundation) — add them to `linkerSettings` in `Package.swift`, and
   (b) Swift 6 strict-concurrency diagnostics if the tools version is ever raised
   above 5.9.
4. On device: if the screen is black during playback, the video layer is behind
   an opaque egui surface — that is a Rust-side clear-colour/cutout issue, not a
   host issue.
