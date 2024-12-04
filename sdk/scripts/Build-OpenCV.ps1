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
    [String]$Architecture,

    [Parameter(Position = 2)]
    [ArgumentCompleter({
        $PossibleValues = @('Debug', 'Release')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$BuildType
)

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}

function ConvertTo-Framework {
    # Re-pack the .framework as a normal static library
    param (
        [String]$SourcePath,
        [String]$DestinationPath
    )

    $headersPath = Join-Path $SourcePath "Headers"
    $opencv2Path = Join-Path $SourcePath "opencv2"

    if (-Not (Test-Path $DestinationPath)) {
        New-Item -ItemType Directory -Path $DestinationPath | Out-Null
    }

    $opencv2Dest = Join-Path $DestinationPath "opencv2.a"
    Copy-Item -Path $opencv2Path -Destination $opencv2Dest -Recurse -Force

    $headersDest = Join-Path $DestinationPath "opencv2"
    if (-Not (Test-Path $headersDest)) {
        New-Item -ItemType Directory -Path $headersDest | Out-Null
    }
    Copy-Item -Path (Join-Path $headersPath "*") -Destination $headersDest -Recurse -Force
}

$UseCMakeDirectly = $False
$CMakeGenerator = $Null
$CMakeConfigureArgs = $Null
$UseBuildPythonScript = $False
$BuildPythonScriptFile = $Null
$BuildPythonScriptArgs = $Null
Switch($Platform) {
    'macOS' {
        If(-Not $IsMacOS) {
            Write-Error -Message "Your machine needs to be running macOS to build for 'macOS' platform."
            Exit 1
        }
        If(-Not $Architecture) {
            Write-Error -Message "Parameter '-Architecture' is not specified for 'macOS' platform."
            Exit 1
        }
        $UseBuildPythonScript = $True
        $BuildPythonScriptFile = 'opencv/platforms/osx/build_framework.py'
        $BuildPythonScriptArgs = @('--macos_archs')
        Switch($Architecture) {
            'AMD64' { $BuildPythonScriptArgs += 'x86_64' }
            'ARM64' { $BuildPythonScriptArgs += 'arm64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'macOS' platform."
                Exit 1
            }
        }
        $BuildPythonScriptArgs += '--build_only_specified_archs'
    }
    'Mac-Catalyst' {
        If(-Not $IsMacOS) {
            Write-Error -Message "Your machine needs to be running macOS to build for 'Mac-Catalyst' platform."
            Exit 1
        }
        If(-Not $Architecture) {
            Write-Error -Message "Parameter '-Architecture' is not specified for 'Mac-Catalyst' platform."
            Exit 1
        }
        $UseBuildPythonScript = $True
        $BuildPythonScriptFile = 'opencv/platforms/osx/build_framework.py'
        $BuildPythonScriptArgs = @('--catalyst_archs')
        Switch($Architecture) {
            'AMD64' { $BuildPythonScriptArgs += 'x86_64' }
            'ARM64' { $BuildPythonScriptArgs += 'arm64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Mac-Catalyst' platform."
                Exit 1
            }
        }
        $BuildPythonScriptArgs += '--build_only_specified_archs'
    }
    'iOS' {
        If(-Not $IsMacOS) {
            Write-Error -Message "Your machine needs to be running macOS to build for 'iOS' platform."
            Exit 1
        }
        If(-Not $Architecture) {
            $Architecture = 'ARM64'
            Write-Warning -Message "Using the implicit '$Architecture' architecture for 'iOS' platform."
        } ElseIf($Architecture -Ne 'ARM64') {
            Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'iOS' platform."
            Exit 1
        }
        $UseBuildPythonScript = $True
        $BuildPythonScriptFile = 'opencv/platforms/ios/build_framework.py'
        $BuildPythonScriptArgs = @('--iphoneos_archs', 'arm64', '--build_only_specified_archs')
    }
    'iOS-Simulator' {
        If(-Not $IsMacOS) {
            Write-Error -Message "Your machine needs to be running macOS to build for 'iOS-Simulator' platform."
            Exit 1
        }
        If(-Not $Architecture) {
            Write-Error -Message "Parameter '-Architecture' is not specified for 'iOS-Simulator' platform."
            Exit 1
        }
        $UseBuildPythonScript = $True
        $BuildPythonScriptFile = 'opencv/platforms/ios/build_framework.py'
        $BuildPythonScriptArgs = @('--iphonesimulator_archs')
        Switch($Architecture) {
            'AMD64' { $BuildPythonScriptArgs += 'x86_64' }
            'ARM64' { $BuildPythonScriptArgs += 'arm64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'iOS-Simulator' platform."
                Exit 1
            }
        }
        $BuildPythonScriptArgs += '--build_only_specified_archs'
    }
    'Web' {
        If(-Not $Architecture) {
            $Architecture = 'WASM32'
            Write-Warning -Message "Using the implicit '$Architecture' architecture for 'Web' platform."
        } ElseIf($Architecture -Ne 'WASM32') {
            Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Web' platform."
            Exit 1
        }
        # TODO
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$Platform' platform."
        Exit 1
    }
}
If($UseCMakeDirectly) {
    If($UseBuildPythonScript) {
        Write-Error -Message 'ASSERT: Both variables $UseCMakeDirectly and $UseBuildPythonScript cannot be set to $True simultaneously.'
        Exit 1
    }
    If($CMakeGenerator -Eq $Null) {
        Write-Error -Message 'ASSERT: Variable $CMakeGenerator is not set.'
        Exit 1
    }
    If($CMakeConfigureArgs -Eq $Null) {
        Write-Error -Message 'ASSERT: Variable $CMakeConfigureArgs is not set.'
        Exit 1
    }
} ElseIf($UseBuildPythonScript) {
    If($BuildPythonScriptFile -Eq $Null) {
        Write-Error -Message 'ASSERT: Variable $BuildPythonScriptFile is not set.'
        Exit 1
    }
    If($BuildPythonScriptArgs -Eq $Null) {
        Write-Error -Message 'ASSERT: Variable $BuildPythonScriptArgs is not set.'
        Exit 1
    }
} Else {
    Write-Error -Message 'ASSERT: Either variable $UseCMakeDirectly or $UseBuildPythonScript must be set to $True.'
    Exit 1
}

$CMakeCommand = (Get-Command -Name 'cmake') 2> $Null
If(-Not $CMakeCommand) {
    Write-Error -Message "Could not find 'cmake' command. Is CMake installed on your machine?"
    Exit 1
}

$XcodeBuildCommand = $Null
If(($UseCMakeDirectly -And ($CMakeGenerator -Eq 'Xcode')) -Or $UseBuildPythonScript) {
    $XcodeBuildCommand = (Get-Command -Name 'xcodebuild') 2> $Null
    If(-Not $XcodeBuildCommand) {
        Write-Error -Message "Could not find 'xcodebuild' command. Is Xcode installed on your machine?"
        Exit 1
    }
}

$Python3Command = $Null
If($UseBuildPythonScript) {
    $Python3Command = (Get-Command -Name 'python3') 2> $Null
    If(-Not $Python3Command) {
        Write-Error -Message "Could not find 'python3' command. Is Python installed on your machine?"
        Exit 1
    }
}

If(-Not $BuildType) {
    $BuildType = 'Release'
    Write-Warning -Message "Using the implicit '$BuildType' build type."
}
$CMakeBuildTypeFlagForConfiguring = $Null
$CMakeBuildTypeFlagForBuildingAndInstalling = $Null
$BuildPythonScriptBuildTypeFlag = $Null
Switch($BuildType) {
    'Debug' {
        $CMakeBuildTypeFlagForConfiguring = '-DCMAKE_BUILD_TYPE=Debug'
        If($UseCMakeDirectly -And ($CMakeGenerator -Eq 'Xcode')) {
            $CMakeBuildTypeFlagForBuildingAndInstalling = '--config Debug'
        } Else {
            $CMakeBuildTypeFlagForBuildingAndInstalling = ''
        }
        $BuildPythonScriptBuildTypeFlag = '--debug'
    }
    'Release' {
        $CMakeBuildTypeFlagForConfiguring = '-DCMAKE_BUILD_TYPE=Release'
        If($UseCMakeDirectly -And ($CMakeGenerator -Eq 'Xcode')) {
            $CMakeBuildTypeFlagForBuildingAndInstalling = '--config Release'
        } Else {
            $CMakeBuildTypeFlagForBuildingAndInstalling = ''
        }
        $BuildPythonScriptBuildTypeFlag = ''
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$BuildType' build type."
        Exit 1
    }
}
If($CMakeBuildTypeFlagForConfiguring -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $CMakeBuildTypeFlagForConfiguring is not set.'
    Exit 1
}
If($CMakeBuildTypeFlagForBuildingAndInstalling -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $CMakeBuildTypeFlagForBuildingAndInstalling is not set.'
    Exit 1
}
If($BuildPythonScriptBuildTypeFlag -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $BuildPythonScriptBuildTypeFlag is not set.'
    Exit 1
}

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/../../third-party" -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    $DirectoryNameSuffix = "OpenCV-$Platform-$Architecture-$BuildType"
    $BuildDirectoryName = "build-$DirectoryNameSuffix"
    $OutDirectoryName = "out-$DirectoryNameSuffix"
    If(-Not (Test-Path -Path $BuildDirectoryName -PathType Container)) {
        $NewItemResult = (New-Item -Path . -Name $BuildDirectoryName -ItemType Directory) 2> $Null
        If(-Not $NewItemResult) {
            Write-Error -Message "Failed to create '$BuildDirectoryName' directory."
            Exit 1
        }
    }
    If(-Not (Test-Path -Path $OutDirectoryName -PathType Container)) {
        $NewItemResult = (New-Item -Path . -Name $OutDirectoryName -ItemType Directory) 2> $Null
        If(-Not $NewItemResult) {
            Write-Error -Message "Failed to create '$OutDirectoryName' directory."
            Exit 1
        }
    }

    If($UseCMakeDirectly) {
        $CMakeGeneratorFlags = @()
        If($CMakeGenerator -Ne '') {
            $CMakeGeneratorFlags = @('-G', $CMakeGenerator)
        }
        & $CMakeCommand $CMakeGeneratorFlags $CMakeConfigureArgs @($CMakeBuildTypeFlagForConfiguring | Where-Object { $_ }) "-DCMAKE_INSTALL_PREFIX=$OutDirectoryName" -B $BuildDirectoryName -S opencv
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to configure CMake project.'
            Exit 1
        }
        & $CMakeCommand --build $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to build OpenCV library.'
            Exit 1
        }
        & $CMakeCommand --install $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to install CMake project.'
            Exit 1
        }
    } ElseIf($UseBuildPythonScript) {
        & $Python3Command -m venv "$BuildDirectoryName/.venv"
        $VirtualEnvBackup = $env:VIRTUAL_ENV
        $PathBackup = $env:PATH
        Try {
            $env:VIRTUAL_ENV = (Resolve-Path "$BuildDirectoryName/.venv").Path
            If($env:PATH) {
                $env:PATH = "$env:VIRTUAL_ENV/bin" + [System.IO.Path]::PathSeparator + $env:PATH
            } Else {
                $env:PATH = "$env:VIRTUAL_ENV/bin"
            }
            $Python3CommandFromVEnv = (Get-Command -Name 'python3') 2> $Null
            If((-Not $Python3CommandFromVEnv) -Or (-Not $Python3CommandFromVEnv.Source.StartsWith($env:VIRTUAL_ENV))) {
                Write-Error -Message "Could not find 'python3' command in Python virtual environment."
                Exit 1
            }
            & $Python3CommandFromVEnv $BuildPythonScriptFile $BuildDirectoryName $BuildPythonScriptArgs @($BuildPythonScriptBuildTypeFlag | Where-Object { $_ })
        } Finally {
            $env:VIRTUAL_ENV = $VirtualEnvBackup
            $env:PATH = $PathBackup
        }
    }
} Finally {
    Pop-Location
}
