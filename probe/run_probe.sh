#!/usr/bin/env bash
# Visual-QA probe driver for dado_test. Probe tooling only - not part of the app.
#
# Why a script: every step below was derived by hand (tap coordinates from a
# 1080x2400 capture at 420 dpi, plus sleeps long enough for a frame under
# libndk_translation), and re-deriving them per run is how wrong evidence gets
# produced.
#
# IMPORTANT: background processes do not survive the end of a shell turn here.
# The emulator and the stub both die when the turn ends, so `all` boots the
# emulator, starts the stub, installs and probes in one shot. Budget ~4 min.
#
# Usage:
#   ./run_probe.sh all [APK]    boot emulator + stub, install APK, probe login, capture
#   ./run_probe.sh login        fill the form and submit (emulator already up)
#   ./run_probe.sh shot NAME    capture probe/NAME.png
#   ./run_probe.sh landscape [NAME] / portrait [NAME]

set -u
SDK="C:/Users/Tarik/AppData/Local/Temp/android-build/sdkroot"
ADB="$SDK/platform-tools/adb.exe"
EMU="$SDK/emulator/emulator.exe"
OUT="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$OUT/.." && pwd)"
APK="${2:-$ROOT/Dado-Player-arm64-debug.apk}"
PKG=com.tarik.dadoplayer
ACT=$PKG/.GameActivity

# Login form, physical pixels, portrait 1080x2400 @ 420 dpi.
HOST_Y=999
USER_Y=1235
PASS_Y=1539
SUBMIT_Y=2088
HEADER_Y=250      # neutral area: tapping it surrenders focus and closes the IME

# Once a profile is saved, the whole form sits 302 px lower - the profile row
# and its section label push it down. Measured off
# 94_persist_03_after_relaunch.png by scanning the field background colour at
# x=1000: name 958..1090, host 1222..1354, user 1486..1618, pass 1750..1882,
# Anmelden 1996..2131. Reusing the no-profile constants here taps the *previous*
# field, which looks like a text-entry bug in the app and is not one.
NAME_Y_P=1024
HOST_Y_P=1288
USER_Y_P=1552
PASS_Y_P=1816
SUBMIT_Y_P=2063
ROW_Y_P=543       # saved-profile row centre (avatar centre y=544)

sh_() { "$ADB" shell "$@"; }

shot() {
  local name="$1"
  "$ADB" exec-out screencap -p > "$OUT/$name.png" 2>/dev/null
  local sz
  sz=$(wc -c < "$OUT/$name.png" 2>/dev/null || echo 0)
  echo "  $name.png  ${sz} bytes"
  # A capture of a few KB is a flat screen; that is how a launcher shot once got
  # cited as an app screenshot. Fail loudly instead of citing it.
  if [ "$sz" -lt 20000 ]; then
    echo "  WARN: under 20 KB - probably blank or the launcher. VIEW IT before citing."
  fi
}

alive() {
  local p
  p=$(sh_ pidof $PKG 2>/dev/null | tr -d '\r')
  if [ -z "$p" ]; then echo "  DEAD (no pid)"; return 1; fi
  echo "  pid $p"
}

start_stub() {
  if curl -s -m 2 "http://127.0.0.1:8899/player_api.php?username=probe&password=probe" >/dev/null 2>&1; then
    echo "[stub] already up"
    return
  fi
  ( cd "$ROOT" && nohup python probe/xtream_stub.py > probe/stub.log 2>&1 & )
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    sleep 1
    curl -s -m 2 "http://127.0.0.1:8899/player_api.php?username=probe&password=probe" >/dev/null 2>&1 && { echo "[stub] up"; return; }
  done
  echo "[stub] FAILED to start"; tail -5 "$OUT/stub.log"
}

