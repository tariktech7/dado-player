# dado-player - Android Build (CI only)

Rust + egui/eframe 0.27 IPTV-App, portiert auf Android (arm64). Die APK wird
**ausschließlich in GitHub Actions CI** gebaut - kein lokales NDK nötig.

Die native Rust-Library wird mit `cargo ndk` als `libdadoplayer.so` (arm64-v8a)
gebaut und zusammen mit einer vorgebauten `libmpv.so` von Gradle zu einer APK
paketiert. Release-APKs werden in CI mit einem Keystore aus GitHub Secrets
signiert.

---

## 1. Voraussetzungen auf Rust-Seite (vom Cargo.toml / Rust-Code)

Laut Architekt (verifiziert für eframe 0.27.2 + glow):

- `eframe` muss mit den Features `android-game-activity` + `glow` aktiviert sein.
- Rust-Entrypoint vorhanden:
  ```rust
  #[cfg(target_os = "android")]
  #[no_mangle]
  fn android_main(app: android_activity::AndroidApp) {
      // ... eframe::run_native / run android glue
  }
  ```
- Cargo-Paketname `dado-player`, `[[bin]]` heißt `Dado-Player`.
- `android-activity` muss für den Android-Target verfügbar sein (via egui-winit's
  `android-game-activity`-Feature oder explizite Dependency), da `android_main`
  den Typ `android_activity::AndroidApp` referenziert.

Siehe Abschnitt **"Rust .so naming"** - der `[[bin]]`-Name muss mit
`android.app.lib_name` zusammenpassen.

---

## 2. Repo zu GitHub pushen & Workflow triggern

```sh
git init            # falls noch kein Repo
git add .
git commit -m "Add Android CI scaffold for dado-player"
git remote add origin <your-github-repo-url>
git push -u origin main
```

Der Workflow `.github/workflows/android-apk.yml` läuft automatisch bei:

- `push` auf `main` oder Tag `v*` -> baut **Debug + Release (signiert)**
- `pull_request` auf `main` -> baut nur **Debug** (keine Secrets verfügbar)
- `workflow_dispatch` -> manueller Lauf über die GitHub-UI (Actions -> Run workflow)

APK-Artifacts findest du unter **Actions -> <Lauf> -> Artifacts**
(`dado-player-debug-apk`, `dado-player-release-apk`).

---

## 3. Secrets für den Release-Build

Unter **Repo -> Settings -> Secrets and variables -> Actions** anlegen:

| Secret                | Inhalt                                                          |
|-----------------------|-----------------------------------------------------------------|
| `SIGNING_KEY`         | base64-kodierter Java-Keystore (`.jks` / `.keystore`)           |
| `KEY_ALIAS`           | Alias des Keys im Keystore                                      |
| `KEY_PASSWORD`        | Passwort des Keys                                               |
| `KEY_STORE_PASSWORD`  | Passwort des Keystores                                          |

Keystore erzeugen (lokal, kein NDK nötig):

```sh
keytool -genkeypair -v \
  -keystore dadoplayer-release.keystore \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias dadoplayer

# base64 für das Secret SIGNING_KEY:
base64 -w0 dadoplayer-release.keystore | pbcopy   # macOS
# bzw.  base64 -w0 dadoplayer-release.keystore | clip  (Windows)
```

Der Keystore wird **nicht** ins Repo committet (siehe `.gitignore`).

---

## 4. Woher kommt `libmpv.so`? (automatisiert in CI)

`libmpv.so` (arm64-v8a) **samt ihrer Laufzeit-Abhängigkeiten** (libavcodec,
libavformat, libswscale, libc++_shared, ...) wird im CI automatisch aus dem
offiziellen **mpv-android** Release-APK (arm64-v8a) extrahiert und nach

```
android/app/src/main/jniLibs/arm64-v8a/
```

kopiert. Der pinbare Tag steht in `.github/workflows/android-apk.yml`
(`env: MPV_ANDROID_TAG`). Das verwendete Laufzeitpaket enthält FFmpeg als Shared Objects;
die oben genannten Shared Objects müssen zur Ladezeit vorhanden sein — daher
wird der komplette `lib/arm64-v8a/`-Ordner des APK extrahiert, nicht nur
libmpv.so.

Auch der lokale Build benötigt diese Bibliotheken unter jniLibs; `bash android/build.sh` paketiert sie mit dem Rust-Host.

---

## 5. Verbleibende Release-Readiness-Aufgaben

1. **GameActivity.java Glue** — ✅ erledigt:
   `android/app/src/main/java/android/app/GameActivity.java` existiert und
   erweitert die AndroidX `GameActivity` aus `androidx.games:games-activity:2.0.2`.
   Siehe `android/app/src/main/java/android/app/README.md`.
