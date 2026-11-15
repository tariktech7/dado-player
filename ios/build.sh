#!/usr/bin/env bash
# Build Dado Player for iOS: Rust static library -> Swift host -> .app -> .ipa.
#
# Sibling of probe/build_apk.sh and built on the same lessons:
#
#   * `set -euo pipefail` — a failed Rust build must not leave a stale .a behind
#     that the Swift link step happily picks up, producing a "successful" build
#     of last week's code.
#   * Preflight says WHY it cannot run instead of failing deep inside a linker
#     with an unreadable error.
#   * Verify the ABI after building. A signature mismatch between
#     ios/Sources/DadoHost/include/DadoBridge.h and src/platform/ios/bridge.rs is
#     not a compile error on either side — it is a crash at runtime.
#
# USAGE
#   bash ios/build.sh            # Rust static lib + ABI check only
#   bash ios/build.sh --ipa      # ... then build Dado-Player-unsigned.ipa
#   bash ios/build.sh --deploy   # ... then `xtool dev` (sign + install)
#   bash ios/build.sh --sim      # simulator slice instead of device
#
# REQUIREMENTS
#   * Apple iOS SDK. Either xtool's Darwin Swift SDK inside WSL (`xtool setup`),
#     or macOS with Xcode. Linking needs the sysroot; `cargo check` does not,
#     which is why CI can type-check without an SDK but cannot produce a binary.
#   * Rust targets: rustup target add aarch64-apple-ios aarch64-apple-ios-sim
#   * `swift` (6.4+) for --ipa, `xtool` for --deploy.

set -eu
set -o pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
python3 scripts/release_version.py --check

MODE="lib"
SIM=0
for arg in "$@"; do
    case "$arg" in
        --ipa) MODE="ipa" ;;
        --deploy) MODE="deploy" ;;
        --sim) SIM=1 ;;
        --sim-app) SIM=1; MODE=simapp ;;
        -h|--help) sed -n '2,28p' "$0"; exit 0 ;;
        *) echo "!! unknown argument: $arg" >&2; exit 2 ;;
    esac
done

if [ "$SIM" -eq 1 ] && [ "$MODE" = ipa -o "$MODE" = deploy ]; then
    echo "Simulator builds support --sim alone; IPA and deploy require a device target." >&2
    exit 2
fi

APPLE_SDK=iphoneos
if [ "$SIM" -eq 1 ]; then
    APPLE_SDK=iphonesimulator
    TARGET="aarch64-apple-ios-sim"
    SWIFT_TRIPLE="arm64-apple-ios15.0-simulator"
else
    TARGET="aarch64-apple-ios"
    SWIFT_TRIPLE="arm64-apple-ios15.0"
fi

LIB="${CARGO_TARGET_DIR:-target}/$TARGET/release/libdadoplayer.a"
VENDOR="ios/Vendor/libdadoplayer.a"
IPA="Dado-Player-unsigned.ipa"

# ---------------------------------------------------------------------------
# 1. Preflight
# ---------------------------------------------------------------------------
echo "==> preflight"

command -v cargo >/dev/null 2>&1 || { echo "!! cargo not in PATH" >&2; exit 1; }

if ! rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
    echo "!! Rust target $TARGET is not installed." >&2
    echo "   Run: rustup target add $TARGET" >&2
    exit 1
fi

# Two ways to reach the iOS SDK: xcrun (macOS / Xcode) or xtool's Darwin Swift
# SDK (Linux/WSL). Store it — the --ipa step needs the same path.
# Kept as an array, not a string: an unquoted $SWIFT_SDK_FLAG would be word-split
# by the shell, and Xcode's SDK path is long enough that a single stray space
# (Xcode installed under a path with spaces) would silently become a bogus flag.
SDK_PATH=""
SWIFT_SDK_ARGS=()
LINUX_SDK=0
if command -v xcrun >/dev/null 2>&1 && xcrun --sdk "$APPLE_SDK" --show-sdk-path >/dev/null 2>&1; then
    SDK_PATH="$(xcrun --sdk "$APPLE_SDK" --show-sdk-path)"
    SWIFT_SDK_ARGS=(--sdk "$SDK_PATH")
    echo "    Apple SDK: $SDK_PATH"
