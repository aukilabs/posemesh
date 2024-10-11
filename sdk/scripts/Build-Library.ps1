#!/usr/bin/env pwsh

Param(
    [Parameter(Position = 0)]
    [ArgumentCompleter({
        $PossibleValues = @('macOS', 'Mac-Catalyst', 'iOS', 'iOS-Simulator')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Platform,

    [Parameter(Position = 1)]
    [ArgumentCompleter({
        $PossibleValues = @('AMD64', 'ARM64')
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
$CMakeToolchainFilePath = $Null
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
        Switch($Architecture) {
            'AMD64' { $CMakeIOSToolchainPlatform = 'MAC_CATALYST' }
            'ARM64' { $CMakeIOSToolchainPlatform = 'MAC_CATALYST_ARM64' }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Mac-Catalyst' platform."
                Exit 1
            }
        }
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
    If($CMakeGenerator -Eq 'Xcode') {
        $XcodeBuildCommand = (Get-Command -Name 'xcodebuild') 2> $Null
        If(-Not $XcodeBuildCommand) {
            Write-Error -Message "Could not find 'xcodebuild' command. Is Xcode installed on your machine?"
            Exit 1
        }
    }
    If(-Not (Test-Path -Path $CMakeToolchainFilePath -PathType Leaf)) {
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
    & $CMakeCommand $CMakeGeneratorFlags $CMakeToolchainFileFlags @($CMakeIOSToolchainPlatformFlag | Where-Object { $_ }) @($CMakeBuildTypeFlagForConfiguring | Where-Object { $_ }) "-DCMAKE_INSTALL_PREFIX=$OutDirectoryName" -B $BuildDirectoryName -S .
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to configure CMake project.'
        Exit 1
    }
    & $CMakeCommand --build $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
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
