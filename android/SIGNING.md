# Release-Signierung (GitHub Secrets)

Die Release-APK wird in CI mit `r0adkll/sign-android-release@v1` signiert. Dafür
werden vier Repository-Secrets benötigt. Der Keystore selbst wird **nicht** ins
Repo committet (`.gitignore` deckt `*.keystore` / `*.jks` ab).

## 1. Keystore erzeugen (lokal, JDK 17 nötig)

```sh
keytool -genkeypair -v \
  -keystore dadoplayer-release.keystore \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias dadoplayer
```

Merke dir `alias` (hier `dadoplayer`), das Key-Passwort und das Keystore-Passwort.

## 2. Keystore als base64 für `SIGNING_KEY`

- macOS / Linux:
  ```sh
  base64 -w0 dadoplayer-release.keystore | pbcopy   # bzw. > keystore.b64
  ```
- Windows (PowerShell):
  ```powershell
  certutil -encode dadoplayer-release.keystore keystore.b64
  # den Inhalt von keystore.b64 kopieren (die -----BEGIN/END----- Zeilen weglassen)
  ```
- Windows (Git Bash / WSL):
  ```sh
  base64 -w0 dadoplayer-release.keystore > keystore.b64
  ```

## 3. Secrets in GitHub anlegen

Repo → **Settings → Secrets and variables → Actions → New repository secret**:

| Secret                | Wert                         |
|-----------------------|------------------------------|
| `SIGNING_KEY`         | base64-String aus Schritt 2  |
| `KEY_ALIAS`           | `dadoplayer`                 |
| `KEY_PASSWORD`        | Key-Passwort                 |
| `KEY_STORE_PASSWORD`  | Keystore-Passwort            |

## 4. Build triggern

- **Debug-APK**: `push` auf `main` / Tag `v*` / `workflow_dispatch`
  (keine Secrets nötig). Artifact: `dado-player-debug-apk`.
- **Signierte Release-APK**: wie oben, **aber erst nach** Anlegen der Secrets.
  Artifact: `dado-player-release-apk`.

> Hinweis: Solange der Repo-Inhalt nicht auf GitHub gepusht ist, läuft der
> Workflow gar nicht — die Signierung setzt also zwingend einen Remote-Push
> (mit den angelegten Secrets) voraus.
