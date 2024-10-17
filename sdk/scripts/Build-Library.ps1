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

$CMakeGenerator = $Null
$CMakeUseIOSToolchain = $False
$CMakeIOSToolchainPlatform = $Null
$CMakeUseEmscripten = $False
$CMakeToolchainFilePath = $Null
$XcodeBuildCommandArgs = $Null
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
        $CMakeGenerator = 'Xcode'
        $CMakeUseIOSToolchain = $True
        Switch($Architecture) {
            'AMD64' { $CMakeIOSToolchainPlatform = 'MAC' }
            'ARM64' { $CMakeIOSToolchainPlatform = 'MAC_ARM64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'macOS' platform."
                Exit 1
            }
        }
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
        $CMakeGenerator = 'Xcode'
        $CMakeUseIOSToolchain = $True
        $MacCatalystArchitectureFlagForXcodeBuild = $Null
        Switch($Architecture) {
            'AMD64' {
                $CMakeIOSToolchainPlatform = 'MAC_CATALYST'
                $MacCatalystArchitectureFlagForXcodeBuild = 'x86_64'
            }
            'ARM64' {
                $CMakeIOSToolchainPlatform = 'MAC_CATALYST_ARM64'
                $MacCatalystArchitectureFlagForXcodeBuild = 'arm64'
            }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Mac-Catalyst' platform."
                Exit 1
            }
        }
        If($MacCatalystArchitectureFlagForXcodeBuild -Eq $Null) {
            Write-Error -Message 'ASSERT: Variable $MacCatalystArchitectureFlagForXcodeBuild is not set.'
            Exit 1
        }
        $XcodeBuildCommandArgs = @('-configuration', $BuildType, "ARCHS=$MacCatalystArchitectureFlagForXcodeBuild", '-sdk', 'macosx', 'SUPPORTS_MACCATALYST=YES', '-project', "$PSScriptRoot/../build-$Platform-$Architecture-$BuildType/Posemesh.xcodeproj", '-scheme', 'Posemesh', '-destination', "platform=macOS,arch=$MacCatalystArchitectureFlagForXcodeBuild,variant=Mac Catalyst")
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
        $CMakeGenerator = 'Xcode'
        $CMakeUseIOSToolchain = $True
        $CMakeIOSToolchainPlatform = 'OS64'
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
        $CMakeGenerator = 'Xcode'
        $CMakeUseIOSToolchain = $True
        Switch($Architecture) {
            'AMD64' { $CMakeIOSToolchainPlatform = 'SIMULATOR64' }
            'ARM64' { $CMakeIOSToolchainPlatform = 'SIMULATORARM64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'iOS-Simulator' platform."
                Exit 1
            }
        }
    }
    'Web' {
        If(-Not $Architecture) {
            $Architecture = 'WASM32'
            Write-Warning -Message "Using the implicit '$Architecture' architecture for 'Web' platform."
        } ElseIf($Architecture -Ne 'WASM32') {
            Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Web' platform."
            Exit 1
        }
        $CMakeGenerator = ''
        $CMakeUseEmscripten = $True
        $CMakeToolchainFilePath = ''
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$Platform' platform."
        Exit 1
    }
}
If($CMakeGenerator -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $CMakeGenerator is not set.'
    Exit 1
}
If($CMakeUseIOSToolchain) {
    If($CMakeIOSToolchainPlatform -Eq $Null) {
        Write-Error -Message 'ASSERT: Variable $CMakeIOSToolchainPlatform is not set.'
        Exit 1
    }
    If($CMakeUseEmscripten) {
        Write-Error -Message 'ASSERT: Both variables $CMakeUseIOSToolchain and $CMakeUseEmscripten cannot be set to $True simultaneously.'
        Exit 1
    }
    $CMakeToolchainFilePath = '../third-party/ios-cmake/ios.toolchain.cmake'
} ElseIf($CMakeToolchainFilePath -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $CMakeToolchainFilePath is not set.'
    Exit 1
}

