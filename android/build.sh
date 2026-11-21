#!/usr/bin/env bash
# Build Dado-Player-arm64-debug.apk. Probe tooling - not part of the app.
#
# Two stages, because the Rust cdylib has to exist before Gradle packages it:
#   1. cargo ndk --release  -> android/app/src/main/jniLibs/arm64-v8a/*.so
#   2. gradlew assembleRelease -> app-release-unsigned.apk -> signed APK
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
python scripts/release_version.py --check

echo "==> cargo ndk (release, arm64-v8a)"
# No `| tail` here: it used to hide the actual Rust compiler error while the
# outer `| tail -35` swallowed the failing exit code. Redirect this whole
# script to a log file when running QA instead.
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --locked --release --lib

echo "==> gradlew assembleRelease"
(cd android && ./gradlew assembleRelease)

APK_IN="android/app/build/outputs/apk/release/app-release-unsigned.apk"
APK_OUT="android/app/build/outputs/apk/release/dado-player-signed.apk"
[ -f "$APK_IN" ] || { echo "!! $APK_IN missing"; exit 1; }

# ---------------------------------------------------------------------------
# SIGNING
#
# Sign Gradle's unsigned release variant with the existing distribution key.
# Keeping this signer lets users update installations of the earlier beta.
#
# Key material: android/app/dadoplayer-release.pk8 (PKCS#8) +
#               android/app/dadoplayer-release.x509.pem (self-signed).
# ---------------------------------------------------------------------------
SIGN_KEY="android/app/dadoplayer-release.pk8.der"
SIGN_CERT="android/app/dadoplayer-release.x509.pem"
[ -f "$SIGN_KEY" ] || { echo "!! signing key missing: $SIGN_KEY"; exit 1; }
[ -f "$SIGN_CERT" ] || { echo "!! signing cert missing: $SIGN_CERT"; exit 1; }

echo "==> sign with stable project key (apksigner)"
"$BT/apksigner.bat" sign --key "$SIGN_KEY" --cert "$SIGN_CERT" --out "$APK_OUT" "$APK_IN"

echo "==> verify signature"
"$BT/apksigner.bat" verify --verbose "$APK_OUT" | sed -n '1,12p'
echo "    signer cert:"
"$BT/apksigner.bat" verify --print-certs "$APK_OUT" 2>&1 | grep -i "certificate DN" || true

echo "==> verify 4 KiB alignment (required for extractNativeLibs=false)"
"$BT/zipalign.exe" -c -v 4 "$APK_OUT" >/dev/null && echo "    zipalign 4 KiB: OK" || {
    echo "!! zipalign check failed - re-aligning"; exit 1; }

echo "==> verify extractNativeLibs=false is in the merged manifest"
if "$BT/aapt.exe" dump xmltree "$APK_OUT" AndroidManifest.xml 2>/dev/null | grep "extractNativeLibs.*0x0" >/dev/null; then
    echo "    extractNativeLibs=false: OK (libs are mmap'd, no 41 MB extraction)"
else
    echo "!! extractNativeLibs not false - install will extract 41 MB and be slow"
fi

echo "==> versionCode / package"
badging=$("$BT/aapt.exe" dump badging "$APK_OUT")
if [[ "$badging" == *application-debuggable* ]]; then
    echo "!! The distribution APK must not be debuggable"; exit 1
fi
printf '%s\n' "$badging" | grep '^package:'

mkdir -p Releases
cp "$APK_OUT" Releases/Dado-Player-arm64-debug.apk
cp Releases/Dado-Player-arm64-debug.apk Dado-Player-arm64-debug.apk
echo "==> Dado-Player-arm64-debug.apk"
md5sum Dado-Player-arm64-debug.apk
ls -la Dado-Player-arm64-debug.apk
