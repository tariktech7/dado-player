# Eigene mobile Film- und Serienseiten

Android 0.2.9 / Build 13 und iOS 0.2.6 / Build 8.

## Verhalten

- Touch-Modus auf Telefonen/Tablets verwendet ein Raster über die gesamte Inhaltsbreite statt verkleinerter TV-Spalten. Oben Kategorien-Dropdown, rechts die Lupe für die Suche innerhalb des jeweiligen Tabs.
- Ein Poster öffnet eine eigene Detailseite. Die untere Navigation wird ausgeblendet; Zurück oben links, Android-Zurück und Wischen vom linken Rand führen zum bisherigen Raster zurück.
- Beschreibung, Wiedergabe/Fortsetzen, horizontal scrollbare Staffeln und Folgen liegen in einer einzigen vertikalen Scrollfläche. Keine kleine verschachtelte Folgenliste. Die Folgen haben mindestens 56 Punkte hohe Touchflächen und kurze Namen.
- Normales Scrollen und Staffel-Wischen aus der Bildschirmmitte lösen kein Zurück aus. Ohne bisherige Wiedergabe öffnet die Detailseite oben beim Titel, statt sofort zur ersten Episode zu springen. Gespeicherte Staffel-/Fortsetzenregeln bleiben erhalten.
- Icon und Beschriftung der unteren Navigation werden gemeinsam vertikal und horizontal zentriert. Virtualisierte Poster sind auf die Inhaltsfläche begrenzt und dürfen die Navigation nicht übermalen.
- Favoriten stehen bei Film/Serie bereits in der gemeinsamen Desktop-/TV-Ansicht und im Tizen-Client zuerst; der mobile Dropdown übernimmt diese Reihenfolge. Mobile Live-TV-Kategorien: Alle, Favoriten, anschließend unveränderte Anbieter-Kategorien.

Windows 0.5.5 und Samsung Tizen 0.1.6 bleiben als vorhandene Pakete erhalten. Der TV-Modus nutzt weiterhin seine eigene Spalten-/Fokusdarstellung. Die Änderung am mobilen Layout wird nicht auf TV-Spalten übertragen.

## Prüfung

- 47 Rust-Tests bestanden, einschließlich eines neuen Layout-/Navigationsfalls für 320×568, 393×852, 852×393 und 768×1024 logische Punkte: eigener Katalog, Detailseite ohne Bottom-Tabs, Zurück für Filme und Serien, Rand-Wisch zurück, vertikale Randbewegung ohne Zurück und Clipping der Poster oberhalb der Navigation.
- cargo fmt, Clippy mit -D warnings und iOS-Host-Typcheck bestanden.
- Eigener Android-Emulator mit synthetischem Xtream-Anbieter: Raster, Serien-Detailseite, lange Folgenliste durch Wischen bis Folge 13, Rand-Wisch zurück und tabspezifische Suche beobachtet. Zwölf Staffeln und zwanzig Folgen je Staffel im Testanbieter; keine echten Zugangsdaten verwendet.
- Screenshots liegen außerhalb des Quellprojekts unter `C:/Users/Tarik/Documents/Codex/2026-11-12/fi/work/mobile-catalogue-qa/`.

Ein erfolgreicher lokaler iOS-Geräte-Build ersetzt keinen Test des Touch-Layouts und der Wiedergabe auf einem echten iPhone. Der Android-Emulator ist ebenfalls kein Nachweis für jedes physische Android-Gerät.

## Abschließende Pakete

Android: endgültige APK mit VersionCode 13 gebaut, stabile Projektsignatur und 4-KiB-Alignment geprüft. Auf dem eigenen Emulator als Update installiert. Film-Detailseite, Serienraster mit Favoriten zuerst im Dropdown, Live-TV mit Alle/Favoriten und die ausgerichtete Navigation am endgültigen Paket beobachtet.

iOS: Rust-Gerätearchiv und Swift-Host unter WSL mit iPhoneOS 27.3 neu gebaut. Dabei eine fehlende Build-Abhängigkeit korrigiert: SwiftPM verfolgt die über -l eingebundene Rust-Bibliothek nicht selbst. Das Skript bereinigt jetzt vor dem Swift-Link dessen Build-Ausgaben, damit reine Rust-Änderungen keine alte ausführbare Datei übernehmen. Der abschließende Lauf hat Swift tatsächlich neu übersetzt und gelinkt. Mach-O arm64, Version 0.2.6/8, Ressourcen und ZIP-Integrität geprüft; der neue mobile UI-Text ist im Executable enthalten, dessen SHA sich vom archivierten Build 7 unterscheidet. Keine Apple-Gerätesignierung oder iPhone-Laufzeitprüfung behauptet.

| Paket | Bytes | SHA-256 |
| --- | ---: | --- |
| Dado-Player-arm64-debug.apk | 45036071 | 8755e6cab320afa81e756ba8c776e9edc0532e63698b741f95bb97dbd31e9932 |
| Dado-Player-unsigned.ipa | 5732187 | 7b4e1e53cc21f636eededdc048134014d3343433dd13fef29f1cd0f5a90d2ca0 |

Die vorherige Android-Version 0.2.8 und iOS 0.2.5/Build 7 wurden unverändert in Releases/Archiv erhalten. Releases/README.md und AKTUELL.json zeigen die neuen Pakete. Xcode-XIP und lokale SDKs bleiben außerhalb von Git und Releases.