If(-Not $BuildType) {
    $BuildType = 'Release'
    Write-Warning -Message "Using the implicit '$BuildType' build type."
}
$CMakeBuildTypeFlagForConfiguring = $Null
$CMakeBuildTypeFlagForBuildingAndInstalling = $Null
Switch($BuildType) {
    'Debug' {
        $CMakeBuildTypeFlagForConfiguring = '-DCMAKE_BUILD_TYPE=Debug'
        If($CMakeGenerator -Eq 'Xcode') {
            $CMakeBuildTypeFlagForBuildingAndInstalling = '--config Debug'
        } Else {
            $CMakeBuildTypeFlagForBuildingAndInstalling = ''
        }
    }
    'Release' {
        $CMakeBuildTypeFlagForConfiguring = '-DCMAKE_BUILD_TYPE=Release'
        If($CMakeGenerator -Eq 'Xcode') {
            $CMakeBuildTypeFlagForBuildingAndInstalling = '--config Release'
        } Else {
            $CMakeBuildTypeFlagForBuildingAndInstalling = ''
        }
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

$CMakeCommand = (Get-Command -Name 'cmake') 2> $Null
If(-Not $CMakeCommand) {
    Write-Error -Message "Could not find 'cmake' command. Is CMake installed on your machine?"
    Exit 1
}

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    $XcodeBuildCommand = $Null
    If(($CMakeGenerator -Eq 'Xcode') -Or ($XcodeBuildCommandArgs -Ne $Null)) {
        $XcodeBuildCommand = (Get-Command -Name 'xcodebuild') 2> $Null
        If(-Not $XcodeBuildCommand) {
            Write-Error -Message "Could not find 'xcodebuild' command. Is Xcode installed on your machine?"
            Exit 1
        }
    }
    $EmCMakeCommand = $Null
    If($CMakeUseEmscripten) {
        $EmCMakeCommand = (Get-Command -Name 'emcmake') 2> $Null
        If(-Not $EmCMakeCommand) {
            Write-Error -Message "Could not find 'emcmake' command. Is Emscripten installed on your machine?"
            Exit 1
        }
    }
    $EmMakeCommand = $Null
    If($CMakeUseEmscripten) {
        $EmMakeCommand = (Get-Command -Name 'emmake') 2> $Null
        If(-Not $EmMakeCommand) {
            Write-Error -Message "Could not find 'emmake' command. Is Emscripten installed on your machine?"
            Exit 1
        }
    }
    If(($CMakeToolchainFilePath -Ne '') -And (-Not (Test-Path -Path $CMakeToolchainFilePath -PathType Leaf))) {
        If($CMakeUseIOSToolchain) {
            Write-Error -Message "CMake iOS toolchain file '$CMakeToolchainFilePath' does not exist. Are the Git repository submodules cloned?"
        } Else {
            Write-Error -Message "CMake toolchain file '$CMakeToolchainFilePath' does not exist."
        }
        Exit 1
    }

    $DirectoryNameSuffix = "$Platform-$Architecture-$BuildType"
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

    $CMakeGeneratorFlags = @()
    If($CMakeGenerator -Ne '') {
        $CMakeGeneratorFlags = @('-G', $CMakeGenerator)
    }
    $CMakeToolchainFileFlags = @()
    If($CMakeToolchainFilePath -Ne '') {
        $CMakeToolchainFileFlags = @("-DCMAKE_TOOLCHAIN_FILE=$CMakeToolchainFilePath")
    }
    $CMakeIOSToolchainPlatformFlag = ''
    If($CMakeUseIOSToolchain) {
        $CMakeIOSToolchainPlatformFlag = "-DPLATFORM=$CMakeIOSToolchainPlatform"
    }
    $CMakeCommandPrefixForConfiguring = @()
    If($CMakeUseEmscripten) {
        If($EmCMakeCommand -Eq $Null) {
            Write-Error -Message 'ASSERT: Variable $EmCMakeCommand is not set.'
            Exit 1
        }
        $CMakeCommandPrefixForConfiguring = @($EmCMakeCommand)
    }
    & $CMakeCommandPrefixForConfiguring $CMakeCommand $CMakeGeneratorFlags $CMakeToolchainFileFlags @($CMakeIOSToolchainPlatformFlag | Where-Object { $_ }) @($CMakeBuildTypeFlagForConfiguring | Where-Object { $_ }) "-DCMAKE_INSTALL_PREFIX=$OutDirectoryName" -B $BuildDirectoryName -S .
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to configure CMake project.'
        Exit 1
    }
    If($XcodeBuildCommandArgs -Ne $Null) {
        If($XcodeBuildCommand -Eq $Null) {
            Write-Error -Message 'ASSERT: Variable $XcodeBuildCommand is not set.'
            Exit 1
        }
        & $XcodeBuildCommand $XcodeBuildCommandArgs
    } Else {
        $CMakeCommandPrefixForBuilding = @()
        If($CMakeUseEmscripten) {
            If($EmMakeCommand -Eq $Null) {
                Write-Error -Message 'ASSERT: Variable $EmMakeCommand is not set.'
                Exit 1
            }
            $CMakeCommandPrefixForBuilding = @($EmMakeCommand)
        }
        & $CMakeCommandPrefixForBuilding $CMakeCommand --build $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
    }
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to build Posemesh library.'
        Exit 1
    }
    & $CMakeCommand --install $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to install CMake project.'
        Exit 1
    }
} Finally {
    Pop-Location
}
