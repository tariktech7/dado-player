"""Stage the pinned VideoLAN x64 runtime next to the Windows executable."""
import argparse
import hashlib
import re
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.request import urlopen
from zipfile import ZipFile

VERSION = "3.0.24"
ARCHIVE = f"vlc-{VERSION}-win64.zip"
SHA256 = "fcf30850371ad10c9373cc4f0f4501e7dee49e3e9ae9f20c72fb2661a1ca6323"
SOURCE = f"https://download.videolan.org/vlc/{VERSION}/"
BINARIES = ["libvlc.dll", "libvlccore.dll"]
NOTICES = "VLC-Notices.txt"
COPYING = "VLC-COPYING.txt"
PLUGIN = re.compile(r"lib[a-z0-9_+.-]+_plugin\.dll", re.IGNORECASE)


def prepare(destination, archive=None):
    destination = Path(destination)
    destination.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix="dado-vlc-") as temporary:
        if archive is None:
            archive = Path(temporary) / ARCHIVE
            with urlopen(f"{SOURCE}win64/{ARCHIVE}", timeout=60) as response:
                data = response.read(90_000_000)
            if hashlib.sha256(data).hexdigest() != SHA256:
                raise ValueError("VideoLAN archive checksum mismatch")
            archive.write_bytes(data)
        elif hashlib.sha256(Path(archive).read_bytes()).hexdigest() != SHA256:
            raise ValueError("VideoLAN archive checksum mismatch")
        prefix = f"vlc-{VERSION}/"
        with ZipFile(archive) as package:
            entries = {name: f"{prefix}{name}" for name in BINARIES}
            for entry in package.namelist():
                if not entry.startswith(f"{prefix}plugins/"):
                    continue
                name = entry[len(prefix):]
                parts = name.split("/")
                if name == "plugins/plugins.dat" or (
                    len(parts) == 3 and re.fullmatch(r"[a-z0-9_]+", parts[1])
                    and PLUGIN.fullmatch(parts[2])
                ):
                    entries[name] = entry
            if "plugins/access/libhttp_plugin.dll" not in entries:
                raise ValueError("VideoLAN HTTP playback plugin missing")
            for name, entry in entries.items():
                path = destination / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(package.read(entry))
            (destination / COPYING).write_bytes(package.read(f"{prefix}COPYING.txt"))
            (destination / NOTICES).write_text(
                f"VideoLAN VLC {VERSION} x64 runtime and upstream modules.\n"
                "Copyright the VideoLAN project and VLC authors. Individual modules retain their upstream licenses.\n"
                f"Upstream binary: {SOURCE}win64/{ARCHIVE}\n"
                f"Corresponding upstream source: {SOURCE}vlc-{VERSION}.tar.xz\n"
                "See VLC-COPYING.txt and the upstream source for component license notices.\n",
                encoding="utf-8",
            )
    return sorted(entries) + [COPYING, NOTICES]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--archive", type=Path)
    args = parser.parse_args()
    print(f"Staged {len(prepare(args.directory, args.archive))} VLC runtime files.")
