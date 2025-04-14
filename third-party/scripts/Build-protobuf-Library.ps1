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
    "-Dprotobuf_BUILD_PROTOC_BINARIES=OFF",
    "-Dprotobuf_BUILD_LIBPROTOC=OFF",
    "-Dprotobuf_FORCE_FETCH_DEPENDENCIES=ON",
    "-Dprotobuf_BUILD_SHARED_LIBS=OFF",
    "-DCMAKE_INSTALL_PREFIX=$InstallPath",
    "-Dprotobuf_INSTALL=ON",
    "-DCMAKE_CXX_STANDARD=17"
)

$Arch = $Architecture
$UseEmscripten = $False
switch ($Platform) {
    "macOS" {
        if ($Arch -like "amd64") {
            $Arch = "x86_64"
        }
        if ($Arch -like "arm64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
    }
    "Mac-Catalyst" {
        if ($Arch -like "amd64") {
            $Arch = "x86_64"
            $CXXFlags = "-target x86_64-apple-ios-macabi"
        }
        if ($Arch -like "arm64") {
            $Arch = "arm64"
            $CXXFlags = "-target arm64-apple-ios-macabi"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=Darwin"
        $CMakeArgs += "-DCMAKE_OSX_SYSROOT=macosx"
        $CMakeArgs += "-DCMAKE_CXX_FLAGS=$CXXFlags"
        $CMakeArgs += "-DCMAKE_C_FLAGS=$CXXFlags"
        $CMakeArgs += "-DCMAKE_EXE_LINKER_FLAGS=$CXXFlags"
    }
    "iOS" {
        if ($Arch -like "ARM64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=iOS"
    }
    "iOS-Simulator" {
        if ($Arch -like "amd64") {
            $Arch = "x86_64"
        }
        if ($Arch -like "arm64") {
            $Arch = "arm64"
        }
        $CMakeArgs += "-DCMAKE_OSX_ARCHITECTURES=$Arch"
        $CMakeArgs += "-DCMAKE_SYSTEM_NAME=iOS"
        $CMakeArgs += "-DCMAKE_OSX_SYSROOT=iphonesimulator"
        $CMakeArgs += "-DCMAKE_OSX_DEPLOYMENT_TARGET=13.0"
    }
    "Web" {
        $UseEmscripten = $True
    }
    "Linux" {
        if ($Arch -like "arm64") {
            $Arch = "arm64"
            $CMakeArgs += "-DCMAKE_SYSTEM_NAME=Linux"
            $CMakeArgs += "-DCMAKE_SYSTEM_PROCESSOR=aarch64"
            $CMakeArgs += "-DCMAKE_C_COMPILER=aarch64-linux-gnu-gcc"
            $CMakeArgs += "-DCMAKE_CXX_COMPILER=aarch64-linux-gnu-g++"
            $CMakeArgs += "-DCMAKE_LINKER=aarch64-linux-gnu-ld"
        }
    }
}

$sw = [Diagnostics.Stopwatch]::StartNew()

Push-Location $BuildPath

if (-not $UseEmscripten) {
    cmake $ProtobufSource @CMakeArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "CMake configuration failed"
        Exit 1
    }

    cmake `
        --build . `
        --config $BuildType `
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

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed"
        Exit 1
    }

    cmake --install .
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build install step failed"
        Exit 1
    }
}
Else {
    if (-not (Test-Path $env:EMSDK)) {
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
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Emscripten CMake configuration failed"
        Exit 1
    }

    emmake make -j -S $ProtobufSource -B $BuildPath
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Emscripten Build failed"
        Exit 1
    }

    emmake make install -j -C $BuildPath
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Emscripten Build install step failed"
        Exit 1
    }
}

$sw.Stop()

Write-Host "Protobuf build completed successfully: $InstallPath`nBuild time: $sw`s" -ForegroundColor Green
Pop-Location