start_emu() {
  if "$ADB" devices 2>/dev/null | grep -q emulator-5554; then
    echo "[emu] already running"
    return
  fi
  echo "[emu] booting"
  ( "$EMU" -avd dado_test -gpu swiftshader_indirect -no-audio -no-boot-anim \
      -memory 3072 -no-window -no-snapshot -port 5554 > "$OUT/emu.log" 2>&1 & )
  "$ADB" wait-for-device
  for _ in $(seq 1 60); do
    b=$(sh_ getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    [ "$b" = "1" ] && { echo "[emu] booted"; return; }
    sleep 5
  done
  echo "[emu] TIMED OUT waiting for boot"; tail -20 "$OUT/emu.log"; return 1
}

# The first frames after a cold start are black on this emulator: the GL surface
# is not up yet and `screencap` returns ~19 KB of flat black. A tap into that
# state silently misses, and the run then "loses" a dialog that was never
# dismissed. Poll until the capture is big enough to contain UI.
#
# The 90 KB floor is measured, not guessed: a flat black frame is 19 KB, the
# "Dado Player isn't responding" dialog is 50 KB, the login screen is 125 KB and
# the Live list is 158 KB. 40 KB was too low and let an ANR dialog through as if
# it were the app.
#
# Cold start on this emulator takes 5.5-7.7 s (`am start -W` TotalTime over five
# launches; a warm restart is 1.6 s), which is over Android's 5 s input-dispatch
# window, so the ANR dialog is a coin flip on a cold start and has to be cleared
# rather than worked around. It is a property of this emulator - arm64 code
# under libndk_translation with software GL - and is not evidence of a defect on
# a real device.
anr_up() {
  [ "$(sh_ "dumpsys window windows" 2>/dev/null | tr -d '\r' | grep -ci 'Not Responding')" -gt 0 ]
}

wait_ready() {
  local i sz=0 tries=0
  for i in $(seq 1 90); do
    if anr_up; then
      tries=$((tries + 1))
      if [ "$tries" -gt 2 ]; then
        echo "  ANR persists after 2 relaunches - giving up"
        sh_ logcat -d 2>/dev/null | tr -d '\r' | tail -25 | sed 's/^/    /'
        return 1
      fi
      # Tapping "Wait" was tried and did not clear it: the dialog belongs to the
      # app process, so force-stopping the app removes it deterministically. The
      # relaunch after a force-stop is warm (1.6 s measured) and usually does not
      # trip the same 5 s window again.
      echo "  ANR dialog up - force-stop and relaunch (attempt $tries)"
      sh_ "am force-stop $PKG"; sleep 3
      "$ADB" shell am start -n "$ACT" >/dev/null 2>&1
      sleep 10
      continue
    fi
    "$ADB" exec-out screencap -p > /tmp/_drawn.png 2>/dev/null
    sz=$(wc -c < /tmp/_drawn.png 2>/dev/null || echo 0)
    if [ "$sz" -ge 90000 ]; then echo "  ready (${sz} bytes)"; return 0; fi
    sleep 2
  done
  echo "  NOT READY after 180 s (${sz} bytes)"
  # A stuck splash means the Rust side never drew a frame; the tail of logcat is
  # the only way to tell a slow load from a deadlock.
  sh_ logcat -d 2>/dev/null | tr -d '\r' | tail -25 | sed 's/^/    /'
  return 1
}

# Kept for callers that want a one-shot check between steps.
anr_check() {
  if anr_up; then
    echo "  ANR dialog up - tapping Wait"
    sh_ input tap 540 1363; sleep 8
  else
    echo "  no ANR dialog"
  fi
}

launch() {
  # `pm clear` is the default because the interesting states are all "fresh
  # install" states: a persisted profile from an earlier run silently answers
  # questions about the login screen. `--keep` opts out for the runs that
  # deliberately continue from what the last step left behind.
  if [ "${1:-}" != "--keep" ]; then
    echo "  $(sh_ "pm clear $PKG" 2>&1 | tr -d '\r')  (pm clear)"
  fi
  sh_ "am force-stop $PKG; logcat -c"
  "$ADB" shell am start -W -n "$ACT" 2>&1 | grep -E 'TotalTime|Status'
  # A fixed sleep is not enough: under host memory pressure the first cold start
  # after an install can exceed `am start -W`'s wait and the process takes a
  # while to appear. Poll for the pid instead of guessing.
  local i p
  for i in $(seq 1 20); do
    p=$(sh_ pidof $PKG 2>/dev/null | tr -d '\r')
    [ -n "$p" ] && break
    sleep 3
  done
  if [ -z "$p" ]; then echo "  NEVER STARTED (no pid after 60 s)"; return 1; fi
  alive
  # The first cold start after an emulator boot gets stuck on the splash
  # roughly half the time on this image (the process is up, `android_main`
  # never draws). A force-stop and a second start is warm and has not failed
  # yet, so one retry here saves the whole session.
  if ! wait_ready; then
    echo "  first start never drew - force-stop and retry once"
    sh_ "am force-stop $PKG"; sleep 3
    "$ADB" shell am start -n "$ACT" >/dev/null 2>&1
    sleep 10
    wait_ready || return 1
  fi
  # The size check catches "black", not "half painted". Give the first real
  # frame time to finish before anything taps into it.
  sleep 4
  alive
}

# Wait until the IME reports the state we want. `input text` is delivered to
# whatever holds input focus at the moment it runs, so typing before the field
# tap has actually raised the keyboard drops the characters on the window: the
# run then ends at "Host, Benutzer und Passwort sind erforderlich." with every
# tap in the log looking correct. That is a *timing* bug in the probe, not in
# the app, so it is fixed here by polling rather than by a longer sleep.
ime_wait() {  # ime_wait [1|0] [seconds]  1 = shown (default), 0 = hidden
  local want="${1:-1}" limit="${2:-12}" i v=""
  for i in $(seq 1 "$limit"); do
    v=$(sh_ dumpsys input_method 2>/dev/null | tr -d '\r' \
        | grep -m1 'mInputShown=' | sed 's/.*mInputShown=\([a-z]*\).*/\1/')
    if { [ "$want" = 1 ] && [ "$v" = true ]; } || { [ "$want" = 0 ] && [ "$v" = false ]; }; then
      echo "  ime shown=$v"
      return 0
    fi
    sleep 1
  done
  echo "  IME: shown=$v after ${limit}s, wanted $want"
  return 1
}

# Tap a field, wait for the keyboard, type. Retries the tap once: `wait_ready`
# returns on the first frame over 90 KB, and a login screen caught mid-paint
# looks ready while the field under the finger is not there yet.
field() {  # field <x_px> <y_px> <text>
  local i
  for i in 1 2; do
    sh_ input tap "$1" "$2"
    if ime_wait 1 12; then
      sleep 1
      sh_ input text "$3"
      sleep 1
      return 0
    fi
    echo "  field tap missed - retrying"
    sleep 3
  done
  return 1
}

# Fields are filled one at a time with a blur between them: the password field
# sits behind the keyboard (keyboard top ~1433 px) and can only be tapped once
# the IME is down.
login() {
  echo "[login] host"
  field 540 $HOST_Y "10.0.2.2:8899" || return 1
  echo "[login] user"
  field 540 $USER_Y "probe" || return 1
  echo "[login] blur"
  sh_ input tap 540 $HEADER_Y
  ime_wait 0
  echo "[login] password"
  field 540 $PASS_Y "probe" || return 1
  echo "[login] blur"
  sh_ input tap 540 $HEADER_Y
  ime_wait 0
  shot 01_form_filled
  echo "[login] submit"
  sh_ input tap 540 $SUBMIT_Y; sleep 10
  alive
  shot 02_after_login
}

rotate() {
  sh_ "settings put system accelerometer_rotation 0; settings put system user_rotation $1"
  sleep 7
}

# Bottom nav, physical px, portrait. Derived from probe/02_after_login.png:
# Live | Filme | Serien | Favoriten | Mehr at y=2257, x=106/333/540/749/963.
NAV_Y=2257
nav() {
  case "$1" in
    live)   sh_ input tap 106 $NAV_Y ;;
    movies) sh_ input tap 333 $NAV_Y ;;
    series) sh_ input tap 540 $NAV_Y ;;
    fav)    sh_ input tap 749 $NAV_Y ;;
    more)   sh_ input tap 963 $NAV_Y ;;
  esac
  echo "  nav $1"; sleep 6
}

