# Dado Player — iOS

Rust + egui/eframe 0.27.2, portiert auf iOS über einen dünnen Swift/UIKit-Host und
[xtool](https://github.com/xtool-org/xtool). Windows ist ausgeliefert, Android baut und ist
signiert. iOS ist **Quelltext, kein Build**.

Diese Datei beschreibt, was existiert, was auf dieser Maschine nachweislich geprüft ist, und
was noch niemand ausgeführt hat. Sie beschreibt keinen Erfolg.

---

## 1. Status

| Baustein | Datei(en) | Zustand |
|---|---|---|
| Rust-Plattformmodul | `src/platform/ios/{mod,entry,sys,bridge,video_surface}.rs` | geschrieben; auf Windows per `--features ios-check` type-checked; **nie für iOS kompiliert** |
| Player-Shim (AVPlayer) | `src/mpv_ios.rs` (als `crate::mpv` eingebunden) | geschrieben; type-checked; **nie gelinkt** |
| C-ABI-Header | `ios/Sources/DadoHost/include/DadoBridge.h` | geschrieben; 3× `dado_app_*`, 16× `dado_host_*` |
| Swift-Host | `ios/Sources/DadoPlayer/*.swift` | geschrieben; **nie von einem Swift-Compiler gelesen** |
| SwiftPM-Manifest | `ios/Package.swift` | geschrieben; nie von SwiftPM aufgelöst |
| xtool-Konfiguration | `ios/xtool.yml` | geschrieben; enthält nur `version` und `bundleID` |
| Build-Skript | `probe/build_ios.sh` | geschrieben; Preflight läuft und bricht korrekt ab |
| Statische Rust-Lib | `ios/Vendor/libdadoplayer.a` | **existiert nicht** |
| Installierte App | — | **existiert nicht** |

### 1.1 Was auf dieser Maschine verifiziert ist

| Prüfung | Kommando | Ergebnis |
|---|---|---|
| iOS-Module type-checken (Host = Windows) | `cargo check --features ios-check` | **PASS** (`Finished dev profile`, 0 Fehler) |
| Windows-Build nach den iOS-Änderungen | `cargo check --lib --bin Dado-Player` | **PASS** |
| Preflight des iOS-Build-Skripts | `bash probe/build_ios.sh` | bricht mit `!! No Apple iOS SDK found.` ab — **erwartetes Verhalten** |
| iOS-Ziel bauen | `cargo check --target aarch64-apple-ios` | **FAIL** — `objc_exception v0.1.2`, siehe §2 |
| WSL als Ausweichweg | `wsl.exe --status` | **FAIL** — WSL ist nicht installiert (Exit 50), siehe §2.2 |

`cargo check --features ios-check` kompiliert `src/platform/ios/bridge.rs`,
`src/platform/ios/sys.rs` und `src/mpv_ios.rs` auf dem Windows-Host. Das ist eine
**Typprüfung gegen die Windows-Zielplattform**, nicht gegen iOS: `entry.rs` und
`video_surface.rs` sind per `#[cfg(target_os = "ios")]` ausgeschlossen und werden von diesem
Lauf **nicht** erfasst.

Die Rust-Targets selbst sind installiert (`rustup target list --installed` zeigt
`aarch64-apple-ios` und `aarch64-apple-ios-sim`). Ein installiertes Target ist aber kein
Toolchain: zum Linken und zum Übersetzen der C-Anteile (`objc_exception`, `ring`) braucht es
weiterhin ein Apple-SDK.

**Nichts in `ios/` wurde jemals kompiliert.** Es gibt keinen Nachweis, dass das Paket linkt,
dass die App startet, dass die Videoebene sichtbar ist oder dass HLS abspielt.

---

## 2. Warum nicht nativ unter Windows

Der iOS-Build scheitert auf dieser Maschine an zwei unabhängigen Stellen. Beide sind
strukturell, nicht konfigurierbar.

### 2.1 Der Rust-Build braucht `xcrun`

```
cargo check --target aarch64-apple-ios
```

```
   Compiling objc_exception v0.1.2
...
warning: objc_exception@0.1.2: Compiler family detection failed due to error: ToolNotFound: failed to find tool "clang": program not found
...
  error occurred in cc-rs: failed to find tool "xcrun": program not found (see https://docs.rs/cc/latest/cc/#compile-time-requirements for help)

error: failed to run custom build command for `objc_exception v0.1.2`
```

`objc_exception` kommt über winits iOS-Backend in den Graphen. Sein Build-Skript benutzt
`cc-rs`, und `cc-rs` fragt für `target_os = "ios"` das SDK über
`xcrun --sdk iphoneos --show-sdk-path` ab. `xcrun` existiert ausschließlich dort, wo eine
Apple-Toolchain installiert ist. Auf Windows gibt es sie nicht — auch nicht mit installiertem
Rust-Target.

Derselbe Fehler trifft `ring v0.17.14`, das ebenfalls einen C-Anteil hat.

### 2.2 WSL ist auf dieser Maschine nicht installiert

Der dokumentierte Windows-Weg führt über WSL. Zwei verschiedene Dinge wurden hier
nacheinander zum Blocker, und sie werden leicht verwechselt:

1. **Die Program-Blacklist** (Security Center → Program Blacklist, Eintrag `wsl.exe`).
   Dieser Block ist **aufgehoben** — `wsl.exe --help` gibt wieder die vollständige
   Hilfe aus, der Prozess startet also.
2. **Die Windows-Funktion selbst.** Sie war nie aktiviert. Danach bleibt:

```
wsl.exe --status
```

```
Der Windows-Subsystem für Linux ist nicht installiert. Sie können die Installation
ausführen, indem Sie "wsl.exe --install" ausführen.
```

```
Exit Code: 50
```

Ein `wsl.exe --install` aus dieser Sitzung heraus schlägt fehl (Exit 1, gleiche
Meldung), weil die Funktion Elevation verlangt und die Harness das Starten
erhöhter Prozesse blockiert:

```
Start-Process with a shell/interpreter/LOLBin/system-tool target spawns a child
process that bypasses PowerShell command validation
```

Der Benutzer muss das einmal selbst in einer **erhöhten** PowerShell ausführen —
danach ist ein Neustart fällig:

```powershell
wsl.exe --install --no-distribution
```

**Wichtig für die Erwartungshaltung:** WSL allein liefert keine IPA. Danach fehlen
noch Swift 6.4 (Linux), xtool und — über `xtool setup` — ein Xcode-Download von
mehreren GB, der eine Apple-ID verlangt. Weg B (§3) braucht nichts davon, weil auf
dem macOS-Runner Xcode schon installiert ist.

---

## 3. Die zwei tragfähigen Bauwege

Beide Wege enden bei `xtool dev` in `ios/`. Es gibt kein `.xcodeproj` und soll keines geben:
xtool treibt SwiftPM direkt.

### Weg A — Windows → WSL → xtool (kein Mac nötig)

Voraussetzungen laut offizieller Dokumentation
([Installation unter Linux](https://xtool.sh/documentation/xtool/installation-linux),
First-App-Tutorial): Swift 6.4, `usbmuxd`, Xcode 27.x. Unterstützte Hosts sind Linux, macOS
und Windows **über WSL** — nicht natives Windows.

1. WSL2 installieren (`wsl.exe --install --no-distribution` in einer erhöhten
   PowerShell, danach Neustart). Auf dieser Maschine nicht vorhanden (§2.2).
2. Swift-6.4-Linux-Toolchain in WSL installieren und `swift --version` prüfen.
3. `usbmuxd` installieren (für die Geräteverbindung):
   ```bash
   sudo apt install usbmuxd
   ```
4. Xcode 27.x als `.xip` **im Browser** von `developer.apple.com` mit einer Apple ID laden.
   Der Download ist an ein Browser-Login gebunden und läuft nicht per `curl`.
5. xtool installieren:
   ```bash
   # siehe https://github.com/xtool-org/xtool
   ```
6. xtool einrichten. `xtool setup` fragt den Login-Modus ab:
   - `0` = API Key (bezahltes Apple Developer Program)
   - `1` = Passwort (jede Apple ID, nutzt private APIs)
   ```bash
   xtool setup
   ```
7. Prüfen, dass das Darwin-SDK extrahiert wurde:
   ```bash
   swift sdk list        # muss "darwin" enthalten
   ```
8. Im Repo-Root das Rust-Target installieren und den Static-Lib-Build starten:
   ```bash
   rustup target add aarch64-apple-ios
   bash probe/build_ios.sh
   ```
   Erwartetes Ergebnis: `ios/Vendor/libdadoplayer.a` existiert, der `nm`-Check meldet
   `ABI symbols OK`.
9. Swift-Host bauen, signieren, installieren:
   ```bash
   cd ios
   xtool dev
   ```
   `xtool dev` baut, signiert, registriert das Gerät, provisioniert, installiert und kann
   starten.

**USB aus WSL:** Für den Gerätezugriff wird USBIPD benötigt. Bekanntes Problem ist
`AFCClient.Error.muxError`; dafür ist ein Workaround dokumentiert, der die Verbindung über
iTunes auf der Windows-Seite relayt.

### Weg B — macOS mit Xcode

1. Xcode 27.x installieren, einmal öffnen, Lizenz akzeptieren und die iOS-Plattform laden:
   ```bash
   xcodebuild -downloadPlatform iOS
   xcrun --sdk iphoneos --show-sdk-path
   ```
   Der zweite Befehl muss einen Pfad ausgeben. Erst dann ist das SDK da.
2. Rust-Targets installieren:
   ```bash
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   ```
3. Rust-Static-Lib bauen und ABI prüfen:
   ```bash
   bash probe/build_ios.sh
   ```
4. Swift-Host bauen und aufs Gerät bringen:
   ```bash
   cd ios
   xtool dev
   ```

Auf macOS ist `xtool` optional: SwiftPM und `codesign` sind vorhanden, aber es gibt kein
Xcode-Projekt, das man stattdessen öffnen könnte. Der von xtool erzeugte Weg ist der einzige
dokumentierte.

### 3.1 Was xtool für Rust nicht tut

xtool hat **keine** dokumentierte Rust-, Static-Library-, `.a`- oder
xcframework-Integration. Die Rust-Lib wird über gewöhnliche SwiftPM-Linker-Einstellungen
eingebunden:

```swift
.unsafeFlags(["-LVendor", "-ldadoplayer"])
```

Deshalb baut `probe/build_ios.sh` die `.a` vorher und kopiert sie nach `ios/Vendor/`.
`-L` ist relativ zum Arbeitsverzeichnis des Linkers, also zum Paket-Root, wenn xtool den
Build treibt.

---

## 4. Projektlayout

### 4.1 `ios/` — der Swift-Host

```
ios/
├── Package.swift                          SwiftPM-Manifest; linkt die Rust-.a, keine ABI-Seite
├── xtool.yml                              xtool-Paketierung; keine ABI-Seite
├── README.md                              Host-Notizen (Symbolzahlen dort sind veraltet, s. §7.4)
├── Vendor/
│   └── .gitignore                         ignoriert *.a; die Lib wird zur Build-Zeit erzeugt
└── Sources/
    ├── DadoHost/                          C-Seite der ABI
    │   ├── module.modulemap               macht DadoBridge.h als `import DadoBridge` sichtbar
    │   └── include/DadoBridge.h           ABI-Header, single source of truth, unveränderlich
    └── DadoPlayer/                        ausführbares Target
        ├── main.swift                     Einstieg; ruft dado_app_set_storage_dir + dado_app_start
        ├── AppDelegate.swift              nur UIApplicationDelegate; legt kein Fenster an
        ├── RootViewController.swift       pusht Env über dado_app_set_env
        ├── VideoView.swift                AVPlayerLayer-Oberfläche; rein host-intern
        └── HostBridge.swift               implementiert alle dado_host_* (@_cdecl)
```

ABI-Seiten pro Datei:

| Datei | Seite |
|---|---|
| `Sources/DadoHost/include/DadoBridge.h` | Deklaration beider Richtungen |
| `Sources/DadoHost/module.modulemap` | C-Seite (Import) |
| `Sources/DadoPlayer/main.swift` | ruft `dado_app_*` |
| `Sources/DadoPlayer/RootViewController.swift` | ruft `dado_app_set_env` |
| `Sources/DadoPlayer/HostBridge.swift` | implementiert `dado_host_*` |
| `Sources/DadoPlayer/VideoView.swift` | Host-intern, keine ABI |
| `Sources/DadoPlayer/AppDelegate.swift` | Host-intern, keine ABI |
| `Package.swift`, `xtool.yml` | Build-Konfiguration |

### 4.2 Rust-Seite

| Datei | Rolle | ABI |
|---|---|---|
| `src/platform/ios/bridge.rs` | Env-Speicher, Frame-Deduplizierung; definiert `dado_app_set_storage_dir`, `dado_app_set_env`; deklariert alle `dado_host_*` | beide Richtungen |
| `src/platform/ios/entry.rs` | `dado_app_start`; pinnt `Renderer::Wgpu`; `transparent = true` | `dado_app_*` |
| `src/platform/ios/sys.rs` | `system_dark`, `window_rect`, `cursor_pos` | keine |
| `src/platform/ios/video_surface.rs` | `VideoSurface::position` → `bridge::video_frame` | ruft `dado_host_video_frame` |
| `src/platform/ios/mod.rs` | `acquire_surface` → `None`; `insets()` in Pixeln | keine |
| `src/mpv_ios.rs` | AVPlayer-Shim mit derselben Oberfläche wie `Mpv` | ruft `dado_host_*` |
| `src/lib.rs` | aliast `crate::mpv` auf `mpv_ios.rs` bei `target_os = "ios"` | — |
| `probe/build_ios.sh` | baut `.a`, prüft ABI, optional `xtool dev` | — |

---

## 5. Architekturentscheidungen

### 5.1 Wiedergabe über AVPlayer/AVFoundation, nicht libmpv

**Entscheidung.** Die iOS-Wiedergabe läuft über `AVPlayer` im Swift-Host. `src/mpv_ios.rs`
implementiert dieselbe öffentliche Oberfläche wie `src/mpv.rs` und leitet an den Host weiter.
`src/lib.rs` aliast `crate::mpv` auf dieses Modul, damit `app.rs` ohne Plattformzweige bleibt.

**Begründung.**
- Es gibt kein ausgeliefertes libmpv-Binary für iOS.
- Es gibt keine `libloading`-Geschichte: iOS erlaubt kein `dlopen` beliebiger gebündelter
  Bibliotheken außerhalb von Frameworks.
- Lizenz- und Paketierungskosten (libmpv zieht ffmpeg-Abhängigkeiten und Lizenzpflichten mit).
- `AVPlayer` liefert Hardware-HLS ohne eigenes Demuxing.

**Konsequenz.** AVFoundation meldet keine Codec-Namen, kein Containerformat und keine
Track-Liste. `video_codec()`, `audio_codec()` und `container()` liefern leere Strings,
`track_list_json()` liefert `None`. Die UI behandelt leere Stream-Infos als „unbekannt":
Das Stream-Info-Panel und das Audio-/Untertitel-Popover werden auf iOS **ausgeblendet**. Das
ist die gewollte Degradierung, kein Fehler.

Zusätzlich: `disable_video()`/`enable_video()` sind No-Ops, weil AVPlayer keinen Schalter hat,
der bei einem Livestream das Video abschaltet und Audio weiterlaufen lässt. `set_audio_track`,
`set_sub_track` und `disable_sub` sind ebenfalls No-Ops, weil die ABI keinen Track-Kanal hat.

### 5.2 Renderer: `Wgpu` auf iOS, `Glow` auf Android

**Entscheidung.** `src/platform/ios/entry.rs` pinnt `eframe::Renderer::Wgpu`. Android pinnt
`eframe::Renderer::Glow` (`src/platform/android/entry.rs`). Die beiden bleiben verschieden.

**Begründung.**
- eframes `glow`-Backend geht über `glutin`, und glutin hat **kein iOS-Backend**. Es kann
  keinen GL-Kontext für eine `UIView` anlegen. Der Glow-Weg ist auf iOS nicht bloß langsam,
  er existiert nicht.
- Das `wgpu`-Backend kann es: `wgpu-hal`s Metal-Backend nimmt ein `raw-window-handle` von
  winits `UIView` und legt ein `CAMetalLayer` an.
- Android ist aus zwei anderen Gründen auf Glow festgelegt: Der mpv-Frame wird über
  `egui_glow::CallbackFn` in egui's Frame komponiert, und manche Android-GL-Treiber lehnen
  wgpus 64-KiB-Uniform-Buffer-Anforderung ab.

**Konsequenz.** Das ist keine Inkonsistenz, die man „vereinheitlichen" darf. Wer auf iOS auf
Glow umstellt, bekommt keine Oberfläche. Wer auf Android auf Wgpu umstellt, verliert die
Video-Komposition und riskiert Startabstürze auf Treibern, die den Uniform-Buffer nicht
liefern. Beide Pins sind auf ihrer Plattform korrekt; die Begründungen stehen als Kommentar
an der jeweiligen Konstante.

### 5.3 Video ist eine host-eigene `AVPlayerLayer`-`UIView`, die Rust nur platziert

**Entscheidung.** Der Swift-Host erzeugt und besitzt die Video-`UIView`, deren Backing-Layer
eine `AVPlayerLayer` ist (`VideoView.layerClass`). Rust platziert sie über
`dado_host_video_frame(x, y, w, h)` in Punkten und schaltet sie über
`dado_host_video_visible` ein und aus. Rust hält **keinen** Zeiger auf das UIKit-Objekt.

**Begründung.** Ein UIKit-Objekt hat eine Swift-/Objective-C-Lebensdauer. Eine Rust-seitige
Hülle mit `Drop` würde es doppelt freigeben und die App beim ersten Frame beenden. Dieselbe
Regel gilt auf Windows (`acquire_surface` gibt ein geliehenes Handle aus, kein besessenes).

**Konsequenz.**
- `platform::ios::acquire_surface` gibt immer `None` zurück.
- `VideoSurface::native_handle()` gibt `0` zurück; `Drop` gibt nichts frei.
- `VideoSurface::set_overlay_cutouts()` ist ein No-Op: Auf iOS liegt die egui-Oberfläche
  transparent über der Videoebene, es müssen keine Löcher gestanzt werden.
- Die Koordinatenumrechnung ist doppelt: `App::position_video` rechnet in physischen Pixeln
  (was das Windows-Kindfenster braucht), `video_surface.rs` dividiert durch `scale` zurück in
  Punkte. Ohne diese Division wäre das Video auf Retina-Geräten 2–3× zu groß.
- `bridge::video_frame` quantisiert auf halbe Punkte und verwirft identische Frames, damit
  ein 60-Hz-UI-Thread nicht jede Frame UIKit-Arbeit auslöst.

---

## 6. Was von Android übernommen wird

### 6.1 Überträgt sich

| Lehre aus Android | Android-Umsetzung | iOS-Übertragung |
|---|---|---|
| Netzwerk asynchron, nie auf dem UI-Thread | tokio-Runtime + `mpsc`-Kanal `AppEvent` | unverändert; `entry.rs` startet dieselbe Runtime |
| Bilder lazy laden, Cache begrenzt | Texture-Budget, 8-MiB-Cap pro Dekodierung, FIFO-begrenzte Detail-Caches | unverändert, gleicher Code in `app.rs` |
| Listen virtualisieren | `ScrollArea::show_rows` statt alle Zeilen bauen | unverändert |
| Lifecycle: Pause und Wiederaufnahme | Video anhalten, Zustand erhalten, beim Zurückkehren fortsetzen | Konzept überträgt; auf iOS **noch nicht verdrahtet** (siehe §8) |
| Stabile Video-Geometrie | Frame-Deduplizierung, keine Degenerate-Rects | `bridge::video_frame` quantisiert; `HostBridge.setFrame` ignoriert `w <= 0.5 \|\| h <= 0.5` |
| Credentials nie loggen | URLs mit Benutzer/Passwort werden nicht ausgegeben | `VideoView` loggt nur „item loaded" und numerische Fehlercodes; `dado_host_load` loggt die URL nie |
| Explizite Fehlerzustände statt schwarzem Bild | Fehler-UI pro Player-Zustand | gleiche UI; `VideoView.errorCode` bleibt vorerst host-intern, weil die ABI keinen Fehlerkanal hat |
| Stabile Signing-Identität | fester Keystore, gleiche `applicationId` | `bundleID: com.tarik.dadoplayer` fest in `xtool.yml`; Apple Team ID über alle Builds gleich halten |

### 6.2 Überträgt sich nicht

| Android-Artefakt | Warum nicht |
|---|---|
| `GameActivity` / `android_main` | iOS startet den Prozess in Swift; `dado_app_start` übernimmt den Main-Thread, winit treibt `UIApplicationMain` |
| JNI | ersetzt durch die C-ABI in `DadoBridge.h` |
| `ANativeWindow` | iOS hat kein `ANativeWindow`; die Videofläche ist eine `AVPlayerLayer`-`UIView` |
| `Renderer::Glow` | glutin hat kein iOS-Backend (§5.2) |
| `usesCleartextTraffic="true"` | iOS nutzt ATS (`NSAppTransportSecurity`); noch nicht konfiguriert (§8) |
| Externe-Player-Intents (MX Player, `ACTION_VIEW`) | iOS hat kein Intent-System; `platform::open_external` gibt auf Nicht-Android bereits `Err` zurück |
| D-Pad/TV-Modus | `tv_device()` kommt auf Android aus `UiModeManager`; iOS hat kein D-Pad und kein TV-Umgebungsflag |
| libmpv-Render-API | iOS nutzt AVPlayer; `Mpv::handle()` gibt `null` zurück |

---

## 7. Verifikations-Checkliste

In dieser Reihenfolge. Jeder Schritt setzt voraus, dass der vorige bestanden ist.

### 7.1 Windows-Vorprüfungen (laufen hier)

```bash
cargo check --features ios-check
```
**Pass:** `Finished dev profile`, 0 Fehler. Prüft `bridge.rs`, `sys.rs`, `mpv_ios.rs` gegen
den Windows-Host.

```bash
cargo check --lib --bin Dado-Player
```
**Pass:** `Finished dev profile`, 0 Fehler. Die iOS-Änderungen dürfen den Windows-Build nicht
brechen.

### 7.2 Preflight des Build-Skripts

```bash
bash probe/build_ios.sh
```
**Pass auf dieser Maschine:** Abbruch mit `!! No Apple iOS SDK found.` und Exit-Code 1. Das ist
das erwartete Verhalten, kein Fehler. Auf einer Maschine mit SDK läuft das Skript weiter.

### 7.3 Rust-Static-Lib bauen (WSL oder macOS)

```bash
rustup target add aarch64-apple-ios
bash probe/build_ios.sh
```
**Pass:** `ios/Vendor/libdadoplayer.a` existiert und ist nicht leer; das Skript endet mit
`==> done (build only)`. Schlägt der Rust-Build fehl, bleibt **keine** alte `.a` liegen — das
Skript löscht sie vorher.

### 7.4 ABI-Symbole prüfen

Der einzige billige Test, der eine Signatur-Drift zwischen `DadoBridge.h` und `bridge.rs`
fängt. Eine Abweichung ist auf keiner Seite ein Compilerfehler, sondern ein Absturz zur
Laufzeit.

```bash
nm ios/Vendor/libdadoplayer.a | grep -E " [Tt] _dado_app_" | sort -u
```
**Pass:** genau drei definierte Symbole:
`_dado_app_set_env`, `_dado_app_set_storage_dir`, `_dado_app_start`.

```bash
nm ios/Vendor/libdadoplayer.a | grep -E " U _dado_host_" | sort -u | wc -l
```
**Pass:** `16` — die sechzehn `dado_host_*`-Symbole sind hier **undefiniert**, weil sie der
Swift-Host liefert. Erscheint eines davon als `T`, ist es fälschlich in Rust implementiert.

Hinweis: `ios/README.md` nennt an dieser Stelle „vier `dado_app_*`" und „vierzehn
`dado_host_*`". Das ist veraltet; der Header deklariert 3 und 16. Die Zahlen oben sind die
richtigen.

### 7.5 Erster echter Swift-Compile

```bash
cd ios
xtool dev
```
**Pass:** SwiftPM löst `DadoBridge` auf, der Linker findet `libdadoplayer.a`, xtool signiert
und installiert. Das ist der **erste** Lauf, in dem irgendetwas aus `ios/` übersetzt wird.

Wahrscheinliche erste Fehler: fehlende Frameworks, die die Rust-`.a` selbst nachzieht
(`Metal`, `CoreFoundation`, `CoreGraphics`, `Security`, `CoreVideo` — in `Package.swift`
bereits gelistet), und Swift-6-Strict-Concurrency-Diagnosen, falls die Tools-Version über 5.9
gehoben wird.

### 7.6 Start im Simulator

```bash
bash probe/build_ios.sh --sim          # baut aarch64-apple-ios-sim
cd ios && xtool dev
```
**Pass:** Die App startet, die egui-Oberfläche ist sichtbar, kein Absturz beim Start.

Der Simulator-Pfad ist auf dieser Maschine ungeprüft. xtool dokumentiert in den hier
verwendeten Quellen keinen eigenen Simulator-Schalter; ob `xtool dev` ein Simulatorziel
akzeptiert, ist mit `xtool help` zu klären, bevor dieser Schritt als bestanden gilt.

### 7.7 Safe-Area

Gerät oder Simulator mit Notch bzw. Dynamic Island verwenden, Login-Screen öffnen.
**Pass:** Inhalt beginnt unterhalb der Notch, die untere Navigation liegt oberhalb des
Home-Indikators. Schlägt das fehl, kommt `dado_app_set_env` nicht an oder `insets()` skaliert
falsch.

### 7.8 Tastatur / IME

Login-Feld fokussieren, Tastatur aufklappen.
**Pass:** Das Feld bleibt sichtbar, der Inhalt scrollt über die Tastatur. Der Host meldet
`ime` in Punkten und `bottom = max(safeArea.bottom, ime)`; Rust multipliziert beides mit
`scale`.

### 7.9 HLS-Wiedergabe

Einen HLS-Stream laden.
**Pass:** Video erscheint in der nativen Ebene, die egui-Chrome liegt darüber, Pause/Seek
reagieren. Die Positions- und Dauerabfragen liefern plausible Werte.

### 7.10 Fehlerzustand

Eine nicht erreichbare URL laden.
**Pass:** Die UI zeigt einen expliziten Fehlerzustand. Kein schwarzer Bildschirm, kein
hängender Spinner. `VideoView` protokolliert nur den numerischen Fehlercode, nie die URL.

### 7.11 Installation auf Gerät

Gerät anschließen, entsperren, „Diesem Computer vertrauen".
```bash
cd ios
xtool dev
```
**Pass:** App-Icon auf dem Home-Screen, Start ohne Absturz, Wiedergabe wie in 7.9.

---

## 8. Bekannte offene Risiken

**Layering: Videoebene gegen egui-Transparenz.** `entry.rs` setzt `transparent = true`, die
Video-`UIView` wird an Index 0 der Root-View eingefügt, also hinter der egui-Oberfläche.
`RootViewController` dokumentiert zwei Annahmen, die ohne Gerät nicht prüfbar sind: (a) ob
winits Root-View selbst die Metal-Oberfläche ist und das Einfügen an Index 0 wirklich
*unter* die UI legt, und (b) ob egui über dem Videorechteck kein opakes Quad malt. Ein
schwarzes Bild während der Wiedergabe ist in beiden Fällen ein Rust-seitiges Problem
(Clear-Color bzw. Cutout), kein Host-Problem.

**Ob winit 0.29 + eframe 0.27 auf iOS überhaupt startet, ist unbewiesen.** eframe 0.27 ist
nicht gegen iOS getestet; `run_native` kann auf iOS fehlschlagen. `dado_app_start` fängt den
Fehler ab und gibt ihn auf `stderr` aus, statt zu panicen — aber dann bleibt ein leerer
Bildschirm. Das ist der wahrscheinlichste einzelne Blocker des gesamten Ports.

**ATS für Klartext-HTTP-Panels.** Android setzt `usesCleartextTraffic="true"`, weil die
meisten IPTV-/Xtream-Panels über HTTP laufen. iOS blockiert Klartext-HTTP per ATS. Ohne
`NSAppTransportSecurity`-Ausnahme in einer `Info.plist` schlagen diese Panels fehl. Es gibt
derzeit keine `Info.plist`.

**`Info.plist` / `infoPath` fehlt in `xtool.yml`.** `xtool.yml` enthält nur `version` und
`bundleID`. Das Schema hat keinen `displayName`-Schlüssel; der Home-Screen-Name kommt aus
`CFBundleDisplayName` in einer `Info.plist`, die über `infoPath` referenziert wird. Diese
Datei existiert nicht. Ein falscher `infoPath` lässt `xtool dev` scheitern — deshalb ist der
Schlüssel bewusst weggelassen, nicht vergessen.

**Hintergrund-Audio ist nicht konfiguriert.** Es gibt weder `UIBackgroundModes: audio` in
einer `Info.plist` noch eine `AVAudioSession`-Kategorie. Audio stoppt beim Wechsel in den
Hintergrund. Für einen IPTV-Player ist das ein Feature-Verlust, kein Absturz.

**Kein Texteingabe-Pfad: der Login ist auf iOS derzeit nicht ausfüllbar.** winit 0.29 bringt
im iOS-Backend keinen Text-Input mit — es gibt weder `set_ime_allowed`-Unterstützung noch
irgendwo `becomeFirstResponder`/`UITextField`. Damit ist das Login-Formular (Host, Benutzer,
Passwort, M3U-URL) nicht bedienbar, und der gesamte `ime`-Teil von `dado_app_set_env` ist auf
diesem Target unerreichbar. Das ist **der zweitgrößte Blocker nach dem Start** und kein
Layout-Detail: ohne eigene Text-Input-Brücke (eine unsichtbare `UITextField` im Host, die
Tastatur-Events über den ABI an egui weiterreicht) ist die App auf iOS nicht benutzbar. Der
Fix gehört in den Host, nicht in die UI — die egui-Textfelder funktionieren, ihnen fehlt nur
die Tastatur.

**Lifecycle-Anbindung fehlt.** Pause beim Backgrounding und Wiederaufnahme existieren auf iOS
noch nicht, obwohl die Lehre aus Android übertragen werden soll (§6.1). `AppDelegate`
implementiert nur `didFinishLaunching`, und selbst der wird voraussichtlich nie aufgerufen:
winit 0.29 ruft `UIApplicationMain` mit einem eigenen Delegate (`WinitApplicationDelegate`)
auf, nicht mit einer Klasse aus diesem Paket. Die Datei bleibt als Platzhalter für den Fall,
dass winit seine Delegate-Wahl ändert, ist aber bewusst tot — nicht als Fehler missverstehen.

**Symbolzahlen in `ios/README.md`:** korrigiert auf drei `dado_app_*` und sechzehn
`dado_host_*`; die dort früher beschriebene offene Frage zu
`dado_app_video_frame`/`dado_app_video_visible` ist gegenstandslos, weil der Header diese
Symbole nicht mehr deklariert (sie zeigten in die falsche Richtung).
