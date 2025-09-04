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
    [String]$BuildType,

    [Parameter(Position=3)]
    [String]$Package, 

    [Switch]$InstallNecessaryRustToolchainsAndTargets
)

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}

If(-Not $Package) {
    Write-Error -Message "Parameter '-Package' is not specified."
    Exit 1
}

$RustToolchain = $Null
$RustTarget = $Null
$WASMTarget = $Null
$NewCargoTargetAArch64UnknownLinuxGNULinker = $Null
$NewCCAArch64UnknownLinuxGNU = $Null
$NewARAArch64UnknownLinuxGNU = $Null
$NewCFlagsAArch64UnknownLinuxGNU = $Null
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
        $RustToolchain = '1.87.0'
        Switch($Architecture) {
            'AMD64' { $RustTarget = 'x86_64-apple-darwin' }
            'ARM64' { $RustTarget = 'aarch64-apple-darwin' }
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
        $RustToolchain = 'nightly'
        Switch($Architecture) {
            'AMD64' { $RustTarget = 'x86_64-apple-ios-macabi' }
            'ARM64' { $RustTarget = 'aarch64-apple-ios-macabi' }
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
        $RustToolchain = '1.87.0'
        $RustTarget = 'aarch64-apple-ios'
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
        $RustToolchain = '1.87.0'
        Switch($Architecture) {
            'AMD64' { $RustTarget = 'x86_64-apple-ios' }
            'ARM64' { $RustTarget = 'aarch64-apple-ios-sim' }
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
        $RustToolchain = '1.87.0'
        $RustTarget = 'wasm32-unknown-unknown'
        $WASMTarget = 'bundler'
    }
    'Linux' {
        If(-Not $IsLinux) {
            Write-Error -Message "Your machine needs to be running GNU/Linux to build for 'Linux' platform."
            Exit 1
        }
        $UNameCommand = (Get-Command -Name 'uname') 2> $Null
        If(-Not $UNameCommand) {
            Write-Error -Message "Could not find 'uname' command."
            Exit 1
        }
        $UNameResult = & $UNameCommand -m
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to determine the current running platform architecture.'
            Exit 1
        }
        If(-Not (($UNameResult -Match 'x86_64') -Or ($UNameResult -Match 'amd64'))) {
            Write-Error -Message "The current running platform should be using 'x86_64' architecture, however yours is actually using '$UNameResult' architecture."
            Exit 1
        }
        If(-Not $Architecture) {
            Write-Error -Message "Parameter '-Architecture' is not specified for 'Linux' platform."
            Exit 1
        }
        $RustToolchain = '1.87.0'
        Switch($Architecture) {
            'AMD64' { $RustTarget = 'x86_64-unknown-linux-gnu' }
            'ARM64' {
                $RustTarget = 'aarch64-unknown-linux-gnu'
                $NewCargoTargetAArch64UnknownLinuxGNULinker = 'aarch64-linux-gnu-gcc'
                $LLVMVersion = 16
                $NewCCAArch64UnknownLinuxGNU = "clang-$LLVMVersion"
                $NewARAArch64UnknownLinuxGNU = "llvm-ar-$LLVMVersion"
                $NewCFlagsAArch64UnknownLinuxGNU="--sysroot=/usr/aarch64-linux-gnu"
            }
            Default {
                Write-Error -Message "Invalid or unsupported '$Architecture' architecture for 'Linux' platform."
                Exit 1
            }
        }
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$Platform' platform."
        Exit 1
    }
}
If($RustToolchain -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $RustToolchain is not set.'
    Exit 1
}
If($RustTarget -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $RustTarget is not set.'
    Exit 1
}

If(-Not $BuildType) {
    $BuildType = 'Release'
    Write-Warning -Message "Using the implicit '$BuildType' build type."
}
$RustBuildTypeDirName = $Null
$RustBuildTypeFlag = $Null
$WASMBuildTypeFlag = $Null
Switch($BuildType) {
    'Debug' {
        $RustBuildTypeDirName = 'debug'
        $RustBuildTypeFlag = ''
        $WASMBuildTypeFlag = '--dev'
    }
    'Release' {
        $RustBuildTypeDirName = 'release'
        $RustBuildTypeFlag = '--release'
        $WASMBuildTypeFlag = '--release'
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$BuildType' build type."
        Exit 1
    }
}
If($RustBuildTypeDirName -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $RustBuildTypeDirName is not set.'
    Exit 1
}
If($RustBuildTypeFlag -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $RustBuildTypeFlag is not set.'
    Exit 1
}
If($WASMBuildTypeFlag -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $WASMBuildTypeFlag is not set.'
    Exit 1
}