walk() {
  alive || return 1
  shot 10_live_portrait
  nav movies;  shot 11_movies
  nav series;  shot 12_series_list
  nav live;    sleep 2
}

# Content screens that matter for the row-cap test.
# Poster grid is 2 columns; card 1 centre (317,767), card 2 centre (781,767).
# Series 2001 "Die Ermittler" is card 2 and carries the ~300-char plot; series
# 2000 "Tatort Nord" is card 1 and carries the 44-char control.
CARD1_X=317
CARD2_X=781
CARD_Y=767

full() {
  alive || return 1
  echo "[full] live list, scrolled to the non-Latin + overlong rows"
  nav live
  sh_ input swipe 540 1800 540 700 300; sleep 4
  shot 20_live_scrolled
  sh_ input swipe 540 1800 540 700 300; sleep 4
  shot 21_live_bottom

  echo "[full] series 2001 detail (300-char plot)"
  nav series
  sh_ input tap $CARD2_X $CARD_Y; sleep 10
  shot 30_series_long_portrait
  rotate 1; shot 31_series_long_landscape
  rotate 0; sleep 3

  echo "[full] series 2000 detail (44-char control)"
  sh_ input keyevent 4; sleep 5
  sh_ input tap $CARD1_X $CARD_Y; sleep 10
  shot 32_series_short_portrait
}

# The login P0: does the process survive a second field focus?
repro() {
  sh_ "am force-stop $PKG; logcat -c"
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 15
  echo "  after launch:   [$(sh_ pidof $PKG | tr -d '\r')]"
  sh_ input tap 540 $HOST_Y; sleep 3
  echo "  after host tap: [$(sh_ pidof $PKG | tr -d '\r')]"
  sh_ input tap 540 $USER_Y; sleep 5
  echo "  after user tap: [$(sh_ pidof $PKG | tr -d '\r')]"
  echo -n "  jni errors: "
  sh_ logcat -d 2>/dev/null | grep -cE 'ClassNotFoundException|JNI DETECTED|Fatal signal 6'
}

# Isolate the login P0 properly: wipe the saved profile (pm clear) so the app
# really is on the login form, prove it with a capture BEFORE tapping, then run
# the bare two-tap sequence that crashed at 14:51 and 15:01. No reinstall, so
# "fresh install" is not a variable.
repro2() {
  sh_ "pm clear $PKG; am force-stop $PKG; logcat -c"
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 16
  shot 40_repro_login
  echo "  after launch:   [$(sh_ pidof $PKG | tr -d '\r')]"
  sh_ input tap 540 $HOST_Y; sleep 3
  echo "  after host tap: [$(sh_ pidof $PKG | tr -d '\r')]"
  shot 41_repro_host
  sh_ input tap 540 $USER_Y; sleep 5
  echo "  after user tap: [$(sh_ pidof $PKG | tr -d '\r')]"
  shot 42_repro_user
  sh_ input tap 540 $HEADER_Y; sleep 5
  echo "  after blur tap: [$(sh_ pidof $PKG | tr -d '\r')]"
  echo -n "  jni errors: "
  sh_ logcat -d 2>/dev/null | grep -cE 'ClassNotFoundException|JNI DETECTED|Fatal signal 6'
}

# Second pass: the screens the first run did not cover - the Mehr/settings page
# (sentence-case section headers) and a movie detail.
walk2() {
  alive || return 1
  nav more;   shot 50_mehr
  nav movies; shot 51_movies_top
  sh_ input tap $CARD1_X $CARD_Y; sleep 10
  shot 52_movie_detail
  sh_ input keyevent 4; sleep 5
  nav series; shot 53_series_top
}

# Third pass: the Mehr sheet (Suche / Fortsetzen / Einstellungen / Abmelden) and
# the two screens that carry the sentence-case headers.
# Sheet row centres, physical px, portrait: Suche 1516, Fortsetzen 1658,
# Einstellungen 1798, Abmelden 1957.
walk3() {
  alive || return 1
  nav more; sleep 2
  sh_ input tap 540 1798; sleep 8
  shot 60_settings
  sh_ input keyevent 4; sleep 6
  nav more; sleep 2
  sh_ input tap 540 1516; sleep 8
  shot 61_search
}

# Fourth pass: the P1-P5 acceptance set. Four captures, in the order the lead
# asked for them, plus the raw inset log so the numbers are on record.
#
# Tap coordinates are derived, not guessed: the grid is 3 columns of 118 dp at
# pad 16, so column centres are 75 / 205 / 335 dp. The vertical origin is the
# panel top, which is `top_inset + appbar_h`; top_inset is read back from the
# app's own DadoInsets log because the emulator has a notch and hard-coding it
# is how the last round's coordinates drifted.
ppp() { echo "2.625"; }

