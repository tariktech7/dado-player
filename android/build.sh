#!/usr/bin/env bash
# Build Dado-Player-arm64-debug.apk. Probe tooling - not part of the app.
#
# Two stages, because the Rust cdylib has to exist before Gradle packages it:
#   1. cargo ndk --release  -> android/app/src/main/jniLibs/arm64-v8a/*.so
#   2. gradlew assembleDebug -> app-debug.apk
#
# `--release` matches the build the QA captures were taken on. A dev-profile
# build is 147 MB and unrepresentative of frame timings.
# `pipefail` matters: without it `gradlew ... | tail` reports tail's status, a
# failed build looks like success, and the copy below ships the *previous*
# APK under a fresh timestamp. That is exactly how a wrong md5 gets into a
# report.
set -eu
set -o pipefail

SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${LOCALAPPDATA:?}/Temp/android-build/sdkroot}}"
JDK="${JAVA_HOME:-${LOCALAPPDATA:?}/Temp/android-build/jdk/jdk-17.0.21+9}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$SDK/ndk/27.0.12077973}"
export JAVA_HOME="$JDK"
export PATH="$JAVA_HOME/bin:$SDK/platform-tools:$PATH"
# Use explicit build-tools paths: relying on PATH to resolve the `.bat`/`.exe`
# from Git Bash is unreliable in this shell (apksigner.bat came back as
# "command not found" even though it exists).
BT="$SDK/build-tools/34.0.0"

cd "$ROOT"

echo "==> cargo ndk (release, arm64-v8a)"
# No `| tail` here: it used to hide the actual Rust compiler error while the
# outer `| tail -35` swallowed the failing exit code. Redirect this whole
# script to a log file when running QA instead.
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --locked --release --lib

echo "==> gradlew assembleDebug"
(cd android && ./gradlew assembleDebug)

APK_IN="android/app/build/outputs/apk/debug/app-debug.apk"
[ -f "$APK_IN" ] || { echo "!! $APK_IN missing"; exit 1; }

# ---------------------------------------------------------------------------
# SIGNING
#
# Gradle produced app-debug.apk signed with the machine-local debug key (or
# unsigned). We re-sign it ONCE with the project-owned, stable key so every
# APK we hand out shares the same certificate. That is what makes an update
# install over a previous build instead of failing with
# INSTALL_FAILED_UPDATE_INCOMPATIBLE ("App was not installed").
#
# Key material: android/app/dadoplayer-release.pk8 (PKCS#8) +
#               android/app/dadoplayer-release.x509.pem (self-signed).
# ---------------------------------------------------------------------------
SIGN_KEY="android/app/dadoplayer-release.pk8.der"
SIGN_CERT="android/app/dadoplayer-release.x509.pem"
[ -f "$SIGN_KEY" ] || { echo "!! signing key missing: $SIGN_KEY"; exit 1; }
[ -f "$SIGN_CERT" ] || { echo "!! signing cert missing: $SIGN_CERT"; exit 1; }

echo "==> re-sign with stable project key (apksigner)"
# apksigner must run AFTER zipalign; AGP's assembleDebug output is already
# 4 KiB-aligned, so signing in place is safe and preserves alignment.
"$BT/apksigner.bat" sign --key "$SIGN_KEY" --cert "$SIGN_CERT" "$APK_IN"

echo "==> verify signature"
"$BT/apksigner.bat" verify --verbose "$APK_IN" | sed -n '1,12p'
echo "    signer cert:"
"$BT/apksigner.bat" verify --print-certs "$APK_IN" 2>&1 | grep -i "certificate DN" || true

echo "==> verify 4 KiB alignment (required for extractNativeLibs=false)"
"$BT/zipalign.exe" -c -v 4 "$APK_IN" >/dev/null && echo "    zipalign 4 KiB: OK" || {
    echo "!! zipalign check failed - re-aligning"; exit 1; }

echo "==> verify extractNativeLibs=false is in the merged manifest"
if "$BT/aapt.exe" dump xmltree "$APK_IN" AndroidManifest.xml 2>/dev/null | grep -q "extractNativeLibs.*0x0"; then
    echo "    extractNativeLibs=false: OK (libs are mmap'd, no 41 MB extraction)"
else
    echo "!! extractNativeLibs not false - install will extract 41 MB and be slow"
fi

echo "==> versionCode / package"
"$BT/aapt.exe" dump badging "$APK_IN" 2>/dev/null | grep -iE "package:|versionCode" || true

mkdir -p Releases
cp "$APK_IN" Releases/Dado-Player-arm64-debug.apk
cp Releases/Dado-Player-arm64-debug.apk Dado-Player-arm64-debug.apk
echo "==> Dado-Player-arm64-debug.apk"
md5sum Dado-Player-arm64-debug.apk
ls -la Dado-Player-arm64-debug.apk
