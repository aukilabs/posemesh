#!/usr/bin/env pwsh

Param(
    [Parameter(Position = 0)]
    [ArgumentCompleter({
        $PossibleValues = @('macOS', 'Mac-Catalyst', 'iOS', 'iOS-Simulator', 'Web', 'Linux')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Platform,

    [Parameter(Position = 1)]
    [ArgumentCompleter({
        $PossibleValues = @('AMD64', 'ARM64', 'WASM32')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Architecture,

    [Parameter(Position = 2)]
    [ArgumentCompleter({
        $PossibleValues = @('Debug', 'Release')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$BuildType
)

$ProtobufSource = Join-Path $PSScriptRoot "../protobuf"
$BuildDir = "../build-protobuf-$Platform-$Architecture-$BuildType"
$InstallDir = "../out-protobuf-$Platform-$Architecture-$BuildType"
$BuildPathRel = Join-Path $PSScriptRoot $BuildDir
$InstallPathRel = Join-Path $PSScriptRoot $InstallDir

if (!(Test-Path $BuildPathRel)) {
    New-Item -ItemType Directory -Path $BuildPathRel | Out-Null
}
if (!(Test-Path $InstallPathRel)) {
    New-Item -ItemType Directory -Path $InstallPathRel | Out-Null
}

$BuildPath = (Resolve-Path -Path $BuildPathRel).Path
$InstallPath = (Resolve-Path -Path $InstallPathRel).Path

Write-Host "BuildPath = $BuildPath"
Write-Host "InstallPath = $InstallPath"

$CMakeArgs = @(
    "-DCMAKE_BUILD_TYPE=$BuildType",
    "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
    "-Dprotobuf_BUILD_TESTS=OFF",
    "-Dprotobuf_BUILD_CONFORMANCE=OFF",
    "-Dprotobuf_BUILD_EXAMPLES=OFF",
    "-Dprotobuf_BUILD_PROTOBUF_BINARIES=ON",
    "-Dprotobuf_FORCE_FETCH_DEPENDENCIES=ON",
    "-Dprotobuf_BUILD_SHARED_LIBS=OFF",
    "-DCMAKE_INSTALL_PREFIX=$InstallDir",
    "-Dprotobuf_INSTALL=ON",
    "-DCMAKE_CXX_STANDARD=17"
)

# TODO: Fix platforms
$UseEmscripten = $False
switch ($Platform) {
    "macOS" {
        $Arch = $Architecture
        if ($Arch -like "amd64") {
            $Arch = "x86_64"
        }
        if ($Arch -like "arm64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
    }
    "Mac-Catalyst" {
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=iOS"
        $CMakeArgs += "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0"
    }
    "iOS" {
        $Arch = $Architecture
        if ($Arch -like "ARM64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=iOS"
    }
    "iOS-Simulator" {
        $Arch = $Architecture
        if ($Arch -like "amd64") {
            $Arch = "x86_64"
        }
        if ($Arch -like "arm64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=iOS"
        $CMakeArgs += "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0"
    }
    "Web" {
        $UseEmscripten = $True
    }
    "Linux" {
        $CMakeArgs += "-DCMAKE_SYSTEM_PROCESSOR=$Architecture"
    }
}

$sw = [Diagnostics.Stopwatch]::StartNew()

if (-not $UseEmscripten) {
    Push-Location $BuildPath

    cmake $ProtobufSource @CMakeArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "CMake configuration failed" -ForegroundColor Red
        Exit 1
    }

    cmake `
        --build . `
        --config $BuildType `
        --target protoc `
        --target libprotobuf `
        --target libprotobuf-lite `
        --target libupb `
        --target protoc-gen-upb `
        --target protoc-gen-upbdefs `
        --target scoped_set_env `
        --target poison `
        --target failure_signal_handler `
        --target flags_usage_internal `
        --target flags_usage `
        --target flags_parse `
        --target periodic_sampler `
        --target random_internal_distribution_test_util `
        --target cordz_sample_token `
        --target bad_any_cast_impl `
        --target log_flags `
        --parallel

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed" -ForegroundColor Red
        Exit 1
    }

    cmake --install .
}
Else {
    if (-not (Test-Path env:EMSDK)) {
        Write-Error "Emscripten environment not found. Run emsdk_env script first."
        exit 1
    }
    
    Write-Host "Configuring build with CMake..."
    emcmake cmake `
        -S $ProtobufSource `
        -B $BuildPath `
        -DCMAKE_BUILD_TYPE="$BuildType" `
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON `
        -Dprotobuf_BUILD_TESTS=OFF `
        -Dprotobuf_BUILD_CONFORMANCE=OFF `
        -Dprotobuf_BUILD_EXAMPLES=OFF `
        -Dprotobuf_BUILD_PROTOBUF_BINARIES=ON `
        -Dprotobuf_FORCE_FETCH_DEPENDENCIES=ON `
        -Dprotobuf_BUILD_SHARED_LIBS=OFF  `
        -DCMAKE_INSTALL_PREFIX="$InstallPath" `
        -DCMAKE_CXX_STANDARD=17
    
    emmake make -j -S $ProtobufSource -B $BuildPath
    
    emmake make install -j -C $BuildPath
    
    Write-Host "Build complete. Output files are in: $InstallPath"
}
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build install step failed" -ForegroundColor Red
    Exit 1
}

$sw.Stop()

Write-Host "Protobuf build completed successfully: $InstallPath`nBuild time: $sw`s" -ForegroundColor Green
Pop-Location