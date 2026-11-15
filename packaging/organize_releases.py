"""Keep current downloads at the top level; archive previous packages without deleting them."""
import hashlib
import json
import plistlib
import re
import shutil
import sys
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
    base = target
    index = 2
    while target.exists():
        target = base.with_name(f"{base.stem}-{index}{base.suffix}")
        index += 1
    shutil.move(str(file), str(target))
    print(f"Archived {file.name} → {target.relative_to(RELEASES)}")


def install_windows(package):
    """Keep one canonical download and preserve the previous file before replacing it."""
    package = Path(package).resolve()
    destination = RELEASES / "Dado-Player.exe"
    RELEASES.mkdir(exist_ok=True)
    if not package.is_file():
        raise RuntimeError(f"Portable package missing: {package}")
    if package == destination.resolve():
        return
    incoming = hashlib.sha256(package.read_bytes()).digest()
    if destination.exists() and hashlib.sha256(destination.read_bytes()).digest() != incoming:
        backup = RELEASES / "Archiv" / "Windows" / "vorherige-Datei" / f"Dado-Player-{hashlib.sha256(destination.read_bytes()).hexdigest()[:12]}.exe"
        backup.parent.mkdir(parents=True, exist_ok=True)
        if not backup.exists():
            shutil.copy2(destination, backup)
        elif hashlib.sha256(backup.read_bytes()).digest() != hashlib.sha256(destination.read_bytes()).digest():
            raise RuntimeError("Windows archive backup differs; refusing to replace it")
    staging = RELEASES / "Dado-Player.exe.pending"
    shutil.copy2(package, staging)
    if hashlib.sha256(staging.read_bytes()).digest() != incoming:
        raise RuntimeError("Windows download copy verification failed")
    # A running/locked EXE fails here rather than silently publishing a second name.
    staging.replace(destination)


def main():
    RELEASES.mkdir(exist_ok=True)
    if len(sys.argv) == 3 and sys.argv[1] == "--install-windows":
        install_windows(sys.argv[2])
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
    windows_file = "Dado-Player.exe"
    for file in RELEASES.glob("Dado-Player-Windows-*.exe"):
        match = re.fullmatch(r"Dado-Player-Windows-(\d+\.\d+\.\d+)\.exe", file.name)
        if match:
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
