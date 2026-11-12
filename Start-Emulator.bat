@echo off
chcp 65001 >nul
setlocal
title Dado-Player - Android Emulator
cd /d "%~dp0"

set "SDK=C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot"
set "JAVA_HOME=C:\Users\Tarik\AppData\Local\Temp\android-build\jdk\jdk-17.0.21+9"
set "ADB=%SDK%\platform-tools\adb.exe"
set "EMU=%SDK%\emulator\emulator.exe"
set "APK=%~dp0Dado-Player-arm64-debug.apk"

if not exist "%APK%" (
  echo [!] APK nicht gefunden: %APK%
  pause & exit /b 1
)
if not exist "%EMU%" (
  echo [!] Emulator nicht installiert. Einmalig ausfuehren:
  echo     "%SDK%\cmdline-tools\latest\bin\sdkmanager.bat" --install "emulator" "system-images;android-34;google_apis;x86_64"
  pause & exit /b 1
)

echo ============================================================
echo   Dado-Player im Android-Emulator (Pixel 6, Android 14)
echo ============================================================
echo.

echo [1/3] Emulator starten ...
start "Dado-Emulator" "%EMU%" -avd dado_test -gpu swiftshader_indirect -no-audio -no-boot-anim -memory 3072

echo [2/3] Warten bis Android hochgefahren ist ...
"%ADB%" wait-for-device
:bootwait
for /f "delims=" %%b in ('"%ADB%" shell getprop sys.boot_completed 2^>nul') do set "BOOT=%%b"
if not "%BOOT%"=="1" (
  timeout /t 2 /nobreak >nul
  goto bootwait
)
echo       Android ist bereit.

echo [3/3] APK installieren und starten ...
"%ADB%" install -r "%APK%"
"%ADB%" shell am start -n com.tarik.dadoplayer/.GameActivity

echo.
echo ============================================================
echo   Fertig. Das Emulator-Fenster ist jetzt offen.
echo.
echo   Bedienung:
echo     - Mit der Maus klicken/tasten wie auf einem Touchscreen
echo     - Strg+Links/Rechts dreht das Geraet ins Querformat
echo     - App neu starten:  "%ADB%" shell am start -n com.tarik.dadoplayer/.GameActivity
echo     - Log mitlesen:     "%ADB%" logcat -s DadoPlayer RustStdout
echo     - Emulator beenden: "%ADB%" emu kill
echo ============================================================
echo.
pause
