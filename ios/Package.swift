// swift-tools-version: 5.9
//
// Package.swift — Dado Player iOS host.
//
// Two targets:
//
//   DadoBridge  .systemLibrary  path: Sources/DadoHost  -> `import DadoBridge`
//               Exposes include/DadoBridge.h (immutable, shared with the Rust
//               core) through Sources/DadoHost/module.modulemap. A system
//               library target is used because SwiftPM has no bridging headers
//               outside Xcode: a module map is the only portable way to make a
//               C header visible to Swift on Linux/WSL.
//
//   DadoPlayer  .executableTarget  path: Sources/DadoPlayer
//               The UIKit host: main.swift (top-level entry), AppDelegate,
//               RootViewController, VideoView, HostBridge.
//
// The Rust core is NOT built by SwiftPM. `ios/build.sh` builds
// libdadoplayer.a for aarch64-apple-ios and copies it into ios/Vendor/; the
// linker settings below pick it up by name. There is deliberately no
// .xcodeproj — xtool drives SwiftPM directly.
//
// Tools version 5.9 (Swift 5 language mode) on purpose: Swift 6 mode promotes
// actor-isolation warnings to errors, and every UIKit entry point in this host
// is main-thread-by-construction rather than actor-annotated.

import Foundation
import PackageDescription

// Absolute path to ios/Vendor. A relative "-LVendor" only resolves when the
// linker happens to run with the package root as its working directory, which
// is not guaranteed to be the same for `swift build` and for xtool. Deriving it
// from #filePath removes the assumption instead of documenting it.
let vendorPath = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()
    .appendingPathComponent("Vendor")
    .path

let package = Package(
    name: "DadoPlayer",
    platforms: [
        .iOS(.v15)
    ],
    products: [
        .executable(name: "DadoPlayer", targets: ["DadoPlayer"])
    ],
    targets: [
        // System library targets have no `publicHeadersPath` parameter: SwiftPM
        // always uses `include/` for headers and looks for `module.modulemap` in
        // the target root. Both are already in that shape.
        .systemLibrary(
            name: "DadoBridge",
            path: "Sources/DadoHost"
        ),
        .executableTarget(
            name: "DadoPlayer",
            dependencies: ["DadoBridge"],
            path: "Sources/DadoPlayer",
            swiftSettings: [.unsafeFlags(["-F\(vendorPath)"])],
            linkerSettings: [
                // libdadoplayer.a, copied here by ios/build.sh.
                .unsafeFlags(["-L\(vendorPath)", "-ldadoplayer"]),
                .unsafeFlags(["-F\(vendorPath)", "-framework", "MobileVLCKit", "-Xlinker", "-ObjC"]),
                .unsafeFlags(["-Xlinker", "-rpath", "-Xlinker", "@executable_path/Frameworks"]),
                .linkedLibrary("c++"), .linkedLibrary("xml2"),
                .linkedLibrary("z"), .linkedLibrary("bz2"), .linkedLibrary("iconv"),
                .linkedFramework("AudioToolbox"), .linkedFramework("VideoToolbox"),
                .linkedFramework("CFNetwork"), .linkedFramework("CoreText"),
                .linkedFramework("OpenGLES"), .linkedFramework("Accelerate"),

                // Frameworks the Swift host references, plus the ones the Rust
                // core pulls in transitively:
                //   Metal            wgpu's iOS backend (eframe renderer)
                //   CoreFoundation   Foundation-level types used by winit/wgpu
                //   CoreGraphics     CGGeometry in winit + AVFoundation
                //   Security         rustls/ring keychain-free crypto primitives
                //   CoreVideo        AVPlayerLayer pixel buffers
                .linkedFramework("UIKit"),
                .linkedFramework("AVFoundation"),
                .linkedFramework("AVKit"),
                .linkedFramework("QuartzCore"),
                .linkedFramework("CoreMedia"),
                .linkedFramework("Metal"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("CoreGraphics"),
                .linkedFramework("CoreVideo"),
                .linkedFramework("Security")
            ]
        )
    ]
)