panel_top_dp() {
  local top
  top=$(sh_ logcat -d -s DadoInsets 2>/dev/null | tr -d '\r' \
        | grep -o 'sys=[0-9]*' | head -1 | cut -d= -f2)
  [ -z "$top" ] && top=128
  awk -v t="$top" 'BEGIN{printf "%.2f", t/2.625 + 56.0}'
}

tap_dp() {  # tap_dp <x_dp> <y_dp>
  sh_ input tap "$(awk -v v="$1" 'BEGIN{printf "%d", v*2.625}')" \
                "$(awk -v v="$2" 'BEGIN{printf "%d", v*2.625}')"
}

p1() {
  alive || return 1
  echo "[p1] inset log:"
  sh_ logcat -d -s DadoInsets 2>/dev/null | tr -d '\r' | tail -4 | sed 's/^/    /'

  nav live;    shot 70_live
  nav movies;  shot 71_movies_grid
  nav series;  shot 72_series_list

  local pt; pt=$(panel_top_dp)
  echo "[p1] panel_top = ${pt} dp"
  # Row 0 image centre: panel_top + 48 (chips) + 8 + 2 + 88.5 (half of 177)
  local r0 r1
  r0=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5}')
  r1=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5 + 229.0}')

  echo "[p1] Tatort Nord (row 0, col 0) at 75,${r0} dp"
  tap_dp 75 "$r0"; sleep 10
  shot 73_series_short_portrait

  # `input keyevent 4` has twice failed to pop the series detail (the next tap
  # then lands on an episode row and starts playback), so back is driven by the
  # app bar chevron, which is an in-app tap. The state after the back key is
  # captured as 73b so the failure is evidence rather than recollection.
  sh_ input keyevent 4; sleep 6
  shot 73b_after_keyevent4
  # App bar back chevron: 48 dp hit area centred at `pad + 24` = 40 dp, and the
  # bar's centre is `top_inset + 28`.
  tap_dp 40 "$(awk -v p="$pt" 'BEGIN{printf "%.1f", p - 56.0 + 28.0}')"; sleep 6
  shot 73c_after_chevron

  echo "[p1] Deep Signal (row 1, col 2) at 335,${r1} dp"
  tap_dp 335 "$r1"; sleep 10
  shot 74_series_long_portrait
  rotate 1; shot 75_series_long_landscape
  rotate 0
}

# ------------------------------------------------------------ persistence ----
#
# `ls -l` proves a file was *written*. It cannot prove a file is *read*: the
# read path recomputes `app_dir()` independently, and that computation can
# return a different answer than the write path did. A passing listing is
# therefore fully compatible with the bug still being present. Three candidates
# predict three different combinations:
#
#   A  path right, write right      -> file present and grows, profile survives
#                                      the relaunch, no banner
#   B  writes here, reads elsewhere -> file present and grows, profile GONE
#                                      after the relaunch, no banner
#   C  write fails                  -> no file, banner visible
#
# The observation that separates A from B is the capture taken *after* the
# relaunch and *before* any tap. Straight after login A and B are identical,
# because the profile is in memory either way.
files() {
  echo "[files] files/dado-player:"
  sh_ "run-as $PKG ls -l files/dado-player" 2>&1 | tr -d '\r' | sed 's/^/    /'
  echo "[files] profiles.json / settings.json anywhere in the data dir:"
  sh_ "run-as $PKG find . -maxdepth 3 \\( -name profiles.json -o -name settings.json \\) -exec ls -l {} \\;" 2>&1 \
    | tr -d '\r' | sed 's/^/    /'
  echo "[files] what the app says it resolved:"
  sh_ logcat -d 2>/dev/null | tr -d '\r' | grep -i 'dado:' | tail -3 | sed 's/^/    /'
}

# Growth with an identified cause. `ls -l` showing a file of some size proves
# one write landed; it says nothing about the next one. Selecting a channel
# writes `last_channel` into the profile (`App::select_channel`), so a single
# tap on a channel row must make profiles.json strictly larger - and it must do
# so *after* a restart, into the same file the read path just read from.
#
# Channel row 0: the 44 dp avatar tile spans 618..729 px at x=60 in
# 70_live.png, pitch 189 px (72 dp), so the row centre is y=674.
grow() {
  echo "[grow] S1, before the channel tap"
  files
  echo "[grow] tap channel row 0"
  sh_ input tap 540 674; sleep 8
  alive
  echo "[grow] S2, after selecting a channel - profiles.json must be larger"
  files
}

# Step 1-3 of the acceptance: fresh data, log in with a named profile, prove
# something was written, force-stop, relaunch, capture the screen it opens on.
persist() {
  echo "== 1. pm clear + launch (fresh data dir)"
  launch || return 1
  shot 94_persist_00_login

  echo "== 1b. log in with a named profile"
  sh_ input tap 540 722
  ime_wait 1 || return 1
  sleep 1; sh_ input text "Wohnzimmer"; sleep 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $HOST_Y "10.0.2.2:8899" || return 1
  field 540 $USER_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $PASS_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  shot 94_persist_01_form
  sh_ input tap 540 $SUBMIT_Y; sleep 12
  alive
  shot 94_persist_02_logged_in

  echo "== 2. what landed on disk (S1) - and: is a banner visible?"
  files

  echo "== 3. force-stop, relaunch - this is the assertion"
  sh_ "am force-stop $PKG"; sleep 3
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 14
  alive
  # No taps before this capture: it has to show what the app opened on.
  shot 94_persist_03_after_relaunch
}

