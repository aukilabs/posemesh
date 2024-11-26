#!/usr/bin/env pwsh

Param(
    [Parameter(Position = 0)]
    [ArgumentCompleter({
        $PossibleValues = @('macOS', 'Mac-Catalyst', 'iOS', 'iOS-Simulator', 'Web')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Platform,

    [Parameter(Position = 1)]
    [ArgumentCompleter({
        $PossibleValues = @('AMD64', 'ARM64', 'WASM32')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Architecture
)

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}
If(-Not $Architecture) {
    Write-Error -Message "Parameter '-Architecture' is not specified."
    Exit 1
}

Push-Location -Path "sdk/"
Write-Host "Cloning OpenCV..."

# Ensure the "third-party" directory exists
if (-Not (Test-Path "third-party")) {
    New-Item -ItemType Directory -Path "third-party" | Out-Null
}
Push-Location -Path "third-party/"

# Clone the OpenCV repository
git clone git@github.com:opencv/opencv.git
Push-Location -Path "opencv/"

Write-Host "Building OpenCV..."
if ($Platform -eq "iOS") {
    # iOS building requires a virtual environment
    python3 -m venv .venv
    $env:VIRTUAL_ENV = (Resolve-Path ./.venv).Path
    $env:PATH = "$env:VIRTUAL_ENV/bin" + [System.IO.Path]::PathSeparator + $env:PATH
    python ../opencv/platforms/ios/build_framework.py ios --iphoneos_archs $Architecture --build_only_specified_archs
} ElseIf ($Platform -eq "macOS") {
    # Ensure the "build" directory exists
    if (-Not (Test-Path "build")) {
        New-Item -ItemType Directory -Path "build" | Out-Null
    }
    Push-Location -Path "build/"

    # Run cmake and build commands
    Write-Host "MacOS!"
    cmake .. -DBUILD_SHARED_LIBS=OFF -DBUILD_OPENEXR=OFF -DWITH_OPENEXR=OFF -DCMAKE_INSTALL_PREFIX="./install"

    cmake --build . --parallel 8
    make -j 8 install

    Pop-Location # build/
} ElseIf ($Platform -eq "web") {
    Write-Host "TODO!"
}

Write-Host "OpenCV build complete"

# Return to previous directories
Pop-Location # opencv/
Pop-Location # third-party/
Pop-Location # sdk/