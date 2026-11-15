# Dado Player — iOS host

A thin Swift/UIKit shell around the Rust core. There is no Xcode project and no
SwiftUI: `main.swift` runs top-level code, calls `dado_app_set_storage_dir` +
`dado_app_start`, and Rust/winit takes over `UIApplicationMain` from there. Swift
only does three things:

1. provide the app entry point and a `UIApplicationDelegate` for UIKit,
2. own the AVPlayer/VLCKit video surfaces and report its state over the C ABI,
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
    VideoView.swift         AVPlayer + embedded VLC surfaces
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

### Local Windows/WSL build status (2026-11-14)

GitHub Actions currently rejects jobs before execution because of an account billing/spending restriction. There is no runner queue ETA for this failure. See [local setup status](../docs/history/2026-11-14-local-ios-build-setup.md).

The Windows/WSL device package **0.6.1 / build 10** is built locally with Swift 6.4.0, Rust 1.99.1 and the iPhoneOS 27.3 SDK extracted from the user's official Xcode XIP. The SDK archive is kept in git-ignored `ios/SDKs/`, not Downloads or Releases. No Apple-account login through xtool is needed for the unsigned AltStore package.

`dado_host_options` passes the shared playback settings to Swift before loading an item:
forward buffer duration, subtitle size and a startup timeout watchdog. A custom HTTP user agent
uses the public AVURLAssetHTTPUserAgentKey on iOS 16+. This does not impose a timeout on every
subsequent media segment. Audio/subtitle preferences are applied after track discovery;
manual choices remain in effect for the current stream. Device playback still requires separate verification.

Use the same `bash ios/build.sh --ipa` entry point inside WSL with the Linux Swift/Rust tools on PATH and xtool's Darwin artifact installed. Set `CARGO_TARGET_DIR` to a Linux-filesystem cache (locally `/home/dado-build/.cache/dadoplayer/target`) to keep Windows outputs separate. Swift uses a separate cache under `~/.cache/dadoplayer/swift-build`; `DADO_SWIFT_BUILD_DIR` overrides it.

The script reads the Darwin SDK metadata for Rust's sysroot and C compiler, then clears SDKROOT before SwiftPM compiles its Linux-hosted manifest. SwiftPM uses `--swift-sdk darwin` with the bare device triple and the package's iOS 15 minimum. On macOS the storyboard/ibtool path is preserved; Linux uses the supported iOS 14+ UILaunchScreen dictionary instead of a missing compiled storyboard. Package checks do not replace an iPhone layout/playback test.

SwiftPM does not track the external Rust archive referenced through the linker flags. The build therefore cleans the Swift package's outputs before rebuilding/linking the host, on macOS and WSL, so Rust-only changes cannot silently reuse an old executable. Rust's own build cache is retained.

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

## Automatic file playback (0.6.1)

AVPlayer error -11828 means an unrecognized file format. Xtream movie and
series URLs retain their actual provider container extension. Renaming MKV to
MP4 does not convert media and is not a fallback strategy.

Automatic mode uses AVPlayer for HLS (.m3u8, ignoring query strings and case)
and MobileVLCKit 3.7.3 for file streams including MKV/AVI/MP4/TS and opaque URLs.
A failed/unknown AVPlayer startup can switch once to VLC with the same URL and
resume position. Stop/new loads invalidate old callbacks. Pausing, seeking,
volume, EOF and audio/subtitle choices use the active engine beneath Dado's UI.
VLC does not report a buffered duration through this API; the UI receives zero
rather than an invented value. VLC errors use -2; startup timeout uses -1001.

Settings → Expertenmodus exposes automatic/Apple/VLC decoding and the optional
Apple full-screen controls. Apple controls force AVPlayer and its supported
formats. Selecting Automatisch restores Dado controls and the automatic engine.
External Android players are also expert choices, not required dependencies.

`prepare_vlc.py` downloads the official archive into ignored SDKs/, verifies its
pinned SHA-256 and caches a verified Vendor framework. Python 3.11+ is required.
The dynamic framework is linked with @executable_path/Frameworks and packaged
as an arm64 slice by package_vlc.py. AltStore/Sideloadly must sign the embedded
framework as well as the executable. `verify_ipa.py` checks the actual zip,
Mach-O load dependency, rpath, framework identity and bundled LGPL notice.
License/source information is in Resources/ThirdPartyNotices.txt. No separate
VLC app is needed. Frameworks stay replaceable rather than being statically
merged into Dado Player.

Run the platform-independent routing tests with:

```sh
swiftc ios/Sources/DadoPlayer/PlaybackPolicy.swift ios/tests/main.swift -o /tmp/dado-playback-policy
/tmp/dado-playback-policy
```

A successful device build verifies compilation/linking/packaging, not playback
on an iPhone. Hardware decoding, provider responses, DRM and codec support still
need real-device verification. Android emulator and TV browser mocks are not
substitutes for that test.
