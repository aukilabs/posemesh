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

If (-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}
If (-Not $Architecture) {
    Write-Error -Message "Parameter '-Architecture' is not specified."
    Exit 1
}

function Build-iOS {
    # Python virtual environment required to build on a M1 Mac
    # https://github.com/opencv/opencv/issues/21926#issuecomment-1184684648
    python3 -m venv .venv
    $env:VIRTUAL_ENV = (Resolve-Path ./.venv).Path
    $env:PATH = "$env:VIRTUAL_ENV/bin" + [System.IO.Path]::PathSeparator + $env:PATH
    python ../opencv/platforms/ios/build_framework.py ios --iphoneos_archs $Architecture --build_only_specified_archs

    # Re-pack the framework as a normal static library
    $sourcePath = "../opencv/ios/opencv2.framework/Versions/A"
    $destinationPath = "../opencv/opencv-static-lib"
    $headersPath = Join-Path $sourcePath "Headers"
    $opencv2Path = Join-Path $sourcePath "opencv2"

    if (-Not (Test-Path $destinationPath)) {
        New-Item -ItemType Directory -Path $destinationPath | Out-Null
    }

    $opencv2Dest = Join-Path $destinationPath "opencv2.a"
    Copy-Item -Path $opencv2Path -Destination $opencv2Dest -Recurse -Force

    $headersDest = Join-Path $destinationPath "opencv2"
    if (-Not (Test-Path $headersDest)) {
        New-Item -ItemType Directory -Path $headersDest | Out-Null
    }
    Copy-Item -Path (Join-Path $headersPath "*") -Destination $headersDest -Recurse -Force
}

function Build-MacOS {
    if (-Not (Test-Path "build")) {
        New-Item -ItemType Directory -Path "build" | Out-Null
    }
    Push-Location -Path "build/"

    cmake .. -DBUILD_SHARED_LIBS=OFF -DBUILD_OPENEXR=OFF -DWITH_OPENEXR=OFF -DCMAKE_INSTALL_PREFIX="./install"
    cmake --build . --parallel 8
    make -j 8 install

    Pop-Location # build/
}

function Build-Web {
    Write-Host "TODO!"
}

Push-Location -Path "sdk/"

if (-Not (Test-Path "third-party")) {
    New-Item -ItemType Directory -Path "third-party" | Out-Null
}
Push-Location -Path "third-party/"

git clone git@github.com:opencv/opencv.git
Push-Location -Path "opencv/"

if ($Platform -eq "iOS") {
    Build-iOS
}
ElseIf ($Platform -eq "macOS") {
    Build-MacOS
}
ElseIf ($Platform -eq "web") {
    Build-Web
}

Pop-Location # opencv/
Pop-Location # third-party/
Pop-Location # sdk/