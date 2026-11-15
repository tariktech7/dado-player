"""Fetch the pinned official VLCKit SDK; never link an unchecked/partial download."""
import hashlib
import json
import pathlib
import shutil
import sys
import tarfile
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent
NAME = "MobileVLCKit-3.7.3-319ed2c0-79128878.tar.xz"
SHA256 = "0d04059906962ddc9a7bd1ebaa12e1f9ae85eb2466116a97a2f46886dd27a0a9"

def tree_hash(root):
    digest = hashlib.sha256()
    if not root.is_dir():
        return ""
    for file in sorted(p for p in root.rglob("*") if p.is_file()):
        digest.update(file.relative_to(root).as_posix().encode())
        with file.open("rb") as stream:
            digest.update(hashlib.file_digest(stream, "sha256").digest())
    return digest.hexdigest()

def prepare(simulator=False):
    cache = ROOT / "SDKs"
    cache.mkdir(exist_ok=True)
    archive = cache / NAME
    def verified():
        return archive.is_file() and hashlib.file_digest(archive.open("rb"), "sha256").hexdigest() == SHA256
    if not verified():
        temporary = archive.with_suffix(".download")
        urllib.request.urlretrieve("https://download.videolan.org/cocoapods/prod/" + NAME, temporary)
        if hashlib.file_digest(temporary.open("rb"), "sha256").hexdigest() != SHA256:
            raise RuntimeError("VLCKit checksum mismatch")
        temporary.replace(archive)
    slice_name = "ios-arm64_i386_x86_64-simulator" if simulator else "ios-arm64_armv7_armv7s"
    destination = ROOT / "Vendor" / "MobileVLCKit.framework"
    marker = cache / "vlckit-prepared.json"
    if marker.exists():
        record = json.loads(marker.read_text())
        if record.get("archive") == SHA256 and record.get("slice") == slice_name and record.get("tree") == tree_hash(destination) and (ROOT / "Resources/VLCKit-COPYING.txt").exists():
            print("Verified cached MobileVLCKit 3.7.3:", slice_name)
            return
    source = cache / "MobileVLCKit-binary" / "MobileVLCKit.xcframework" / slice_name / "MobileVLCKit.framework"
    # Re-extract the selected slice from the verified archive, not a stale local SDK.
    prefix = f"MobileVLCKit-binary/MobileVLCKit.xcframework/{slice_name}/"
    with tarfile.open(archive) as package:
        selected = [m for m in package.getmembers() if m.name.startswith(prefix) or m.name == "MobileVLCKit-binary/COPYING.txt"]
        package.extractall(cache, members=selected, filter="data")
    if destination.exists():
        if not destination.resolve().is_relative_to((ROOT / "Vendor").resolve()):
            raise RuntimeError("Framework cleanup escaped ios/Vendor")
        shutil.rmtree(destination)
    # NTFS/WSL may refuse copystat/chmod on files created by Windows tar.
    # Framework contents are ordinary files; permissions/owners are not inputs.
    for entry in source.rglob("*"):
        target = destination / entry.relative_to(source)
        if entry.is_dir():
            target.mkdir(parents=True, exist_ok=True)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(entry, target)
    shutil.copyfile(cache / "MobileVLCKit-binary/COPYING.txt", ROOT / "Resources/VLCKit-COPYING.txt")
    marker.write_text(json.dumps({"archive": SHA256, "slice": slice_name, "tree": tree_hash(destination)}))
    print("Verified MobileVLCKit 3.7.3:", slice_name)

if __name__ == "__main__":
    prepare("--sim" in sys.argv)