2. **Gradle Wrapper** — ✅ erledigt: echter Gradle-8.9-Wrapper ist committet
   (`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar`). Kein lokales
   `gradle wrapper` mehr nötig.
3. **Rust .so naming (geklärt)** - cross-platform-architect hat **Option B**
   gewählt: Cargo.toml erhält zusätzlich
   `[lib] name = "dadoplayer", crate-type = ["cdylib", "rlib"]`
   (der bestehende `[[bin]] Dado-Player` bleibt für Windows unverändert). Das
   erzeugt exakt `libdadoplayer.so` und passt zu `android.app.lib_name =
   "dadoplayer"`. Der temporäre Rename-Schritt im Workflow wurde daher entfernt.
   `android_main` liegt in `src/lib.rs` unter `#[cfg(target_os = "android")]`;
   Windows-bin und Android-cdylib teilen sich den Source.
4. **Keystore + Secrets** - siehe Abschnitt 3.
5. **Versionen verifizieren** - AGP 8.7.0 + Gradle 8.9, NDK 27.0.12077973,
   Action-Tags (`android-actions/setup-android@v3`, `r0adkll/sign-android-release@v1`,
   `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`) gegen aktuelle
   Upstream-Docs prüfen, bevor produktiv gebaut wird.
6. **Geräteabnahme** - echte Android-Geräte für Login, Tastatur, EPG, Kategorienwechsel,
   Videooberfläche, Drehung, Hintergrund-/Fortsetzenverhalten und Signed-Release testen.

---

## 6. Hinweise zum Scaffold

- `applicationId`: `com.tarik.dadoplayer`, `minSdk 24`, `targetSdk 34`.
- `local.properties` wird in CI aus `$ANDROID_HOME` erzeugt (siehe Workflow).
- Signing erfolgt **nach** dem Build via `r0adkll/sign-android-release`, daher
  ist in `app/build.gradle` **kein** `signingConfig` konfiguriert (Keys bleiben
  in GitHub Secrets).
- Launcher-Icon ist ein minimaler adaptiver Icon-Platzhalter
  (`mipmap-anydpi-v26`, API 26+). Für volle Abdeckung (API 24/25) später
  density-PNGs ergänzen.

## Playback defaults since 0.6.1

Automatic mode uses the bundled libmpv/FFmpeg player and Dado controls, without
requiring MX Player or VLC to be installed. External apps, hardware-decoder
experiments, live format and custom network options are available in expert
mode. Older Easy-mode settings migrate once to the integrated player; profiles,
favorites and history remain intact. Expert selections are preserved. Actual
MP4/MKV decoding is part of the compatibility smoke checks; individual provider
codec combinations and Android TV hardware still require device validation.
The local build entry point is `bash android/build.sh` (SDK/NDK/JDK required),
which signs updates with the existing project key.

### 0.6.3 refresh-rate hint

The native host listens for display changes and requests the highest supported
refresh rate at the current physical resolution via preferredRefreshRate. It does
not force a display mode or resolution. Android power/thermal policy still applies.
Display callbacks are registered only while resumed. Shared texture uploads are
limited per UI frame to avoid loading many covers in one burst.

### 0.6.4 catalogue scroll work

The shared catalogue draws only viewport categories plus remote-navigation neighbours.
Image decoding, downsampling and ColorImage conversion run on a background worker;
the UI receives upload-ready pixels. Poster/detail textures keep their proportions
with a 640-pixel maximum edge. Full caches evict older textures instead of permanently
blocking new covers. The display-rate hint remains unchanged. Current build: 18.

## 0.7.0 · Dado Connect

Build 19: M3U-/Xtream-Profilimport in Touch- und TV-Modus. Private Geräteadresse, einmaliger Kopplungscode, danach Aktualisieren. Keine Hintergrundabrufe oder automatische Anmeldung. Siehe ../connect/README.md.

## 0.7.1

Gemeinsame warme Tageslichtdarstellung mit Dunkel/Hell/System, strengere Film-/Serien-Zuordnung und kompatible Einstellungen. Android-Build 20, iOS-Build 15. MX als externe installierte App nur auf Android/Android TV; kein plattformübergreifendes MX-SDK vorhanden.

## 0.7.2

Lavendel-Design und neue Logos. Geräteadresse wird einmalig registriert; Profile nur mit „Profile aktualisieren“ abrufen. Keine Portal-/Kontodetails oder Kopplungscodes im App-UI. MAC-Zuordnung und gespeicherte Vorlagen werden im Portal verwaltet. iOS Build 16, Android Build 21.
