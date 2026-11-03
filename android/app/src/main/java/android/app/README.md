# GameActivity.java — DONE

This file is the required Java glue for the Android entry activity. It is a thin
subclass of the AndroidX Games Activity base class, which is provided by the
`androidx.games:games-activity:2.0.2` AAR (declared in
`android/app/build.gradle`). That AAR also contributes the native
`libandroid_game_activity.so` that loads `libdadoplayer.so` and calls the Rust
`android_main` entry point.

No manual copying from the `android-game-activity` crate source is needed — the
dependency brings everything required. The manifest references
`android.app.GameActivity`, which matches this class's package.

If you switch the eframe backend to `NativeActivity` instead, delete this file
and point the manifest at `android.app.NativeActivity`.