elif command -v swift >/dev/null 2>&1 && swift sdk list 2>/dev/null | grep -qi darwin; then
    LINUX_SDK=1
    SDK_TRIPLE=arm64-apple-ios
    [ "$SIM" -eq 0 ] || SDK_TRIPLE=arm64-apple-ios-simulator
    SWIFTPM_DIR="${XDG_CONFIG_HOME:+$XDG_CONFIG_HOME/swiftpm}"
    DARWIN_BUNDLE="${DADO_DARWIN_SDK:-${SWIFTPM_DIR:-$HOME/.swiftpm}/swift-sdks/darwin.artifactbundle}"
    # xtool registers platform-specific triples, not a single generic device SDK.
    SDK_PATH="$(python3 - "$DARWIN_BUNDLE" "$SDK_TRIPLE" <<'PY'
import json, pathlib, sys
bundle = pathlib.Path(sys.argv[1]).resolve()
metadata = json.loads((bundle / 'swift-sdk.json').read_text())
print((bundle / metadata['targetTriples'][sys.argv[2]]['sdkRootPath']).resolve())
PY
)"
    [ -d "$SDK_PATH" ] || { echo "!! Missing Darwin SDK: $SDK_PATH" >&2; exit 1; }
    SWIFT_BUILD_DIR="${DADO_SWIFT_BUILD_DIR:-$HOME/.cache/dadoplayer/swift-build}"
    SWIFT_SDK_ARGS=(--swift-sdk darwin --build-system native
        --scratch-path "$SWIFT_BUILD_DIR")
    export SDKROOT="$SDK_PATH"
    # Native C/Objective-C dependencies must target Apple, never Linux headers.
    export CC_aarch64_apple_ios="$(command -v clang)"
    export CFLAGS_aarch64_apple_ios="--target=$SWIFT_TRIPLE -isysroot $SDK_PATH"
    export AR_aarch64_apple_ios="$(command -v llvm-ar)"
    if [ "$SIM" -eq 1 ]; then
        export CC_aarch64_apple_ios_sim="$CC_aarch64_apple_ios"
        export CFLAGS_aarch64_apple_ios_sim="$CFLAGS_aarch64_apple_ios"
        export AR_aarch64_apple_ios_sim="$AR_aarch64_apple_ios"
    fi
    echo "    Apple SDK (xtool): $SDK_PATH"
else
    cat >&2 <<'EOF'
!! No Apple iOS SDK found — cannot produce an .app or .ipa here.

   On a Mac with Xcode, verify: xcrun --sdk iphoneos --show-sdk-path
   For CI use .github/workflows/ios.yml with a macOS runner.
   The Rust target alone does not include the Apple device SDK.
EOF
    exit 1
fi

# ---------------------------------------------------------------------------
# 2. Rust static library
# ---------------------------------------------------------------------------
echo "==> cargo build ($TARGET, release, lib)"
# No `rm -f "$LIB"` here on purpose. It used to be there to guarantee that a
# failed compile cannot leave a stale .a behind for the Swift link to pick up,
# but `set -e` already does that: a failing cargo aborts the script before the
# `cp` below, so the copy is never reached. The `rm` actively hurt, because
# cargo does not notice a deleted final artifact and reports "Finished" without
# relinking — the second local run would then fail with "was not produced".
export IPHONEOS_DEPLOYMENT_TARGET=15.0
cargo rustc --locked --release --target "$TARGET" --lib --crate-type staticlib

if [ ! -f "$LIB" ]; then
    echo "!! $LIB was not produced" >&2
    echo "   Check that Cargo.toml still lists \"staticlib\" in [lib] crate-type." >&2
    exit 1
fi

mkdir -p ios/Vendor
cp "$LIB" "$VENDOR"
echo "    libdadoplayer.a: $(wc -c <"$VENDOR" | tr -d ' ') bytes"

# ---------------------------------------------------------------------------
# 3. ABI verification
# ---------------------------------------------------------------------------
echo "==> verify C ABI"

NM=""
for candidate in llvm-nm nm; do
    if command -v "$candidate" >/dev/null 2>&1; then NM="$candidate"; break; fi
done

if [ -z "$NM" ]; then
    echo "    (no nm found — skipping symbol check)"