# Step 4: a second, known write. Growth with an identified cause is the only
# form of "the file grows" that means anything - a size that never changes
# proves one write happened and nothing about the next one.
persist2() {
  echo "== 4. second profile -> profiles.json must grow (S2 > S1)"
  echo "   (runs on a screen that already lists a profile: use the _P offsets)"
  sh_ input tap 540 $NAME_Y_P
  ime_wait 1 || return 1
  sleep 1; sh_ input text "Kueche"; sleep 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $HOST_Y_P "10.0.2.2:8899" || return 1
  field 540 $USER_Y_P "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $PASS_Y_P "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  shot 94_persist_04_form_second
  sh_ input tap 540 $SUBMIT_Y_P; sleep 12
  alive
  shot 94_persist_05_second_profile
  files
}

# Sign in by tapping the saved profile rather than retyping the form. One tap
# instead of four typed fields: the form scrolls itself to keep the focused
# field above the keyboard, so absolute y coordinates for the third and fourth
# fields are not stable and the run dies waiting for an IME that never opens.
signin() {
  echo "[signin] tap the saved profile row"
  sh_ input tap 540 $ROW_Y_P; sleep 12
  anr_check
  alive
  shot C3_live_after_signin
}

# The confirm dialog, reached from the login screen's trash glyph.
dialog() {
  echo "[dialog] tap the trash glyph on the saved-profile row"
  sh_ input tap 1017 $ROW_Y_P; sleep 5
  shot A1_confirm_dialog
}

# Back must cancel the dialog and leave the activity alone.
dialog_back() {
  echo "[dialog_back] keyevent 4 with the dialog open"
  sh_ input keyevent 4; sleep 5
  alive
  shot A2_dialog_after_back
}

# Sign in from the form while a profile row is present, using the _P offsets.
# The name field is left empty: submitting the same host/user/password updates
# the existing profile instead of adding a second one.
formp() {
  field 540 $HOST_Y_P "10.0.2.2:8899" || return 1
  field 540 $USER_Y_P "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $PASS_Y_P "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  shot C1_formp_filled
  sh_ input tap 540 $SUBMIT_Y_P; sleep 14
  alive
  shot C2_formp_after_login
}

# Series detail on the persistence build: the hero's meta line, the conditional
# episode subtitle, and an app bar that no longer repeats the title.
meta() {
  alive || return 1
  local pt; pt=$(panel_top_dp)
  local r0; r0=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5}')
  nav series; shot B0_series_list
  echo "[meta] card 0 at 75,${r0} dp"
  tap_dp 75 "$r0"; sleep 10
  shot B1_series_detail_portrait
  rotate 1; shot B2_series_detail_landscape
  rotate 0; sleep 3
}

# The banner needs a failing write and a screen that shows it. Deleting the
# profile is a write that happens *on the login screen*, so the same three taps
# serve both the banner test and the confirm-dialog test - with the directory
# permissions as the only variable between the two runs.
banner_break() {
  echo "[banner] make the data dir unwritable"
  sh_ "run-as $PKG chmod 000 files/dado-player"
  sh_ "run-as $PKG ls -ld files/dado-player" 2>&1 | tr -d '\r' | sed 's/^/    /'
}

banner_fix() {
  echo "[banner] restore the data dir"
  sh_ "run-as $PKG chmod 700 files/dado-player"
  sh_ "run-as $PKG ls -ld files/dado-player" 2>&1 | tr -d '\r' | sed 's/^/    /'
}

# Löschen sits at y=1262 px and Abbrechen at y=1414 px, measured off
# 96_confirm_dialog.png: the card spans 854..1545 px (264 dp), the blue button
# 1347..1482 px. Both are 840 px wide, centred.
DELETE_Y=1262
CANCEL_Y=1414

dialog_cancel() {
  echo "[cancel] tap Abbrechen at y=$CANCEL_Y"
  sh_ input tap 540 $CANCEL_Y; sleep 5
  alive
  shot A3_after_cancel
}

dialog_delete() {
  echo "[delete] tap Löschen at y=$DELETE_Y"
  sh_ input tap 540 $DELETE_Y; sleep 6
  alive
  shot A4_after_delete
  files
}

# Same taps, but the data dir is unwritable, so this is the failing-write case
# and the banner is the observable. Separate capture name on purpose: the two
# runs are the A and B of one comparison and must not overwrite each other.
dialog_delete_failing() {
  echo "[delete] tap Löschen at y=$DELETE_Y with the data dir unwritable"
  sh_ input tap 540 $DELETE_Y; sleep 6
  alive
  shot A4_banner_fail
  files
}

# Create a profile from an empty form (no saved-profile row, so the form sits at
# the top and the no-suffix coordinates apply).
# Create a profile from the login form. Two layouts exist and the shift between
# them is *not* uniform (name +302, host +289, user +317, pass +277, and the
# action bar moves up 25 px), so both sets are measured and selected, not
# offset from one another.
#
# The probe tap is at x=200, not x=540: with a saved-profile row present, y=722
# is inside the source segmented control, and a tap at the centre switches the
# form to "M3U / Playlist" - which is how an earlier run silently ended up
# typing a host into an M3U URL field.
mkprofile() {
  sh_ input tap 200 722
  if ime_wait 1 8; then
    echo "  layout: no saved profile"
    sleep 1; sh_ input text "Wohnzimmer"; sleep 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    field 540 $HOST_Y "10.0.2.2:8899" || return 1
    field 540 $USER_Y "probe" || return 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    field 540 $PASS_Y "probe" || return 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    shot E1_form_named
    sh_ input tap 540 $SUBMIT_Y; sleep 14
  else
    echo "  layout: saved profile present"
    field 540 $NAME_Y_P "Wohnzimmer" || return 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    field 540 $HOST_Y_P "10.0.2.2:8899" || return 1
    field 540 $USER_Y_P "probe" || return 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    field 540 $PASS_Y_P "probe" || return 1
    sh_ input tap 540 $HEADER_Y; ime_wait 0
    shot E1_form_named
    sh_ input tap 540 $SUBMIT_Y_P; sleep 14
  fi
  alive
  shot E2_after_login
}

