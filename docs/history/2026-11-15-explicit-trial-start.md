# Expliziter Teststart – Windows 0.5.8 / Samsung 0.1.8

Nur „7 Tage kostenlos testen“ legt eine Testversion an. Der bisherige gemeinsame Abruf konnte beim Klicken auf „Freischaltung abrufen“ ebenfalls einen Trial erzeugen. Die Korrektur trennt die Aktionen im Backend und in beiden Clients:

- `/v1/trial/start`: idempotenter ausdrücklicher Start. Vorhandener Beginn und Ablauf werden nicht erneuert.
- `/v1/license`: vorhandenen Status lesen; kein Anlegen unbekannter Geräte.
- `/v1/license/activation`: ausschließlich genehmigte Dauerlizenzen. Ohne Freigabe `NOT_APPROVED`, ohne Trial-Zertifikat oder neuen Geräteeintrag.
- Beide Clients verweigern beim Aktivierungsabruf auch eine korrekt signierte Trial-Antwort. Gültige bestehende Trials bleiben bei einem abgelehnten Freischaltabruf nutzbar.

Unter dem App-Namen zeigt die Navigation „Testversion · noch N Tage“. Eingeklappt bleibt eine kurze Anzeige erhalten, auch im Windows-TV-Modus. Resttage werden lokal aktualisiert. Eine Dauerfreigabe entfernt den Trial-Hinweis und verursacht weiterhin keine automatischen Lizenz-Serverkontakte. Bestehende Geräte-IDs, Freigaben, Profile und Testfristen bleiben unverändert.

## Prüfergebnisse

- Backend: 6 Tests mit echter SQLite-Datenbank und separaten Testschlüsseln bestanden. Neue Fälle prüfen beide Plattformen, Aktivierung vor Teststart, fehlende DB-Neuanlage, unveränderten Beginn bei Wiederholung und Dauerlizenz nach manueller Genehmigung.
- TV: 16 Unit-Tests und 21 Browser-UI-Tests bestanden. Neuer kompletter Ablauf: Aktivierungsfehler ohne Trial → ausdrücklicher Teststart → Anzeige 7 Tage → lokaler Zeitsprung und 6 Tage → kompakte Navigation → Dauerfreigabe entfernt Hinweis → Neustart ohne Serverabruf.
- Windows: 51 Rust-Tests, Clippy und Formatprüfung bestanden. Neue Regression prüft Ablehnen eines gültig signierten Trials beim Aktivierungsabruf. Echte portable EXE gestartet und „Testversion · noch 7 Tage“ direkt unter Dado-Player beobachtet.
- Vier bestehende Packaging-Tests bestanden. `Releases/Dado-Player.exe` ersetzt die vorherige Datei; Vorgänger archiviert. Keine versionierte aktuelle Windows-EXE.
- Beide Cloudflare-Worker aktualisiert. Produktiver API-Test mit rein synthetischen Geräten bestätigt: unbekannter Status/Aktivierung erzeugen keinen Trial; expliziter Start stellt einen Trial aus; Aktivierung lehnt diesen ab; ein bereits genehmigtes synthetisches Gerät erhält eine signierte Dauerlizenz. Kein Kundengerät durch diesen Test genehmigt.
- Android NDK/Gradle/Signatur/Alignment erneut erfolgreich geprüft; anschließend ursprüngliche stabile APK wiederhergestellt (SHA-256 `8755e6cab320afa81e756ba8c776e9edc0532e63698b741f95bb97dbd31e9932`). iOS-Rust-Gerätebibliothek mit Apple-SDK und ABI-Prüfung gebaut. Mobilgeräte erhalten weiterhin keine Lizenzsperre; keine neue APK/IPA veröffentlicht.
- Samsung-Pakete gebaut und Paketstruktur/Signaturdateien geprüft. Diese Änderung wurde nicht auf dem echten Fernseher installiert oder abgenommen; Browser-Tests ersetzen das nicht.

## Pakete

| Datei | Bytes | SHA-256 |
| --- | ---: | --- |
| Dado-Player.exe | 55040589 | `d7f7b77020fe47f92e12f1c388577d06d8699e1262ec84960e4d182b7224eef3` |
| Dado-Player-Tizen-0.1.8.wgt | 917204 | `400ded0feb9e810f2fab6b26b4f50d5ccca80fe9fce23e5db89fc4c0a06e0a91` |
| Dado-Player-Tizen-0.1.8-unsigned.wgt | 930175 | `a34e5b259e26a05528b9809a121bf05ec5be5503df57ece6e4f9154f7d7a148f` |

[Getrenntes GitHub-Prerelease](https://github.com/tariktech7/dado-player/releases/tag/dado-trial-2026-11-15). Vorheriges stabiles Release bleibt unverändert. Für den ersten Teststart müssen ältere Lizenz-Betas aktualisiert werden; ihr früherer allgemeiner Abruf darf seit der Backend-Korrektur keinen Trial mehr anlegen. Bestehende Lizenzen bleiben gültig.
