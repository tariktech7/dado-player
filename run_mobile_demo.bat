@echo off
rem Startet die mobile/touch-Oberflaeche des Dado-Players am PC,
rem gefuellt mit der Demo-Bibliothek (kein Login noetig).
rem Nach dem Oeffnen das Fenster einfach schmal ziehen (~400 px breit),
rem um das echte Phone-Layout zu sehen.
set IPTV_MOBILE=1
set IPTV_DEMO=1
set IPTV_DEMO_VIEW=home
start "" "%~dp0dist\Dado-Player.exe"