# Back, wherever it is supposed to act. `keyevent 4` is the same event the
# system sends for the back gesture.
back() {  # back <capture_name>
  echo "[back] keyevent 4"
  sh_ input keyevent 4; sleep 5
  if alive; then
    shot "${1:-back}"
  else
    echo "  process gone - back finished the activity"
  fi
}

# The settings screen, for the last unverified write path: `save_settings_now()`
# has exactly one mobile call site (`settings.rs:132`), so settings.json only
# exists after a control on this page is used.
#
# Sheet rows, measured off N1_more_sheet.png: Suche 1696, Favoriten 1838,
# Einstellungen 1981. The old 1798 was for the pre-§20 sheet and now lands on
# Favoriten - which is how a settings capture turned out to be the Favoriten
# page.
settings_page() {
  nav more; sleep 3
  sh_ input tap 540 1981; sleep 8
  alive
  shot S1_settings
}

# Tap a control on the settings page at absolute coordinates, then show what the
# write produced. Coordinates come from S1_settings, not from arithmetic: the
# `add_space` + `item_spacing` double count makes every gap 8 dp wider than the
# source suggests.
settings_tap() {  # settings_tap <x_px> <y_px> <capture_name>
  sh_ input tap "$1" "$2"; sleep 5
  alive
  shot "${3:-S2_after_tap}"
  files
}

# P0 exercise: the profile-delete confirm. The saved-profile row only renders
# when a profile was stored *with a name*, and every earlier run left the name
# field empty - which is why this path has never been on screen.
#
# Row geometry: brand 48 dp ends at 116.8 dp, `section` adds 14 + label 18 + 2
# with 8 dp item spacing, so the 64 dp row spans ~174.8..238.8 dp; centre 207 dp
# = 543 px. The delete hit area is centred at `right - 24` = 387.4 dp = 1017 px.
#
# Stops on the dialog capture: the card's height depends on how the two body
# sentences wrap, so the button centres are read off `81_confirm_delete` rather
# than predicted. Guessing them is how a "cancel" tap lands on "Löschen".
p0() {
  echo "[p0] relaunch to the login form with a named profile"
  sh_ "am force-stop $PKG"; sleep 2
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 12
  alive || return 1

  echo "[p0] create a named profile so the row renders"
  sh_ input tap 540 722
  ime_wait 1 || return 1
  sleep 1; sh_ input text "Wohnzimmer"; sleep 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $HOST_Y "10.0.2.2:8899" || return 1
  field 540 $USER_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $PASS_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  shot 79_form_with_name
  sh_ input tap 540 $SUBMIT_Y; sleep 12
  alive

  echo "[p0] relaunch: the login screen must now list it"
  sh_ "am force-stop $PKG"; sleep 2
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 14
  alive
  shot 80_login_with_profile

  echo "[p0] tap the delete glyph"
  sh_ input tap 1017 543; sleep 5
  shot 81_confirm_delete
}

# Back key must cancel the dialog, not close the activity underneath it.
p0back() {
  echo "[p0back] keyevent 4 with the dialog open"
  sh_ input keyevent 4; sleep 5
  alive
  shot 82_after_back
}

p0cancel() {  # p0cancel <y_px>
  echo "[p0cancel] cancel button at y=$1"
  sh_ input tap 540 "${1:?y required}"; sleep 5
  shot 83_after_cancel
}

p0confirm() {  # p0confirm <delete_y_px>
  echo "[p0confirm] reopen the dialog, then confirm at y=$1"
  sh_ input tap 1017 543; sleep 5
  sh_ input tap 540 "${1:?y required}"; sleep 6
  alive
  shot 84_after_delete
  files
}

# ---- P1a: the delete dialog's name bound -----------------------------------
#
# 25 characters, chosen so it is over the 20-character cap and still short
# enough that the *row* on the login screen renders it whole — a row that had
# already ellipsised would make the dialog's ellipsis unreadable as evidence.
#
# `%s` is `input text`'s escape for a space: `adb shell` joins its arguments and
# the device shell splits them again, so a literal space arrives as a second
# word and `input text` drops everything after it.
LONG_NAME='Wohnzimmer%sGartengeschoss'

# The number the cap is applied to. `chars()`, not `len()`: the interesting
# failure is a name whose byte length differs from its character length, and
# only printing both makes the difference visible in the log.
namebytes() {
  echo "[name] stored profile names, chars vs bytes:"
  sh_ "run-as $PKG cat files/dado-player/profiles.json" 2>/dev/null | tr -d '\r' \
    | python -c 'import sys, json
try:
    d = json.load(sys.stdin)
except Exception as e:
    print("    unreadable:", e); raise SystemExit(0)
for p in d:
    n = p.get("name", "")
    print("    %r  chars=%d bytes=%d" % (n, len(n), len(n.encode("utf-8"))))' \
    || echo "    (no profiles.json)"
}