$RustUpCommand = (Get-Command -Name 'rustup') 2> $Null
If(-Not $RustUpCommand) {
    Write-Error -Message "Could not find 'rustup' command. Is Rust installed on your machine?"
    Exit 1
}

$CargoCommand = (Get-Command -Name 'cargo') 2> $Null
If(-Not $CargoCommand) {
    Write-Error -Message "Could not find 'cargo' command. Is Rust installed on your machine?"
    Exit 1
}

$NewCC = $Null
$NewCXX = $Null

$WASMPackCommand = $Null
If($RustTarget -Eq 'wasm32-unknown-unknown') {
    $WASMPackCommand = (Get-Command -Name 'wasm-pack') 2> $Null
    If(-Not $WASMPackCommand) {
        Write-Error -Message "Could not find 'wasm-pack' command. Is WASM-Pack installed on your machine?"
        Exit 1
    }
    If($IsMacOS) {
        $BrewCommand = (Get-Command -Name 'brew') 2> $Null
        If(-Not $BrewCommand) {
            Write-Error -Message "Could not find 'brew' command. Is Homebrew installed on your machine?"
            Exit 1
        }
        $BrewListResult = (& $BrewCommand list llvm) 2> $Null
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Could not find 'llvm' package. Is LLVM installed on your machine via Homebrew? If not, please run the 'brew install llvm' command."
            Exit 1
        }
        $BrewListVersionsResult = & $BrewCommand list --versions llvm
        $FoundGreatestVersionNumber = $Null
        ForEach($PotentialVersionNumber In ($BrewListVersionsResult -Split ' ')) {
            Try {
                If($PotentialVersionNumber -Match '[0-9]+\.[0-9]+(\.[0-9]+(\.[0-9]+)?)?') {
                    $PotentialVersionNumber = $Matches[0]
                } Else {
                    Continue
                }
                $VersionNumber = [System.Version]$PotentialVersionNumber
                If($FoundGreatestVersionNumber) {
                    If($VersionNumber -Gt $FoundGreatestVersionNumber) {
                        $FoundGreatestVersionNumber = $VersionNumber
                    }
                } Else {
                    $FoundGreatestVersionNumber = $VersionNumber
                }
            } Catch {
                Continue
            }
        }
        If($FoundGreatestVersionNumber) {
            $MinRequiredVersionNumber = [System.Version]'16.0.0'
            If($MinRequiredVersionNumber -Gt $FoundGreatestVersionNumber) {
                Write-Error -Message "Minimum required LLVM package version is $MinRequiredVersionNumber but the installed version is $FoundGreatestVersionNumber. Please run the 'brew upgrade llvm' command."
                Exit 1
            }
        } Else {
            Write-Error -Message 'Could not determine the installed LLVM package version.'
            Exit 1
        }
        $PkgDirPrefix = & $BrewCommand --prefix llvm
        $NewCC = "$PkgDirPrefix/bin/clang"
        $NewCXX = "$PkgDirPrefix/bin/clang++"
    }
} ElseIf(($RustTarget -Match "^wasm32-") -Or ($RustTarget -Match "^wasm64-")) {
    Write-Error -Message "Unsupported Rust '$RustTarget' web target."
    Exit 1
}

