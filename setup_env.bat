# Environment Setup Script for Android Development
@echo off

REM Set up environment for Dado Player Android development
REM Run this script before building or testing APKs

REM Check if we're in the right directory
if not exist "android\app\build.gradle" (
    echo ERROR: This script must be run from the project root directory
    exit /b 1
)

REM Setup Android SDK and JDK environment
REM SDK path (adjust if needed)
set SDK="C:\Users\Tarik\AppData\Local\Temp\android-build\sdkroot"

REM Setup JDK 17 (Temurin)
set JAVA_HOME="C:\Users\Tarik\AppData\Local\Temp\android-build\jdk\jdk-17.0.21+9"

REM Add required tools to PATH
set PATH="%JAVA_HOME%\bin;%SDK%\platform-tools;%PATH%"

REM Verify Java installation
echo Checking Java installation...
java -version
if errorlevel 1 (
    echo ERROR: Java 17 not found. Please install JDK 17
    exit /b 1
)

echo.
echo Java setup complete.

REM Verify ADB is available
echo Checking ADB...
adb version
if errorlevel 1 (
    echo ERROR: ADB not found. Please ensure Android SDK platform-tools is installed
    exit /b 1
)

echo.
echo ADB setup complete.

REM Check for connected devices
echo Checking for connected devices...
adb devices
if %ERRORLEVEL% neq 0 (
    echo WARNING: No devices connected. Emulator may need to be started
)

echo.
echo Environment setup complete!
echo You can now run:
echo   gradle build
exit /b 0