# Fresh data, one long-named profile, relaunch, then the dialog. The two
# captures are one argument: `90_login_long_name` shows what the row has room
# for, `91_dialog_long_name` shows where the sentence cut it. The log line from
# `namebytes` is what ties them together — it is the only place the true length
# of the name appears.
longname() {
  launch || return 1
  echo "[longname] create a 25-character profile"
  sh_ input tap 540 722
  ime_wait 1 || return 1
  sleep 1; sh_ input text "$LONG_NAME"; sleep 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $HOST_Y "10.0.2.2:8899" || return 1
  field 540 $USER_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  field 540 $PASS_Y "probe" || return 1
  sh_ input tap 540 $HEADER_Y; ime_wait 0
  sh_ input tap 540 $SUBMIT_Y; sleep 14
  alive || return 1
  namebytes

  echo "[longname] relaunch: the row must render it"
  sh_ "am force-stop $PKG"; sleep 2
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 14
  alive || return 1
  shot 90_login_long_name

  echo "[longname] delete glyph -> the dialog"
  sh_ input tap 1017 543; sleep 6
  alive
  shot 91_dialog_long_name
}

# Guarantee the series *list*. A detail page keeps `selected_series`, and
# tapping the nav tab does not clear it — only `App::mobile_back` does — so a
# step that follows `compact` would otherwise "open card 2" and silently capture
# series 2000's detail a second time.
#
# The order matters: nav to Series first, so the back key is handled by
# `mobile_back` (detail -> list, or list -> Live) instead of falling through to
# `finish()`. A back press on the Live view exits the activity.
series_list() {
  nav series
  sh_ input keyevent 4; sleep 3
  nav series; sleep 2
  alive
}

# One detail page at a chosen card column, portrait then landscape. Card centres
# are 75 / 205 / 335 dp — 3 columns of 118 dp on a 411 dp screen, from
# `poster_grid`'s own arithmetic.
#
# Both orientations in one call on purpose: the compact branch is only visible
# as a *difference* between the two frames, and two runs minutes apart could
# differ for reasons that have nothing to do with the layout.
detail_at() {  # detail_at <x_dp> <capture_prefix>
  alive || return 1
  series_list || return 1
  local pt; pt=$(panel_top_dp)
  local r0; r0=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5}')
  echo "[detail_at] card at $1,${r0} dp"
  tap_dp "$1" "$r0"; sleep 10
  alive || return 1
  shot "${2}_portrait"
  rotate 1; sleep 4
  shot "${2}_landscape"
  rotate 0; sleep 4
}


# ---- P1b: the compact hero -------------------------------------------------
#
# One session, one series, two orientations. Card 1 is series 2000 ("Tatort
# Nord", 6 episodes across 2 seasons), so the meta line has something to say in
# portrait: "2 Staffeln · 6 Folgen".
#
# The discriminating observable is the *meta line*, not the row count. The row
# count is the prediction (exactly one complete episode row in 253 dp of page);
# the meta line is what separates "the compact branch ran" from "the page got
# shorter and something scrolled out of view". If the meta line is still there
# in landscape, the branch did not run, whatever the row count says.
#
# Grid geometry, from `poster_grid`: cols = (379+12)/(100+12) = 3, cw = 118, so
# card centres are 75 / 205 / 335 dp. The old CARD2_X=781 px is 297 dp — that is
# card *3*, and it is why the fixture comment above it no longer matched.
compact() {
  alive || return 1
  series_list || return 1
  local pt; pt=$(panel_top_dp)
  local r0; r0=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5}')
  shot C0_series_list
  echo "[compact] open card 1 (series 2000) at 75,${r0} dp"
  tap_dp 75 "$r0"; sleep 10
  alive || return 1
  shot C1_detail_portrait
  echo "[compact] rotate to landscape - the meta line must disappear"
  rotate 1; sleep 4
  shot C2_detail_landscape
  echo "[compact] back to portrait"
  rotate 0; sleep 4
}

# One season, one episode: the meta line must not be drawn at all. Series 2001
# ("Die Ermittler", card 2 at 205 dp) is seeded with a single episode in
# `App::new` for exactly this check, so the observable is an *absence* — which
# only means something next to a neighbour that still has the line.
# `C1_detail_portrait` from the `compact` run is that neighbour: same page, same
# plot length, 6 episodes instead of 1.
meta1() {
  alive || return 1
  series_list || return 1
  local pt; pt=$(panel_top_dp)
  local r0; r0=$(awk -v p="$pt" 'BEGIN{printf "%.1f", p + 146.5}')
  shot M0_series_list
  echo "[meta1] open card 2 (series 2001, one episode) at 205,${r0} dp"
  tap_dp 205 "$r0"; sleep 10
  alive || return 1
  shot M1_one_episode
}

# Positive control for the capture pipeline, to be run in the same session as
# any gesture test. The chevron's hit area is 48 dp centred at
# `rect.left() + pad + 24` with the bar's centre line at `top_inset + appbar_h/2`
# (`components::app_bar`) — 40 dp and 76.8 dp in portrait, so 105 x 201 px.
#
# Its effect is not in question. If the frame changes after the chevron and not
# after a gesture, the pipeline is live and the gesture is dead; three
# byte-identical captures on their own prove neither, because a stale buffer
# produces the same file whether or not input works.
chevron() {  # chevron <capture_name>
  local name="${1:-P0_after_chevron}"
  local before after
  before=$("$ADB" exec-out screencap -p 2>/dev/null | md5sum | cut -d' ' -f1)
  echo "[chevron] tap 105,201 (bar-back hit area)"
  sh_ input tap 105 201; sleep 5
  shot "$name"
  after=$(md5sum "$OUT/$name.png" | cut -d' ' -f1)
  echo "[chevron] before=$before"
  echo "[chevron] after =$after"
  if [ "$before" = "$after" ]; then
    echo "  FRAME UNCHANGED - the tap had no effect either; this capture proves nothing"
  fi
  return 0
}


