# PREBUILT-TODO: libmpv.so (arm64-v8a)

This directory is where `cargo ndk` and the CI place native libraries:

- `libdadoplayer.so` - built from Rust by `cargo ndk` in CI (see "Rust .so naming")
- `libmpv.so`        - **PREBUILT - must be supplied** (see below)

No stable direct download URL for a prebuilt arm64 `libmpv.so` is known. Provide
it by one of:

1. Build from source: https://github.com/mpv-android/mpv-android
2. Extract `libmpv.so` from an mpv-android release artifact
3. Commit it here (avoid committing large binaries to git where possible)

The CI step **"PREBUILT-TODO: Provide libmpv.so"** will fail until this file
exists at:

```
android/app/src/main/jniLibs/arm64-v8a/libmpv.so
```
