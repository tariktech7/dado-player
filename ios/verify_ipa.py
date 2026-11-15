"""Check the app's dyld contract before publishing an unsigned sideload IPA."""
import plistlib
import struct
import sys
import zipfile

def commands(binary):
    magic, cpu, _, kind, count, _, _, _ = struct.unpack_from("<8I", binary)
    assert magic == 0xFEEDFACF and cpu == 0x0100000C, "Expected thin arm64 Mach-O"
    offset = 32
    result = []
    for _ in range(count):
        command, size = struct.unpack_from("<II", binary, offset)
        assert size >= 8 and offset + size <= len(binary), "Invalid Mach-O load command"
        if command in (0xC, 0xD, 0x8000001C):
            string_offset = struct.unpack_from("<I", binary, offset + 8)[0]
            value = binary[offset + string_offset:offset + size].split(b"\0")[0].decode()
            result.append((command, value))
        offset += size
    return kind, result

def verify(path):
    root = "Payload/DadoPlayer.app/"
    with zipfile.ZipFile(path) as package:
        assert package.testzip() is None
        info = plistlib.loads(package.read(root + "Info.plist"))
        assert info["CFBundleIdentifier"] == "com.tarik.dadoplayer"
        _, loads = commands(package.read(root + "DadoPlayer"))
        framework = "@rpath/MobileVLCKit.framework/MobileVLCKit"
        assert (0xC, framework) in loads, "VLC not linked"
        assert (0x8000001C, "@executable_path/Frameworks") in loads, "Missing framework search path"
        kind, loads = commands(package.read(root + "Frameworks/MobileVLCKit.framework/MobileVLCKit"))
        assert kind == 6 and (0xD, framework) in loads, "Wrong embedded dynamic library"
        framework_info = plistlib.loads(package.read(root + "Frameworks/MobileVLCKit.framework/Info.plist"))
        assert framework_info["CFBundleExecutable"] == "MobileVLCKit"
        assert "GNU LESSER GENERAL PUBLIC LICENSE" in package.read(root + "VLCKit-COPYING.txt").decode()
        assert "MobileVLCKit 3.7.3" in package.read(root + "ThirdPartyNotices.txt").decode()
        print("IPA verified: version", info["CFBundleShortVersionString"], "build", info["CFBundleVersion"], "arm64 + embedded VLC + rpath + license")

if __name__ == "__main__":
    verify(sys.argv[1])
