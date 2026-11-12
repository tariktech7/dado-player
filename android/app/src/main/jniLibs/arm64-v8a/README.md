# Android arm64 native libraries

The Android CI workflow builds `libdadoplayer.so` with `cargo ndk` and fetches
`libmpv.so` plus its arm64 runtime dependencies from the pinned mpv-android
release APK. These downloaded binaries are generated during CI and should not be
committed to this directory.

For a local build, populate this directory with the same native libraries before
running Gradle. `libmpv.so` must be present alongside the `.so` dependencies
extracted from the matching mpv-android APK.
