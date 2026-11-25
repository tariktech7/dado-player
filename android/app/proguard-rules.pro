# ProGuard / R8 rules for Dado Player.
#
# The release build enables shrinking (see build.gradle). That is safe ONLY
# with the keep rules below, because two JNI surfaces are resolved by *name*
# at runtime and R8 would otherwise rename or delete them:
#
#  1. Java -> native. `GameActivity` declares
#     `private static native void nativeOnKey(int,int)` etc. The Rust side
#     exports `Java_com_tarik_dadoplayer_GameActivity_nativeOnKey`. The symbol
#     name is derived from package, class and method name, so none of the
#     three may change.
#
#  2. native -> Java. `src/platform/android/jni_bridge.rs` calls static
#     methods on GameActivity by string:
#       setImeFields, seedIme, prepareEmbeddedVlc
#     A renamed method makes those calls fail with NoSuchMethodError at
#     runtime - the app would compile, install and then crash on the first
#     keystroke or the first playback.
#
# Keeping the whole class covers both directions and costs nothing: it is a
# single class with a handful of methods.

-keep class com.tarik.dadoplayer.GameActivity { *; }

# Activities are looked up by string from the manifest; keep the package.
-keep class com.tarik.dadoplayer.** { *; }

# Games Activity / GameTextInput glue is reached reflectively from its own
# native library. Its AAR ships consumer rules, but those only apply when the
# AAR is consumed as a library - keep it explicitly so a future AGP change
# cannot silently drop it.
-keep class com.google.androidgamesdk.** { *; }

# R8 warns about optional annotations that are not on the classpath.
-dontwarn org.jetbrains.annotations.**
