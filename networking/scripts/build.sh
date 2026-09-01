#!/bin/bash

# Default values for optional arguments
BuildType="Release"
InstallToolchain="false"
Feature="cpp"
BuildExamples=""

# Function to display usage information
usage() {
    echo "Build networking module"
    echo ""
    echo "Usage: $0 --platform <platform> --architecture <architecture> [--feature <feature>] [--build-type <type>] [--install-toolchains]"
    echo ""
    echo "Options:"
    echo "  --platform, -p            Target Platform (required; macOS, Mac-Catalyst, iOS, iOS-Simulator, browser, linux)"
    echo "  --architecture, -a        Target Architecture (required; amd64, arm64, wasm32)"
    echo "  --feature, -f             Build feature (default: cpp; cpp, wasm, rust)"
    echo "  --examples, -e            Build examples"
    echo "  --build-type, -b          Build type (default: Release; Debug, Release)"
    echo "  --install-toolchains, -i  Install toolchains (default: false)"
    exit 1
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --platform|-p)
            Platform="$2"
            shift 2
            ;;
        --architecture|-a)
            Architecture="$2"
            shift 2
            ;;
        --feature|-f)
            Feature="$2"
            shift 2
            ;;
        --build-type|-b)
            BuildType="$2"
            shift 2
            ;;
        --examples|-e)
            BuildExamples="--examples"
            shift 1
            ;;
        --install-toolchains|-i)
            InstallToolchain="true"
            shift 1
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Check required arguments
if [[ -z "$Platform" || -z "$Architecture" ]]; then
    echo "Error: --platform and --architecture are required arguments."
    usage
fi

RustToolchain=""
RustTarget=""
WASMTarget=""

# Platform detection
case "$Platform" in
    "macOS")
        if [[ "$(uname)" != "Darwin" ]]; then
            echo "Error: Your machine needs to be running macOS to build for 'macOS' Platform." >&2
            exit 1
        fi
        if [[ -z "$Architecture" ]]; then
            echo "Error: Parameter '--architecture' or '-a' is not specified for 'macOS' Platform." >&2
            exit 1
        fi
        RustToolchain="1.81.0"
        case "$Architecture" in
            "amd64") RustTarget="x86_64-apple-darwin" ;;
            "arm64") RustTarget="aarch64-apple-darwin" ;;
            *) echo "Error: Invalid or unsupported '$Architecture' Architecture for 'macOS' Platform." >&2; exit 1 ;;
        esac
        ;;
    "Mac-Catalyst")
        if [[ "$(uname)" != "Darwin" ]]; then
            echo "Error: Your machine needs to be running macOS to build for 'Mac-Catalyst' Platform." >&2
            exit 1
        fi
        if [[ -z "$Architecture" ]]; then
            echo "Error: Parameter '--architecture' is not specified for 'Mac-Catalyst' Platform." >&2
            exit 1
        fi
        RustToolchain="nightly-2024-10-06"
        case "$Architecture" in
            "amd64") RustTarget="x86_64-apple-ios-macabi" ;;
            "arm64") RustTarget="aarch64-apple-ios-macabi" ;;
            *) echo "Error: Invalid or unsupported '$Architecture' Architecture for 'Mac-Catalyst' Platform." >&2; exit 1 ;;
        esac
        Feature="cpp"
        ;;
    "iOS")
        if [[ "$(uname)" != "Darwin" ]]; then
            echo "Error: Your machine needs to be running macOS to build for 'iOS' Platform." >&2
            exit 1
        fi
        Architecture="${Architecture:-arm64}"
        echo "Warning: Using the implicit '$Architecture' Architecture for 'iOS' Platform."
        if [[ "$Architecture" != "arm64" ]]; then
            echo "Error: Invalid or unsupported '$Architecture' Architecture for 'iOS' Platform." >&2
            exit 1
        fi
        RustToolchain="1.81.0"
        RustTarget="aarch64-apple-ios"
        Feature="cpp"
        ;;
    "browser")
        Architecture="${Architecture:-wasm32}"
        echo "Warning: Using the implicit '$Architecture' Architecture for 'browser' Platform."
        if [[ "$Architecture" != "wasm32" ]]; then
            echo "Error: Invalid or unsupported '$Architecture' Architecture for 'browser' Platform." >&2
            exit 1
        fi
        RustToolchain="1.81.0"
        RustTarget="wasm32-unknown-unknown"
        WASMTarget="no-modules"
        Feature="wasm"
        ;;
    "linux")
        if [[ "$(uname)" != "Linux" ]]; then
            echo "Error: Your machine needs to be running Linux to build for 'linux' Platform." >&2
            exit 1
        fi
        if [[ -z "$Architecture" ]]; then
            echo "Error: Parameter '--architecture' is not specified for 'linux' Platform." >&2
            exit 1
        fi
        RustToolchain="1.81.0"
        case "$Architecture" in
            "amd64") RustTarget="x86_64-unknown-linux-gnu" ;;
            "arm64") RustTarget="aarch64-unknown-linux-gnu" ;;
            *) echo "Error: Invalid or unsupported '$Architecture' Architecture for 'linux' Platform." >&2; exit 1 ;;
        esac
        ;;
    *)
        echo "Error: Invalid or unsupported '$Platform' Platform." >&2
        exit 1
        ;;
