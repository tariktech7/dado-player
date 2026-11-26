# Dado Player bauen und veröffentlichen

`release.json` ist die gemeinsame Versionsquelle für Windows, Android, iOS und Samsung. Nach Änderungen synchronisiert `python scripts/release_version.py` die Plattformdateien. `python scripts/release_version.py --check` prüft, ob sie übereinstimmen. Befehle in dieser Übersicht laufen aus dem Projektordner.

| Plattform | Build oder Paketierung | Details |
| --- | --- | --- |
| Windows | `./windows/package.ps1` | Erstellt die Portable `Releases/Dado-Player.exe`. Für einen isolierten Build ohne Portable `./windows/build.ps1` verwenden. |
| Android | `bash android/build.sh` | Benötigt JDK, Android SDK, NDK und cargo-ndk. Das Skript prüft die APK-Signatur. |
| iOS | `bash ios/build.sh --ipa` | Erzeugt die IPA zur nachträglichen Gerätesignierung. [Build und Tests für iOS](../ios/README.md) |
| Samsung Tizen | `./SmartTV/client/build.ps1` | Das WGT braucht für die Installation eine passende TV-Signatur. [Samsung-Profile und Entwicklungsrichtlinien](../SmartTV/README.md) |

Windows, Android und iOS teilen den Rust-Kern in `src/`. Die Samsung-Oberfläche liegt in `SmartTV/client/`. Gemeinsame UI-Einstellungen liegen in `shared/`. Funktionsumfang und Plattformunterschiede stehen im [Plattformabgleich](PLATFORM-PARITY.md).

## Veröffentlichung

1. In `release.json` die Versions- und Buildnummern der betroffenen Ziele aktualisieren, synchronisieren und die Pakete bauen.
2. `python packaging/organize_releases.py` ausführen. Die kanonischen Namen, Größen und SHA-256-Werte stehen lokal in `Releases/AKTUELL.json`.
3. Die Pakete mit den dort angegebenen Asset-Namen veröffentlichen. Betas erhalten eigene nummerierte Vorab-Releases. Stabile Versionen wie `v0.8.2` erscheinen als reguläre Releases.
4. Samsung-Updates für bereits installierte Apps separat für beide Kompatibilitätsprofile signieren und über `npm run publish:update -- <Verzeichnis>` aus `SmartTV/client/` veröffentlichen. Das Skript prüft Signaturen und Buildnummern und aktualisiert den Branch `tizen-updates`. Das WGT im GitHub-Release dient der Geräteinstallation.

Unter Windows kann die Portable ein geprüftes Paket am ursprünglichen Ort ersetzen und die vorige Datei als `.previous` sichern. Android übergibt das APK an den Systeminstaller. Auf iOS erfolgt die Installation mit dem üblichen Signierverfahren des Geräts. Windows kann optional per `./windows/package.ps1 -SigningThumbprint <Thumbprint>` mit einem lokal verfügbaren Codesignaturzertifikat signiert werden.

Zugangsdaten, private Schlüssel und gerätespezifische Signierprofile lokal halten. [Lizenzverwaltung](../licensing/README.md) · [Entwicklungsrichtlinien](../AGENTS.md)