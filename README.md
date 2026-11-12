# Dado Player

Der Rust-Kern unter `src/` enthält App-Zustand, IPTV-Client, Speicherung, Datenmodell und egui-Oberfläche. Alle drei Apps verwenden dieselbe Crate und dieselbe Cargo.lock.

| Ziel | Einstieg und Wiedergabe | Build aus dem Projektordner | Ergebnis |
| --- | --- | --- | --- |
| Windows | src/main.rs → src/platform/windows, libmpv | `./windows/build.ps1` in PowerShell | target/release/Dado-Player.exe |
| Android | android/ (Gradle), src/platform/android, JNI und libmpv | `bash android/build.sh` unter Windows mit Git Bash | Dado-Player-arm64-debug.apk |
| iOS | ios/Sources (UIKit, AVPlayer), src/platform/ios (Rust und C-ABI) | `bash ios/build.sh --ipa` mit iOS-SDK | Dado-Player-unsigned.ipa |

Windows bündelt seine Laufzeit-DLLs in `dist/`. Das separate portable Paket wird mit `packaging/build_portable.py` aus dem Inhalt von dist gebaut. Der Cargo-Build aktualisiert dist nicht. Android benötigt JDK, Android SDK, NDK, cargo-ndk und für den lokalen signierten Build den vorhandenen Projektschlüssel. Details zum Swift-Host stehen in [ios/README.md](ios/README.md).

[AGENTS.md](AGENTS.md) beschreibt Zuständigkeiten und Prüfungen. SmartTV/ enthält den separaten Portierungsplan. references/ enthält Vergleichsmaterial. probe/ und .workbuddy-ai/ enthalten Prüfwerkzeuge und historische Aufnahmen. Frühere Baupläne und Prüfberichte stehen unter docs/history/.
