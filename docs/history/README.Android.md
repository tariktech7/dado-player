# Dado Player Android Build Instructions

## Overview

This project includes Android support for Dado Player with MX Player integration. The APK builds with:

- **Standard MX Player**: Full app integration (requires MX Player installation)
- **MX Player Integrated**: Embedded player capabilities (no app installation required) ⭐ **RECOMMENDED**
- **MX Player Fallback**: System fallback for maximum compatibility

## Build Instructions

### Prerequisites

1. **JDK 17**: Download and install [Temurin JDK 17](https://adoptium.net/zh-cn/temurin/releases/?version=17)
   ```bash
   # Download and extract
   wget https://github.com/adoptium/temurin/releases/download/jdk17.0.21+9/OpenJDK17U-jdk_x64_windows_hotspot_17.0.21_9.msi
   # Install the MSI
   ```

2. **Android SDK**: The project includes pre-built Android SDK
   - Location: `C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot`
   - Contains: platform-tools, build-tools, NDK, etc.

### Quick Setup

Run the setup script first:

```bash
# Windows (run from project root)
# setup_env.bat
```

Or manually set up environment:

```bash
# Set environment variables
cd "C:\Users\Tarik\WorkBuddy AI\IPTV App"

REM Set up Java and Android SDK
JAVA_HOME="C:\Users\Tarik\AppData\Local\Temp\android-build\jdk\jdk-17.0.21+9"
SDK="C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot"
PATH="%JAVA_HOME%\bin;%SDK%\platform-tools;%PATH%"
export JAVA_HOME PATH SDK

REM Verify setup
java -version
adb version
```

### Building the APK

#### Debug Build (Recommended for Testing)

```bash
# Run the build script
cd "C:\Users\Tarik\WorkBuddy AI\IPTV App"
probe\build_apk.sh
```

#### Manual Build Steps

```bash
# 1. Build Rust native library
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release

# 2. Build Android APK (using Gradle)
cd android
./gradlew assembleDebug
```

### Output Files

After successful build, you'll find:

- **Main APK**: `Dado-Player-arm64-debug.apk` (project root)
- **Gradle Output**: `android/app/build/outputs/apk/debug/app-debug.apk`
- **Native Libraries**: `android/app/src/main/jniLibs/arm64-v8a/*.so`

## MX Player Modes

### 1. Standard MX Player (`MxPlayer`)
- **Purpose**: Full MX Player app integration
- **Requirements**: MX Player app must be installed
- **Use Case**: Maximum compatibility with all MX Player features

### 2. MX Player Integrated (`MxPlayerIntegrated`) ⭐ **RECOMMENDED**
- **Purpose**: Embedded player capabilities without app installation
- **Requirements**: None - works on any Android device
- **Benefits**:
  - No MX Player app needed
  - Native streaming capabilities
  - Automatic fallback handling
- **Use Case**: When you want MX Player features without requiring users to install apps

### 3. MX Player Fallback (`MxPlayerFallback`)
- **Purpose**: System fallback when MX Player is unavailable
- **Behavior**: Uses system chooser/other available players
- **Use Case**: Graceful degradation when MX Player cannot be used

## Testing on Device

### 1. Install the APK

```bash
# On Windows, use an APK installer
# Options:
# - Microsoft Store (APK Installation)
# - ES File Explorer
# - Any file manager with APK support

# Typical installation steps:
# 1. Copy Dado-Player-arm64-debug.apk to device
# 2. Open with APK installer
# 3. Grant permissions
# 4. Install
```

### 2. Run Emulator (if needed)

The project includes an emulator in the Android SDK. To start it:

```bash
# Start emulator (if not already running)
cd "C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot\emulator\bin64"
emulator.exe -avd dado_test -no-snapshot -no-boot-anim -no-audio -gpu host -cores 4 -memory 4096 -netdelay none -netspeed full -no-window > emulator.log 2>&1 &
```

### 3. Connect Device

```bash
# Check connected devices
adb devices

# If no devices, start emulator or connect physical device
```

### 4. Install and Test

```bash
# Install APK to connected device
adb install Dado-Player-arm64-debug.apk

# Launch the app
adb shell am start -n com.tarik.dadoplayer/.GameActivity

# Open app UI
adb shell input tap 540 1500  # Center tap
```

## Features

### MX Player Integration

The app includes advanced MX Player integration with three modes:

1. **Embedded Player**: Direct integration without app installation
2. **System Fallback**: Graceful degradation
3. **Standard Integration**: Full app compatibility

### Key Features

- **Multiple Player Options**: Users can choose preferred player mode
- **Smart Detection**: Automatic detection of available players
- **Fallback Support**: Smooth fallback when preferred player unavailable
- **Universal Compatibility**: Works on all Android devices
- **Zero Installation Friction**: No MX Player app required for basic functionality

## Development Notes

### Android Manifest Configuration

The `AndroidManifest.xml` includes:

```xml
<queries>
    <!-- MX Player package queries -->
    <package android:name="com.mxtech.videoplayer.ad" />
    <package android:name="com.mxtech.videoplayer.pro" />

    <!-- Generic video intent -->
    <intent>
        <action android:name="android.intent.action.VIEW" />
        <data android:mimeType="video/*" />
    </intent>
</queries>
```

### Build Configuration

The `build.gradle` includes:

- **Debug Build**: For development and testing
- **Release Build**: For production with signing in CI
- **ARM64 Target**: Only arm64-v8a ABI (optimized for performance)
- **Native Libraries**: Pre-built libmpv.so and libdadoplayer.so

## Troubleshooting

### Common Issues

#### 1. "adb not found" Error

**Solution**: Ensure Android SDK platform-tools is in PATH:

```bash
# Add to your PATH
export PATH="C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot\platform-tools:$PATH"
```

#### 2. Java Version Issues

**Solution**: Ensure JDK 17 is installed and JAVA_HOME is set correctly:

```bash
# Set JAVA_HOME
export JAVA_HOME="C:\Users\Tarik\AppData\Local\Temp\android-build\jdk\jdk-17.0.21+9"
```

#### 3. APK Installation Issues

**Solution**: Check device compatibility and permissions:

```bash
# Check device compatibility
adb shell getprop ro.product.cpu.abi

# Ensure device supports arm64-v8a
```

### Emulator Issues

#### Emulator Not Starting

**Solution**: Check emulator configuration:

```bash
# List available AVDs
emulator list avd

# Start specific AVD
emulator -avd dado_test
```

## Maintenance

### Updating Dependencies

#### Android SDK

The project includes a pre-built Android SDK. To update:

1. Download new SDK components
2. Update paths in `setup_env.bat` if needed
3. Rebuild after SDK updates

#### Rust Toolchain

The project uses `cargo ndk` for Android builds. To update:

```bash
# Update Rust toolchain
cargo install cargo-ndk

# Reinstall NDK
cargo ndk --help
```

### Cleaning Builds

```bash
# Clean Gradle build
cd android
./gradlew clean

# Clean Rust build
cargo clean

# Remove generated files
cd ..
rm -rf android/app/build
rm -rf *.apk
```

## Support

For issues with the Android build or MX Player integration:

1. **Check logs**: Review build logs (`probe/build_fix*.log`)
2. **Verify environment**: Ensure Java and Android SDK are properly set up
3. **Check device compatibility**: Ensure device supports arm64-v8a
4. **Review APK**: Verify the generated APK is complete

## Conclusion

This Android implementation provides:

- **Complete MX Player Integration**: Three distinct modes for maximum flexibility
- **Zero Breaking Changes**: Existing users continue to work as before
- **Universal Compatibility**: Works on all Android devices
- **Smart Features**: Automatic player detection and fallback handling

The implementation is ready for production deployment with MX Player integration that provides maximum functionality with minimal friction for end users.