esac

# Determine build flags based on BuildType
RustBuildTypeDirName=""
RustBuildTypeFlag=""
WASMBuildTypeFlag=""
case "$BuildType" in
    "Debug")
        RustBuildTypeDirName="debug"
        RustBuildTypeFlag=""
        WASMBuildTypeFlag="--dev"
        ;;
    "Release")
        RustBuildTypeDirName="release"
        RustBuildTypeFlag="--release"
        WASMBuildTypeFlag="--release"
        ;;
    *)
        echo "Error: Invalid or unsupported '$BuildType' build type." >&2
        exit 1
        ;;
esac

# Display configuration
echo "Configuration:"
echo "  Platform:          $Platform"
echo "  Architecture:      $Architecture"
echo "  Build Type:        $BuildType"
echo "  Feature:           $Feature"
echo "  Build Examples:    $BuildExamples"
echo "  Rust Toolchain:    $RustToolchain"
echo "  Rust Target:       $RustTarget"
echo "  Rust Build Type:   $RustBuildTypeDirName"
echo "  Install Toolchains: $InstallToolchain"

# Check for Rust toolchain and cargo
if ! command -v rustup &> /dev/null; then
    echo "Error: 'rustup' command not found. Is Rust installed on your machine?" >&2
    exit 1
fi
if ! command -v cargo &> /dev/null; then
    echo "Error: 'cargo' command not found. Is Rust installed on your machine?" >&2
    exit 1
fi

# Check for wasm-pack if targeting wasm32
if [[ "$RustTarget" == "wasm32-unknown-unknown" ]]; then
    if ! command -v wasm-pack &> /dev/null; then
        echo "Error: 'wasm-pack' command not found. Is WASM-Pack installed on your machine?" >&2
        exit 1
    fi
fi

# Verify Rust toolchain installation
if ! rustup toolchain list | grep -q "$RustToolchain"; then
    if [[ "$InstallToolchain" != "true" ]]; then
        echo "Error: Rust toolchain '$RustToolchain' is required. Please run the script with '--install-toolchain' or '-i' flag." >&2
        exit 1
    fi
    rustup toolchain install "$RustToolchain" || { echo "Error: Failed to install Rust '$RustToolchain' toolchain." >&2; exit 1; }
fi

# Add target 
if ! rustup +$RustToolchain target list --installed | grep -q "$RustTarget"; then
    rustup +$RustToolchain target add "$RustTarget" || { echo "Error: Failed to install Rust target '$RustTarget'." >&2; exit 1; }
fi

# Build command
if [[ "$RustTarget" == "wasm32-unknown-unknown" ]]; then
    wasm-pack build --target "$WASMTarget" $WASMBuildTypeFlag --out-dir pkg/"$BuildType" --out-name PosemeshNetworking --features $Feature
else
    cargo +"$RustToolchain" build --target "$RustTarget" $RustBuildTypeFlag --features $Feature $BuildExamples
fi

if [[ $? -ne 0 ]]; then
    echo "Error: Failed to build Posemesh Networking library." >&2
    exit 1
fi

# Rename static library if necessary
if [[ "$RustTarget" != "wasm32-unknown-unknown" ]]; then
    StaticLibraryPathOriginal="target/$RustTarget/$RustBuildTypeDirName/libposemesh_networking.a"
    StaticLibraryPathRenamed="target/$RustTarget/$RustBuildTypeDirName/libposemesh_networking_static.a"
    if [[ -f "$StaticLibraryPathRenamed" ]]; then
        rm "$StaticLibraryPathRenamed" || { echo "Error: Failed to remove '$StaticLibraryPathRenamed'." >&2; exit 1; }
    fi
    if [[ -f "$StaticLibraryPathOriginal" ]]; then
        cp "$StaticLibraryPathOriginal" "$StaticLibraryPathRenamed" || { echo "Error: Failed to rename '$StaticLibraryPathOriginal'." >&2; exit 1; }
    else
        echo "Error: File '$StaticLibraryPathOriginal' does not exist." >&2
        exit 1
    fi
fi

echo "Build process completed."