$OldCargoTargetAArch64UnknownLinuxGNULinker = $env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER
$OldCCAArch64UnknownLinuxGNU = $env:CC_aarch64_unknown_linux_gnu
$OldARAArch64UnknownLinuxGNU = $env:AR_aarch64_unknown_linux_gnu
$OldCFlagsAArch64UnknownLinuxGNU = $env:CFLAGS_aarch64_unknown_linux_gnu
$OldCC = $env:CC
$OldCXX = $env:CXX

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    If($NewCargoTargetAArch64UnknownLinuxGNULinker) {
        $env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = $NewCargoTargetAArch64UnknownLinuxGNULinker
    }
    If($NewCCAArch64UnknownLinuxGNU) {
        $env:CC_aarch64_unknown_linux_gnu = $NewCCAArch64UnknownLinuxGNU
    }
    If($NewARAArch64UnknownLinuxGNU) {
        $env:AR_aarch64_unknown_linux_gnu = $NewARAArch64UnknownLinuxGNU
    }
    If($NewCFlagsAArch64UnknownLinuxGNU) {
        $env:CFLAGS_aarch64_unknown_linux_gnu = $NewCFlagsAArch64UnknownLinuxGNU
    }
    If($NewCC) {
        $env:CC = $NewCC
    }
    If($NewCXX) {
        $env:CXX = $NewCXX
    }
    $RustToolchainList = & $RustUpCommand toolchain list
    $RustToolchainInstalled = $False
    ForEach($RustToolchainListItem In $RustToolchainList) {
        If(($RustToolchainListItem -Match "^\s*$RustToolchain$") -Or ($RustToolchainListItem -Match "^\s*$RustToolchain[ -]")) {
            $RustToolchainInstalled = $True
            Break
        }
    }
    If(-Not $RustToolchainInstalled) {
        If(-Not $InstallNecessaryRustToolchainsAndTargets) {
            Write-Error -Message "Rust toolchain '$RustToolchain' is required. Please run this script with '-InstallNecessaryRustToolchainsAndTargets' flag or alternatively run 'rustup toolchain install $RustToolchain'."
            Exit 1
        }
        & $RustUpCommand toolchain install $RustToolchain
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Failed to install Rust '$RustToolchain' toolchain."
            Exit 1
        }
    }
    $RustTargetList = & $RustUpCommand target list --toolchain "$RustToolchain" --installed
    $RustTargetInstalled = $False
    ForEach($RustTargetListItem In $RustTargetList) {
        If($RustTargetListItem -Match "^\s*$RustTarget\s*$") {
            $RustTargetInstalled = $True
            Break
        }
    }
    If(-Not $RustTargetInstalled) {
        If(-Not $InstallNecessaryRustToolchainsAndTargets) {
            Write-Error -Message "Rust target '$RustTarget' is required for '$RustToolchain' toolchain. Please run this script with '-InstallNecessaryRustToolchainsAndTargets' flag or alternatively run 'rustup +$RustToolchain target add $RustTarget'."
            Exit 1
        }
        & $RustUpCommand "+$RustToolchain" target add $RustTarget
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Failed to install Rust '$RustTarget' target for '$RustToolchain' toolchain."
            Exit 1
        }
    }
    If($WASMPackCommand) {
        $capitalized = ($package.Substring(0,1).ToUpper() + $package.Substring(1))
        If($WASMTarget -Eq $Null) {
            Write-Error -Message 'ASSERT: Variable $WASMTarget is not set.'
            Exit 1
        }
        & $RustUpCommand run $RustToolchain $WASMPackCommand build --target $WASMTarget @($WASMBuildTypeFlag | Where-Object { $_ }) --out-dir ../pkg/$Package/$BuildType --out-name Posemesh$capitalized $Package --features c
    } Else {
        & $CargoCommand "+$RustToolchain" build --target $RustTarget @($RustBuildTypeFlag | Where-Object { $_ }) --features "c" --package $Package
    }
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to build Posemesh Core library.'
        Exit 1
    }
    If(-Not $WASMPackCommand) {
        $StaticLibraryPathRenamed = "target/$RustTarget/$RustBuildTypeDirName/lib${Package}_static.a"
        If(Test-Path -Path $StaticLibraryPathRenamed -PathType Leaf) {
            Remove-Item -Force -Path $StaticLibraryPathRenamed 2> $Null
            If(Test-Path -Path $StaticLibraryPathRenamed -PathType Leaf) {
                Write-Error -Message "Failed to remove '$StaticLibraryPathRenamed' file."
                Exit 1
            }
        }
        $StaticLibraryPathOriginal = "target/$RustTarget/$RustBuildTypeDirName/lib$Package.a"
        If(-Not (Test-Path -Path $StaticLibraryPathOriginal -PathType Leaf)) {
            Write-Error -Message "File '$StaticLibraryPathOriginal' does not exist."
            Exit 1
        }
        $CopyItemResult = $(Copy-Item -Path $StaticLibraryPathOriginal -Destination $StaticLibraryPathRenamed) 2>&1
        If($CopyItemResult) {
            Write-Error -Message "Failed to copy '$StaticLibraryPathOriginal' file over to '$StaticLibraryPathRenamed' destination."
            Exit 1
        }
    }
} Finally {
    If($NewCargoTargetAArch64UnknownLinuxGNULinker) {
        $env:CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = $OldCargoTargetAArch64UnknownLinuxGNULinker
    }
    If($NewCCAArch64UnknownLinuxGNU) {
        $env:CC_aarch64_unknown_linux_gnu = $OldCCAArch64UnknownLinuxGNU
    }
    If($NewARAArch64UnknownLinuxGNU) {
        $env:AR_aarch64_unknown_linux_gnu = $OldARAArch64UnknownLinuxGNU
    }
    If($NewCFlagsAArch64UnknownLinuxGNU) {
        $env:CFLAGS_aarch64_unknown_linux_gnu = $OldCFlagsAArch64UnknownLinuxGNU
    }
    If($NewCC) {
        $env:CC = $OldCC
    }
    If($NewCXX) {
        $env:CXX = $OldCXX
    }
    Pop-Location
}