# Why the profile row never appears. `store::app_dir()` falls back to "." when
# `directories` cannot resolve a config dir, and on Android HOME is unset, so
# the app may be writing to a path it cannot create. This run logs in with a
# named profile and then looks for the file on the device.
store() {
  sh_ "am force-stop $PKG"; sleep 2
  "$ADB" shell am start -W -n "$ACT" >/dev/null 2>&1
  sleep 12
  alive || return 1
  sh_ input tap 540 722; sleep 4
  sh_ input text "Wohnzimmer"; sleep 3
  sh_ input tap 540 $HOST_Y; sleep 3
  sh_ input text "10.0.2.2:8899"; sleep 2
  sh_ input tap 540 $USER_Y; sleep 3
  sh_ input text "probe"; sleep 2
  sh_ input tap 540 $HEADER_Y; sleep 3
  sh_ input tap 540 $PASS_Y; sleep 3
  sh_ input text "probe"; sleep 2
  sh_ input tap 540 $HEADER_Y; sleep 3
  sh_ input tap 540 $SUBMIT_Y; sleep 14
  alive
  echo "[store] app data dir:"
  sh_ "run-as $PKG ls -la /data/data/$PKG" 2>&1 | head -12
  echo "[store] /files:"
  sh_ "run-as $PKG ls -la /data/data/$PKG/files" 2>&1 | head -12
  echo "[store] profiles.json under the data dir:"
  sh_ "run-as $PKG find /data/data/$PKG -name 'profiles.json'" 2>&1 | head -5
  echo "[store] profiles.json at the filesystem root:"
  sh_ "run-as $PKG ls -la /profiles.json" 2>&1 | head -3
  echo "[store] app process cwd:"
  sh_ "run-as $PKG readlink /proc/\$(pidof $PKG)/cwd" 2>&1 | head -3
}

# P3: is the last list row flush against the navigation bar or clear of it?
# Swipes hard to the end (4 x 1100 px, more than the content height) so the
# scroll offset is at its maximum, which is the only state that answers the
# question. The row has no background, so the measurement uses the 44 dp avatar
# tile: row bottom = avatar bottom + 10 dp.
p3() {
  alive || return 1
  nav live
  shot 90_live_top
  local i
  for i in 1 2 3 4; do
    sh_ input swipe 540 1900 540 600 250; sleep 3
  done
  sleep 3
  shot 91_live_max_scroll
}

case "${1:-}" in
  all)
    start_stub
    start_emu || exit 1
    "$ADB" reverse tcp:8899 tcp:8899 >/dev/null 2>&1
    echo "[install] $APK"
    "$ADB" install -r "$APK" 2>&1 | tail -2
    launch
    shot 00_login
    login
    ;;
  login)     login ;;
  walk)      walk ;;
  full)      full ;;
  walk2)     walk2 ;;
  walk3)     walk3 ;;
  p1)        p1 ;;
  p0)        p0 ;;
  p0back)    p0back ;;
  p0cancel)  p0cancel "${2:-}" ;;
  p0confirm) p0confirm "${2:-}" ;;
  store)     store ;;
  p3)        p3 ;;
  files)     files ;;
  grow)      grow ;;
  signin)    signin ;;
  formp)     formp ;;
  dialog)    dialog ;;
  dialogback) dialog_back ;;
  dialogcancel) dialog_cancel ;;
  dialogdelete) dialog_delete ;;
  dialogfail) dialog_delete_failing ;;
  mkprofile) mkprofile ;;
  back)      back "${2:-back}" ;;
  settings)  settings_page ;;
  settap)    settings_tap "${2:?x}" "${3:?y}" "${4:-S2_after_tap}" ;;
  bannerbreak) banner_break ;;
  bannerfix)  banner_fix ;;
  meta)      meta ;;
  compact)   compact ;;
  detail)    detail_at "${2:?x_dp required}" "${3:?capture prefix required}" ;;
  meta1)     meta1 ;;
  chevron)   chevron "${2:-P0_after_chevron}" ;;
  longname)  longname ;;
  namebytes) namebytes ;;
  persist)   persist ;;
  persist2)  persist2 ;;
  repro)     repro ;;
  repro2)    repro2 ;;
  nav)       nav "${2:?live|movies|series|fav|more}" ;;
  launch)    launch "${2:-}" ;;
  shot)      shot "${2:?name required}" ;;
  tap)       sh_ input tap "${2:?x}" "${3:?y}"; echo "  tap $2 $3" ;;
  alive)     alive ;;
  landscape) rotate 1; shot "${2:-landscape}" ;;
  portrait)  rotate 0; shot "${2:-portrait}" ;;
  stub)      start_stub ;;
  emu)       start_emu ;;
  *) echo "usage: $0 {all [APK]|emu|stub|launch [--keep]|login|shot NAME|alive|landscape [NAME]|portrait [NAME]|files|persist|persist2|p0|p0back|p0cancel Y|p0confirm Y|p1|p3|store|walk|full|walk2|walk3|repro|repro2|nav X|meta|compact|meta1|chevron [NAME]|longname|namebytes}"; exit 2 ;;
esac
