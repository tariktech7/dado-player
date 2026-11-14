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
    ios = {"status": "Kein aktueller Geräte-Build; GitHub macOS-Runner durch Billing blockiert"}
    if ipa.exists():
        with zipfile.ZipFile(ipa) as package:
            info = next(n for n in package.namelist() if re.fullmatch(r"Payload/[^/]+\.app/Info.plist", n))
            metadata = plistlib.loads(package.read(info))
        source = plistlib.loads((ROOT / "ios/Info.plist").read_bytes())
        if metadata["CFBundleVersion"] != source["CFBundleVersion"]:
            version = f"{metadata['CFBundleShortVersionString']}-Build-{metadata['CFBundleVersion']}"
            archive(ipa, "iOS", version)
    windows = (ROOT / "windows/VERSION").read_text().strip()
    gradle = (ROOT / "android/app/build.gradle").read_text()
    android = re.search(r'versionName "([^"]+)"', gradle)[1]
    current = []
    for platform, version, name in [
        ("Windows portable", windows, "Dado-Player.exe"),
        ("Android", android, "Dado-Player-arm64-debug.apk"),
        ("Samsung TV", tv, f"Dado-Player-Tizen-{tv}.wgt"),
        ("Tizen Entwicklerpaket, unsigniert", tv, f"Dado-Player-Tizen-{tv}-unsigned.wgt"),
    ]:
        file = RELEASES / name
        if file.exists():
            current.append(dict(platform=platform, version=version, file=name, bytes=file.stat().st_size,
                                sha256=hashlib.sha256(file.read_bytes()).hexdigest()))
    (RELEASES / "AKTUELL.json").write_text(json.dumps(dict(packages=current, ios=ios), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    lines = ["# Aktuelle Dado-Player-Downloads", "", "| Plattform | Version | Datei |", "| --- | --- | --- |"]
    lines += [f"| {p['platform']} | {p['version']} | [{p['file']}]({p['file']}) |" for p in current]
    lines += ["", "Windows: portable EXE direkt starten. Android: signierte APK installieren; der historische Dateiname enthält weiterhin ‚debug‘.",
              "Samsung: signierte WGT für den eingerichteten Entwickler-TV. Installation über den PC; nicht per USB-Dateiauswahl.",
              "Die unsignierte WGT ist nur für Entwickler und benötigt ein Samsung-Zertifikat für das jeweilige Gerät.",
              "", "iOS: Hier liegt bewusst keine angeblich neue IPA. Die alte IPA 0.2.1/Build 4 ist unter Archiv/iOS abgelegt und enthält die aktuellen Änderungen nicht.",
              "", "Archiv enthält vorherige Builds nach Plattform und Version. Es wurde kein Paket gelöscht.",
              "AKTUELL.json enthält Dateigrößen und SHA-256 zur Kontrolle.", ""]
    (RELEASES / "README.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"Current packages: {len(current)}; index: Releases/README.md")


if __name__ == "__main__":
    main()
