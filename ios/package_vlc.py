"""Embed the arm64 dynamic framework, including its original bundle identity."""
import pathlib
import shutil
import struct
import sys

def package(app):
    source = pathlib.Path(__file__).resolve().parent / "Vendor/MobileVLCKit.framework"
    destination = pathlib.Path(app) / "Frameworks/MobileVLCKit.framework"
    destination.mkdir(parents=True, exist_ok=True)
    data = (source / "MobileVLCKit").read_bytes()
    if data[:4] == b"\xca\xfe\xba\xbe":
        count = struct.unpack_from(">I", data, 4)[0]
        for i in range(count):
            cpu, _, offset, size, _ = struct.unpack_from(">IIIII", data, 8 + i * 20)
            if cpu == 0x0100000C:  # CPU_TYPE_ARM64, device or simulator selected at fetch.
                data = data[offset:offset + size]
                break
        else:
            raise RuntimeError("VLCKit has no arm64 slice")
    if data[:4] != b"\xcf\xfa\xed\xfe":
        raise RuntimeError("Expected an arm64 Mach-O framework")
    binary = destination / "MobileVLCKit"
    binary.write_bytes(data)
    binary.chmod(0o755)
    shutil.copyfile(source / "Info.plist", destination / "Info.plist")
    print("Embedded dynamic MobileVLCKit (arm64):", len(data), "bytes")

if __name__ == "__main__":
    package(sys.argv[1])
