"""Regression checks for the Windows 11 DLLs that broke Windows 10 startup."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import build_portable as pack


class RuntimePolicyTests(unittest.TestCase):
    def test_os_dlls_cannot_enter_the_package(self):
        for name in ('msvcp_win.dll', 'd3dcompiler_47.dll'):
            with self.subTest(name=name), patch.object(pack, 'FILES', pack.FILES + [name]):
                with self.assertRaisesRegex(RuntimeError, 'OS DLLs'):
                    pack.validate_runtime()

    def test_architecture_mismatch_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            data = bytearray(256)
            data[:2] = b'MZ'
            data[0x3c:0x40] = (128).to_bytes(4, 'little')
            data[128:132] = b'PE\0\0'
            data[132:134] = (0x14c).to_bytes(2, 'little')  # x86 in x64 package
            Path(directory, 'wrong.dll').write_bytes(data)
            with patch.object(pack, 'DIST', directory), patch.object(pack, 'FILES', ['wrong.dll']):
                with self.assertRaisesRegex(RuntimeError, 'x64'):
                    pack.validate_runtime()


if __name__ == '__main__':
    unittest.main()
