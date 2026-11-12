@echo off
REM Dado Player - UI-Vorschau im Demo-Modus (kein Provider-Login noetig).
REM Zeigt dieselbe egui-Oberflaeche, die auch in der Android-APK steckt.
set IPTV_DEMO=1
set IPTV_DEMO_VIEW=live
start "" "%~dp0dist\Dado-Player.exe"
