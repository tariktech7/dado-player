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

The header is the ABI reference. Keep declarations and ownership contracts in sync with the Rust bridge and exported Swift functions; validate with the build script’s nm check.

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

## Build

Run `bash ios/build.sh --ipa` from the repository root with the Apple device SDK, Swift, and the Rust target aarch64-apple-ios installed. The script builds the Rust archive, links the Swift host and packages Payload/DadoPlayer.app. Info.plist supplies the bundle metadata. Package.swift resolves Vendor relative to its own location.

The ios.yml workflow runs the device build and uploads Dado-Player-unsigned.ipa on success. Windows can type-check the Rust bridge, AVPlayer adapter and startup API with `cargo check --features ios-check --lib`. Device linking, signing and playback need separate validation.

## Dado and Apple controls

`ios_native_player` defaults to false. Dado renders its controls in transparent
egui/Metal above an AVPlayerLayer behind winit’s root view. Opting into Apple
Player presents AVPlayerViewController instead. Audio and subtitle option IDs
are one-based within each AVMediaSelectionGroup; subtitle -1 disables selection.
The host copies track JSON into a caller-owned buffer. No cross-runtime allocation
needs to be freed by the other language.

Winit 0.29’s iOS inner_size omits safe areas. Our vendored egui-winit and
egui-wgpu use outer_size on iOS so input, drawable and layout share full-window
coordinates; app content applies safe areas exactly once.
