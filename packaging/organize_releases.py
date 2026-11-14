"""Keep current downloads at the top level; archive previous packages without deleting them."""
import hashlib
import json
import plistlib
import re
import shutil
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RELEASES = (ROOT / "Releases").resolve()


def archive(file, platform, version):
    target = RELEASES / "Archiv" / platform / version / file.name
    if not file.resolve().is_relative_to(RELEASES) or not target.resolve().is_relative_to(RELEASES):
        raise RuntimeError("Archive path escaped Releases")
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        digest = hashlib.sha256(file.read_bytes()).hexdigest()[:12]
        target = target.with_name(f"{target.stem}-{digest}{target.suffix}")
    if target.exists():
        raise RuntimeError(f"Archive already contains this package: {target}")
    shutil.move(str(file), str(target))
    print(f"Archived {file.name} → {target.relative_to(RELEASES)}")


def main():
    RELEASES.mkdir(exist_ok=True)
    tv = json.loads((ROOT / "SmartTV/client/package.json").read_text())["version"]
    for file in RELEASES.glob("Dado-Player-Tizen-*.wgt"):
        version = re.fullmatch(r"Dado-Player-Tizen-(\d+\.\d+\.\d+)(?:-unsigned)?\.wgt", file.name)
        if version and version[1] != tv:
            archive(file, "Samsung-Tizen", version[1])
    ipa = RELEASES / "Dado-Player-unsigned.ipa"
    ios = {"status": "Kein aktueller Geräte-Build vorhanden"}
    current_ipa = None
    if ipa.exists():
        with zipfile.ZipFile(ipa) as package:
            info = next(n for n in package.namelist() if re.fullmatch(r"Payload/[^/]+\.app/Info.plist", n))
            metadata = plistlib.loads(package.read(info))
        source = plistlib.loads((ROOT / "ios/Info.plist").read_bytes())
        if any(metadata[key] != source[key] for key in ("CFBundleVersion", "CFBundleShortVersionString")):
            version = f"{metadata['CFBundleShortVersionString']}-Build-{metadata['CFBundleVersion']}"
            archive(ipa, "iOS", version)
        else:
            current_ipa = metadata
            ios = dict(status="Aktueller Geräte-Build; vor Installation mit eigener Apple-ID signieren",
                       version=metadata["CFBundleShortVersionString"], build=metadata["CFBundleVersion"],
                       file=ipa.name)
    windows = (ROOT / "windows/VERSION").read_text().strip()
    versioned_windows = f"Dado-Player-Windows-{windows}.exe"
    windows_file = versioned_windows if (RELEASES / versioned_windows).exists() else "Dado-Player.exe"
    legacy_windows = windows_file != "Dado-Player.exe" and (RELEASES / "Dado-Player.exe").exists()
    for file in RELEASES.glob("Dado-Player-Windows-*.exe"):
        match = re.fullmatch(r"Dado-Player-Windows-(\d+\.\d+\.\d+)\.exe", file.name)
        if match and match[1] != windows:
            archive(file, "Windows", match[1])
    gradle = (ROOT / "android/app/build.gradle").read_text()
    android = re.search(r'versionName "([^"]+)"', gradle)[1]
    current = []
    for platform, version, name in [
        ("Windows portable", windows, windows_file),
        ("Android", android, "Dado-Player-arm64-debug.apk"),
        ("Samsung TV", tv, f"Dado-Player-Tizen-{tv}.wgt"),
        ("Tizen Entwicklerpaket, unsigniert", tv, f"Dado-Player-Tizen-{tv}-unsigned.wgt"),
    ]:
        file = RELEASES / name
        if file.exists():
            current.append(dict(platform=platform, version=version, file=name, bytes=file.stat().st_size,
                                sha256=hashlib.sha256(file.read_bytes()).hexdigest()))
    if current_ipa:
        current.append(dict(platform="iOS (AltStore/Sideloadly)",
                            version=f"{ios['version']} / Build {ios['build']}", file=ipa.name,
                            bytes=ipa.stat().st_size, sha256=hashlib.sha256(ipa.read_bytes()).hexdigest()))
    (RELEASES / "AKTUELL.json").write_text(json.dumps(dict(packages=current, ios=ios), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    lines = ["# Aktuelle Dado-Player-Downloads", "", "| Plattform | Version | Datei |", "| --- | --- | --- |"]
    lines += [f"| {p['platform']} | {p['version']} | [{p['file']}]({p['file']}) |" for p in current]
    if legacy_windows:
        lines += ["", "Hinweis: Dado-Player.exe ist noch die bisherige geöffnete/gesperrte Datei. Für die neue Windows-Version die oben verlinkte Datei mit Versionsnummer verwenden."]
    ios_note = ("iOS: Aktuelle IPA mit AltStore/Sideloadly und eigener Apple-ID signieren und installieren. "
                "Ein Paketbuild ersetzt keinen Wiedergabetest auf dem iPhone." if current_ipa else
                "iOS: Kein aktueller Geräte-Build. Alte IPA-Dateien unter Archiv/iOS enthalten die aktuellen Änderungen nicht.")
    lines += ["", "Windows: portable EXE direkt starten. Android: signierte APK installieren; der historische Dateiname enthält weiterhin ‚debug‘.",
              "Samsung: signierte WGT für den eingerichteten Entwickler-TV. Installation über den PC; nicht per USB-Dateiauswahl.",
              "Die unsignierte WGT ist nur für Entwickler und benötigt ein Samsung-Zertifikat für das jeweilige Gerät.",
              "", ios_note,
              "", "Archiv enthält vorherige Builds nach Plattform und Version. Es wurde kein Paket gelöscht.",
              "AKTUELL.json enthält Dateigrößen und SHA-256 zur Kontrolle.", ""]
    (RELEASES / "README.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"Current packages: {len(current)}; index: Releases/README.md")


if __name__ == "__main__":
    main()
