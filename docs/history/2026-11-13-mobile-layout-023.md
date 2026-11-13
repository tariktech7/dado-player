# Mobile layout release — 2026-11-13

Implementation source: 182c41238ce2cfc8d43677346ab969d9a822addf.
Release: https://github.com/tariktech7/dado-player/releases/tag/mobile-layout-023

## Delivered packages

- Android 0.2.3, versionCode 7. Stable project signer, v2/v3 verification and zip alignment passed. SHA-256: 54257b8cc249097dbc7e5150e7ee5aeb8ed0ad9d860a5dc64d9327d851bd5fa5.
- Windows portable 0.5.1. SHA-256: 24e67c88d9a88d900a53d011066f50a61ef83e27e7ef8d9563df38e3ecbd7ff3. Fresh-folder extraction (11 files), launch, video playback log and clean exit verified. Existing user application was left running.
- iOS asset is UNCHANGED 0.2.1 / build 4, included only so the existing updater can identify the latest actually available iOS build. SHA-256: 230f1b1047acdbc0c0c6afdc67e9f0c22aeb502596173621eb7de0481082bb7b. It contains NONE of this release's new UI fixes.

## Validation and limits

36 Rust tests passed, including responsive mobile home at 320×568, 393×852, 440×956 and 852×393 logical points. Formatting, Clippy with warnings denied for project targets and ios-check passed. Android emulator confirmed update installation, compact logo cards, separate favorites, full-height home and direct entry into playback. The emulator showed video rendering artifacts; no physical Android playback test was performed in this pass.

The new iOS sources target 0.2.3 / build 5 but have NOT passed Swift compilation, simulator pixel validation or hardware tests. GitHub Actions rejects all runner starts with: "The job was not started because recent account payments have failed or your spending limit needs to be increased." Initial failed run: 43489126211; subsequent source pushes also failed before execution. No new IPA was fabricated or relabeled.

After GitHub Billing & plans is resolved, run iOS workflow on the current main: build device IPA, inspect home/video/HLS simulator artifacts and verify full-window geometry. Confirm Dado controls over visible video and Apple Player option on real hardware before publishing ios-build-5. Preserve profiles and use the same Apple-ID signing identity when updating through AltStore.