else
    # The symbol TYPE LETTER is the only portable signal. Do not reach for
    # `-U`/`--defined-only`: on Apple's nm `-U` means "undefined symbols only",
    # on GNU nm `-U` means "defined symbols only" — the exact opposite. The
    # first version of this check used `nm -gU` and therefore asserted that the
    # Rust exports must be ABSENT, which can never pass.
    #
    # `T` = defined text, `U` = undefined (provided elsewhere).
    #
    # Captured into a variable instead of piped into `grep -q`: `grep -q` closes
    # stdin on the first match, nm then dies of SIGPIPE, and with `pipefail` the
    # whole pipeline reports failure even though the symbol was found.
    SYMBOLS="$("$NM" -g "$VENDOR" 2>/dev/null || true)"
    if [ -z "$SYMBOLS" ]; then
        echo "!! nm produced no symbols for $VENDOR" >&2
        exit 1
    fi

    # Rust exports: must be DEFINED in the static library.
    for sym in dado_app_start dado_app_set_storage_dir dado_app_set_env; do
        if ! grep -qE "[[:space:]]T[[:space:]]+_${sym}$" <<<"$SYMBOLS"; then
            echo "!! missing Rust export: $sym" >&2
            exit 1
        fi
    done

    # Swift exports: must be UNDEFINED here (provided by the Swift host, which
    # links against this archive).
    for sym in dado_host_attach dado_host_load dado_host_video_frame dado_host_options; do
        if grep -qE "[[:space:]]T[[:space:]]+_${sym}$" <<<"$SYMBOLS"; then
            echo "!! $sym is defined in the Rust library — it belongs to Swift" >&2
            exit 1
        fi
        if ! grep -qE "[[:space:]]U[[:space:]]+_${sym}$" <<<"$SYMBOLS"; then
            echo "!! missing Swift import in the Rust library: $sym" >&2
            exit 1
        fi
    done

    echo "    ABI symbols OK (3 defined, 4 undefined as expected)"
fi

