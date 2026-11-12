#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Dado-Player - APK im Android-Emulator testen (Windows, Git-Bash)
#
# Einmalige Vorbereitung:
#   sdkmanager --install "emulator" "system-images;android-34;google_apis;x86_64"
#
# Aufruf:
#   bash android/emulator-test.sh            # headless: bootet, installiert, screenshots, beendet
#   HEADLESS=0 bash android/emulator-test.sh # mit sichtbarem Emulator-Fenster
#
# Ergebnisse in .workbuddy-ai/:
#   emulator-1-login.png    Screenshot direkt nach dem Start
#   emulator-2-landscape.png Screenshot nach Drehung ins Querformat
#   emulator-logcat.txt     vollstaendiges Logcat
#   emulator-stdout.log     Emulator-Ausgabe
# ---------------------------------------------------------------------------
set -uo pipefail

SDK="${ANDROID_SDK_ROOT:-/c/Users/Tarik/AppData/Local/Temp/android-build/sdkroot}"
export ANDROID_SDK_ROOT="$SDK" ANDROID_HOME="$SDK"
ADB="$SDK/platform-tools/adb.exe"
EMU="$SDK/emulator/emulator.exe"
AVDM="$SDK/cmdline-tools/latest/bin/avdmanager.bat"
AVD_NAME="${AVD_NAME:-dado_test}"
IMAGE="system-images;android-34;google_apis;x86_64"
PKG="com.tarik.dadoplayer"
ACTIVITY="$PKG/.GameActivity"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APK="$ROOT/Dado-Player-arm64-debug.apk"
OUT="$ROOT/.workbuddy-ai"
SERIAL="emulator-5554"

mkdir -p "$OUT"
adbs() { "$ADB" -s "$SERIAL" "$@"; }

# --- 1. AVD anlegen (idempotent) -------------------------------------------
if ! "$EMU" -list-avds 2>/dev/null | grep -qx "$AVD_NAME"; then
  echo "==> Erzeuge AVD '$AVD_NAME'"
  export JAVA_HOME="${JAVA_HOME:-/c/Users/Tarik/AppData/Local/Temp/android-build/jdk/jdk-17.0.21+9}"
  echo "no" | "$AVDM" create avd -n "$AVD_NAME" -k "$IMAGE" -d "pixel_6" --force || exit 1
fi
CFG="$HOME/.android/avd/$AVD_NAME.avd/config.ini"
if [ -f "$CFG" ]; then
  grep -q '^hw\.ramSize=' "$CFG" || echo 'hw.ramSize=3072' >> "$CFG"
  sed -i 's/^hw\.ramSize=.*/hw.ramSize=3072/' "$CFG"
  sed -i 's/^hw\.keyboard=.*/hw.keyboard=yes/' "$CFG"
fi

# --- 2. Emulator starten ---------------------------------------------------
EXTRA=()
[ "${HEADLESS:-1}" = "1" ] && EXTRA+=(-no-window)
echo "==> Starte Emulator ${EXTRA[*]:-}"
"$EMU" -avd "$AVD_NAME" -no-snapshot -no-boot-anim -no-audio \
       -gpu swiftshader_indirect -memory 3072 \
       -netdelay none -netspeed full "${EXTRA[@]}" \
       > "$OUT/emulator-stdout.log" 2>&1 &
EMU_PID=$!
trap 'kill $EMU_PID 2>/dev/null; adbs emu kill 2>/dev/null' EXIT

echo "==> Warte auf Boot ..."
"$ADB" wait-for-device
BOOT=""
for _ in $(seq 1 200); do
  BOOT="$(adbs shell getprop sys.boot_completed 2>/dev/null | tr -d '\r\n')"
  [ "$BOOT" = "1" ] && break
  sleep 2
done
[ "$BOOT" = "1" ] || { echo "!! Boot fehlgeschlagen - siehe $OUT/emulator-stdout.log"; exit 1; }
adbs shell settings put global window_animation_scale 0 >/dev/null 2>&1
adbs shell settings put global transition_animation_scale 0 >/dev/null 2>&1
adbs shell settings put global animator_duration_scale 0 >/dev/null 2>&1
echo "==> Boot OK ($(adbs shell getprop ro.build.version.release | tr -d '\r'))"

# --- 3. Installieren -------------------------------------------------------
echo "==> Installiere APK ($(du -h "$APK" | cut -f1))"
adbs uninstall "$PKG" >/dev/null 2>&1
adbs install -r "$APK" || { echo "!! Installation fehlgeschlagen (ABI?)"; exit 1; }

# --- 4. Starten ------------------------------------------------------------
adbs logcat -c
adbs shell am start -W -n "$ACTIVITY" 2>&1 | grep -E "Status|TotalTime|Error" 
sleep 10
adbs exec-out screencap -p > "$OUT/emulator-1-login.png"

# --- 5. Querformat (die App ist eine TV-Oberflaeche) -----------------------
adbs shell settings put system accelerometer_rotation 0 >/dev/null 2>&1
adbs shell settings put system user_rotation 1 >/dev/null 2>&1
sleep 6
adbs exec-out screencap -p > "$OUT/emulator-2-landscape.png"

# --- 6. Logs ---------------------------------------------------------------
adbs logcat -d > "$OUT/emulator-logcat.txt" 2>/dev/null
echo "==> Screenshots: .workbuddy-ai/emulator-1-login.png / emulator-2-landscape.png"
echo "==> Prozess laeuft: $(adbs shell pidof "$PKG" | tr -d '\r')"
echo "==> Fehler im Log (falls vorhanden):"
grep -iE "FATAL EXCEPTION|AndroidRuntime: |panicked at|libmpv|mpv.*error|UnsatisfiedLinkError|dlopen failed" \
     "$OUT/emulator-logcat.txt" | head -20
echo "==> Fertig."
