# Local iOS compilation setup — 2026-11-14

## Initial state before the reboot

- GitHub iOS run 43612366781 was rejected before starting: recent account payments failed or the spending limit needs to be increased. The annotation does not identify which account setting caused it. No reset time is provided.
- Installed Windows Subsystem for Linux 3.0.5 and enabled VirtualMachinePlatform through the official elevated `wsl.exe --install -d Ubuntu-24.04 --no-launch` command. Exit code 0. Windows explicitly requires a reboot before these changes take effect. No reboot was initiated.
- Firmware virtualization is enabled; HypervisorPresent is true. No BIOS change is currently indicated.
- No Linux distribution is installed yet. After reboot, finish installing Ubuntu 24.04, then install the Linux Swift/Rust toolchains and xtool.
- Downloaded official xtool 1.24.0 x86_64 AppImage and checked its SHA-256 against the release API: `dbfd9d7216f077c84c171b688c4173c37c23ca76bf7fd185d1a9ae8a53dab59f`.
- Setup files and the official xtool source checkout are outside the repository under `C:/Users/Tarik/Documents/Codex/2026-11-12/fi/work/xtool-local`. The elevated installer transcript is `wsl-install.log` in that directory.
- No Xcode XIP was found in Downloads. Obtain the official Xcode release supported by xtool from Apple; downloading requires the user's Apple account authentication and license acceptance.

## Plan recorded before the reboot

1. Reboot Windows at the user's convenience; install/start Ubuntu 24.04 without automatically restarting the PC.
2. Install the Linux Swift toolchain required by the pinned xtool release, Rust 1.99.1 with aarch64-apple-ios, and required compiler/archiver tools.
3. Install the Darwin SDK from the official Xcode XIP. xtool 1.24.0 provides `xtool sdk install <path> --slim` separately from developer-account login. An unsigned IPA for AltStore does not require logging into Apple through xtool.
4. Resolve the Darwin SDK sysroot and tools for Rust C dependencies and Swift linking. The existing script's `--swift-sdk darwin` assumption needs validation against this xtool release.
5. Keep macOS storyboard compilation intact; implement Linux packaging without calling macOS executables. Verify the launch configuration still enables the complete iPhone/iPad display area. Do not reuse an old executable or silently ignore missing launch resources.
6. Build current iOS 0.2.5 / build 7, verify ABI symbols, device architecture, bundle metadata and archive contents. Only then publish the new IPA to GitHub Releases and update the release index. Signing and real iPhone playback remain separate checks.

No new IPA has been produced by this setup step. Existing releases and source code are preserved.

## Continued after the user's reboot

- Ubuntu 24.04 installed and started successfully. Local build user: `dado-build`.
- Installed Linux Swift 6.4.0, Rust 1.99.1, the aarch64-apple-ios target, clang/LLVM and native dependencies.
- Moved the user's `Xcode_27.3_beta.xip` from Downloads to `ios/SDKs/`. The directory and XIP files are ignored by Git. The XAR Content SHA-1 matches the archive metadata (`53a622bf007cdb198d87f3552cecd7aca30b5a80`).
- xtool's filtered XIP extraction crashed with SIGILL. Building upstream unxip in debug configuration and extracting the complete app succeeded; `xtool sdk install <extracted Xcode-beta.app> --slim` then installed iPhoneOS 27.3, iPhoneSimulator 27.3 and MacOSX 27.3 SDKs successfully. No Apple-account credentials were needed.
- A standalone UIKit/AVFoundation Swift package compiled and linked to a Mach-O arm64 iOS executable with minimum iOS 15.0. This validates the SDK/toolchain, not Dado playback or device installation.
- SwiftPM's native backend requires the artifact query `darwin` with the bare `arm64-apple-ios` triple. The platform triple used as a query was not found. The first complete Rust device build is in progress; its cache is under `/home/dado-build/.cache/dadoplayer/target`, separate from Windows artifacts.

## Completed device package

The final `bash ios/build.sh --ipa` run completed successfully under WSL. It builds the current Rust static library, verifies all three Rust exports and all three Swift imports, links the UIKit/AVPlayer host, packages resources and verifies ZIP integrity. The final script was run again successfully after the ABI and NTFS-copy corrections.

- Version: **0.2.5 / build 7**, bundle ID `com.tarik.dadoplayer`.
- Local artifact: `Releases/Dado-Player-unsigned.ipa`, **5,721,193 bytes**.
- IPA SHA-256: `68d99f925f567278a624b40321b7cc2fc227402a84970f492284c2a0e5597617`.
- Executable SHA-256: `476b7b528e9cf3196f47933d2b7018d0c44e3baf1b61d21e53fc70ef28ca0c11`.
- Mach-O arm64 device executable, minimum iOS 15.0, standard Apple framework/dylib references and runtime paths. Rust/Swift bridge symbols are defined in the final executable.
- Info.plist has correct source version, iPhone/iPad device family, iPhoneOS 27.3 SDK metadata, and a UILaunchScreen dictionary without a dangling storyboard reference. Icons and bundled Playback.mp4 are present.
- Windows/macOS packaging keeps its existing storyboard path. Linux uses the iOS 14+ Info.plist launch-screen mechanism because ibtool cannot run on Linux.
- Release organizer now includes a current IPA only when both its version and build match source metadata, retaining previous packages in the archive.

Validation limits: no Apple-account provisioning/signature, iPhone installation, playback, rotation or screen-layout test was performed here. AltStore/Sideloadly must sign this artifact with the user's Apple ID. The cross-linker emits warnings about platform support while reading the newer SDK stubs; the resulting device architecture, minimum OS, dependencies and bridge symbols were inspected, but actual runtime behavior requires the iPhone test. SwiftPM's native backend also emits a deprecation notice. Neither warning is represented as hardware validation.

## Primary references

- [GitHub billing lock troubleshooting](https://docs.github.com/en/billing/how-tos/troubleshooting/locked-account): payment processing can take up to 24 hours after a successful payment; this is not a guaranteed wait/reset period for an unresolved restriction.
- [Microsoft WSL installation](https://learn.microsoft.com/en-us/windows/wsl/install)
- [xtool Linux/Windows installation](https://github.com/xtool-org/xtool/blob/1.24.0/Documentation/xtool.docc/Installation-Linux.md)
- [xtool 1.24.0 SDK commands](https://github.com/xtool-org/xtool/blob/1.24.0/Sources/XToolSupport/SDKCommand.swift)
- [Apple Xcode downloads](https://developer.apple.com/download/all/?q=Xcode)