# ---------------------------------------------------------------------------
# 4. Package an unsigned IPA
# ---------------------------------------------------------------------------
if [ "$MODE" = "ipa" ] || [ "$MODE" = "simapp" ]; then
    command -v swift >/dev/null 2>&1 || {
        echo "!! swift not in PATH — required for --ipa" >&2; exit 1; }

    # SwiftPM writes into .build/<triple>/release, where <triple> is the triple
    # string verbatim. Guessing that directory name breaks the moment the
    # deployment target changes, so ask SwiftPM instead of globbing.
    #
    # Two triple spellings are tried: the versioned form (arm64-apple-ios15.0)
    # is what the Darwin Swift SDK bundles use, the bare form is what a plain
    # Xcode toolchain accepts. Neither is universally valid across toolchain
    # versions, and this script cannot be exercised on a machine without an
    # Apple SDK, so it tries the canonical spelling first and falls back once —
    # loudly, so a silent double build does not go unnoticed.
    if [ "$LINUX_SDK" -eq 1 ]; then
        # SDKROOT would also affect SwiftPM's Linux-hosted manifest compiler.
        # The Darwin artifact supplies the sysroot to the device compiler only.
        unset SDKROOT
        TRIPLES=("$SDK_TRIPLE")
    elif [ "$SIM" -eq 1 ]; then
        TRIPLES=("$SWIFT_TRIPLE" "arm64-apple-ios-simulator")
    else
        TRIPLES=("$SWIFT_TRIPLE" "arm64-apple-ios")
    fi

    # Vendor/libdadoplayer.a is linked by an unsafe -l flag, so SwiftPM does
    # not know it is an input that changed. Clear only SwiftPM's build outputs
    # before linking; otherwise a Rust-only UI change can ship the old binary.
    SWIFT_CLEAN_ARGS=(--package-path ios)
    if [ "$LINUX_SDK" -eq 1 ]; then
        SWIFT_CLEAN_ARGS+=(--scratch-path "$SWIFT_BUILD_DIR")
    fi
    swift package "${SWIFT_CLEAN_ARGS[@]}" clean

    BIN=""
    for triple in "${TRIPLES[@]}"; do
        echo "==> swift build ($triple)"
        if ( cd ios && swift build -c release --triple "$triple" "${SWIFT_SDK_ARGS[@]}" ); then
            BIN_DIR="$( cd ios && swift build -c release --triple "$triple" \
                "${SWIFT_SDK_ARGS[@]}" --show-bin-path 2>/dev/null )"
            if [ -n "$BIN_DIR" ] && [ -f "$BIN_DIR/DadoPlayer" ]; then
                BIN="$BIN_DIR/DadoPlayer"
                echo "    executable: $BIN"
            else
                echo "!! $triple linked, but no DadoPlayer binary in '${BIN_DIR:-<no bin path>}'" >&2
            fi
            break
        fi
        echo "    $triple rejected by this toolchain — trying the next spelling" >&2
    done

    if [ -z "$BIN" ]; then
        echo "!! no usable iOS executable was produced" >&2
        echo "   Inspect the swift build output above; if the link failed on" >&2
        echo "   -ldadoplayer, check that ios/Vendor/libdadoplayer.a exists." >&2
        find ios/.build -maxdepth 4 -type f -perm -u+x 2>/dev/null | head -20 >&2
        exit 1
    fi

    STAGE="$(mktemp -d)"
    APP="$STAGE/Payload/DadoPlayer.app"
    mkdir -p "$APP"
    cp "$BIN" "$APP/DadoPlayer"
    cp ios/Info.plist "$APP/Info.plist"
    cp ios/Resources/*.png ios/Resources/Playback.mp4 "$APP/"
    if [ "$LINUX_SDK" -eq 0 ]; then
        xcrun ibtool --compile "$APP/LaunchScreen.storyboardc" ios/Resources/LaunchScreen.storyboard --target-device iphone --target-device ipad --minimum-deployment-target 15.0
        /usr/libexec/PlistBuddy -c "Add :DTPlatformName string $APPLE_SDK" "$APP/Info.plist"
        /usr/libexec/PlistBuddy -c "Add :DTSDKName string ${APPLE_SDK}$(xcrun --sdk "$APPLE_SDK" --show-sdk-version)" "$APP/Info.plist"
    else
        # iOS 14+ supports an Info.plist launch screen without compiled nibs.
        # Remove the storyboard reference: ibtool is a macOS-only executable.
        python3 - "$APP/Info.plist" "$SDK_PATH/SDKSettings.json" "$APPLE_SDK" <<'PY'
import json, pathlib, plistlib, sys
path = pathlib.Path(sys.argv[1])
info = plistlib.loads(path.read_bytes())
info.pop('UILaunchStoryboardName', None)
info['UILaunchScreen'] = {}
info['DTPlatformName'] = sys.argv[3]
settings = json.loads(pathlib.Path(sys.argv[2]).read_text())
info['DTSDKName'] = sys.argv[3] + settings['Version']
path.write_bytes(plistlib.dumps(info))
PY
    fi

    # The Rust core is a static library, so it is already inside the executable.
    # Any SwiftPM resource bundles would have to be copied here too; the host
    # currently declares none.
    chmod +x "$APP/DadoPlayer"

    if [ "$MODE" = simapp ]; then
        mkdir -p "$ROOT/target/simulator"
        cp -R "$APP" "$ROOT/target/simulator/"
        exit 0
    fi

    # An unsigned IPA is only installable through a sideloading tool that signs
    # it with the user's own Apple ID (Sideloadly, AltStore, Xcode). Ad-hoc
    # signing is possible when ldid/codesign is present and makes the artefact
    # closer to installable, so try it but do not require it.
    if command -v ldid >/dev/null 2>&1; then
        ldid -S "$APP/DadoPlayer" && echo "    ad-hoc signed (ldid)"
    elif command -v codesign >/dev/null 2>&1; then
        codesign -f -s - "$APP/DadoPlayer" 2>/dev/null && echo "    ad-hoc signed (codesign)"
    else
        echo "    not signed (no ldid/codesign) — expected for a sideload artefact"
    fi

    ( cd "$STAGE" && zip -qry packaged.ipa Payload && unzip -tq packaged.ipa )
    mkdir -p "$ROOT/Releases"
    # Copy across WSL/NTFS without attempting to preserve Linux ownership.
    # Rename on the destination filesystem only after the complete copy.
    cp "$STAGE/packaged.ipa" "$ROOT/Releases/.$IPA.tmp"
    mv "$ROOT/Releases/.$IPA.tmp" "$ROOT/Releases/$IPA"
    cp "$ROOT/Releases/$IPA" "$ROOT/$IPA"
    rm -rf "$STAGE"

    echo "==> $IPA"
    ls -la "$IPA"
    echo "    Install: sign with your Apple ID via Sideloadly/AltStore, then"
    echo "    trust the developer certificate under Settings > General > VPN & Device Management."
    exit 0
fi

# ---------------------------------------------------------------------------
# 5. Deploy via xtool
# ---------------------------------------------------------------------------
if [ "$MODE" = "deploy" ]; then
    echo "==> xtool dev (build, sign, install)"
    command -v xtool >/dev/null 2>&1 || {
        echo "!! xtool not found: https://github.com/xtool-org/xtool" >&2; exit 1; }
    ( cd ios && xtool dev )
else
    echo "==> done (library only)"
    echo "    Next: bash ios/build.sh --ipa      (unsigned .ipa)"
    echo "          bash ios/build.sh --deploy   (signed, on device)"
fi
