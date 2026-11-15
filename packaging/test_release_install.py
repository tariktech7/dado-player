"""The public Windows filename must always contain the incoming package."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import organize_releases as releases


class WindowsReleaseInstallTests(unittest.TestCase):
    def test_replaces_canonical_download_and_preserves_previous_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "Releases"
            target.mkdir()
            (target / "Dado-Player.exe").write_bytes(b"previous-working-package")
            package = root / "incoming.exe"
            package.write_bytes(b"new-package")
            with patch.object(releases, "RELEASES", target):
                releases.install_windows(package)
                releases.install_windows(package)  # Repeat without clobbering backup.
            self.assertEqual((target / "Dado-Player.exe").read_bytes(), b"new-package")
            backups = list((target / "Archiv").rglob("*.exe"))
            self.assertEqual(len(backups), 1)
            self.assertEqual(backups[0].read_bytes(), b"previous-working-package")
            self.assertEqual([p.name for p in target.glob("*.exe")], ["Dado-Player.exe"])

    def test_missing_build_does_not_touch_current_download(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory)
            current = target / "Dado-Player.exe"
            current.write_bytes(b"working")
            with patch.object(releases, "RELEASES", target), self.assertRaises(RuntimeError):
                releases.install_windows(target / "missing.exe")
            self.assertEqual(current.read_bytes(), b"working")


if __name__ == "__main__":
    unittest.main()
