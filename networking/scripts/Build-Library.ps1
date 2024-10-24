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
    [String]$BuildType,

    [Switch]$InstallNecessaryRustToolchainsAndTargets
)

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}

$RustToolchain = $Null
$RustTarget = $Null
$WASMTarget = $Null
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
        $RustToolchain = '1.81.0'
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
        $RustToolchain = 'nightly-2024-10-06'
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
        $RustToolchain = '1.81.0'
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
        $RustToolchain = '1.81.0'
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
        $RustToolchain = '1.81.0'
        $RustTarget = 'wasm32-unknown-unknown'
        $WASMTarget = 'no-modules'
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
$RustBuildTypeFlag = $Null
$WASMBuildTypeFlag = $Null
Switch($BuildType) {
    'Debug' {
        $RustBuildTypeFlag = ''
        $WASMBuildTypeFlag = '--dev'
    }
    'Release' {
        $RustBuildTypeFlag = '--release'
        $WASMBuildTypeFlag = '--release'
    }
    Default {
        Write-Error -Message "Invalid or unsupported '$BuildType' build type."
        Exit 1
    }
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

$WASMPackCommand = $Null
If($RustTarget -Eq 'wasm32-unknown-unknown') {
    $WASMPackCommand = (Get-Command -Name 'wasm-pack') 2> $Null
    If(-Not $WASMPackCommand) {
        Write-Error -Message "Could not find 'wasm-pack' command. Is WASM-Pack installed on your machine?"
        Exit 1
    }
} ElseIf(($RustTarget -Match "^wasm32-") -Or ($RustTarget -Match "^wasm64-")) {
    Write-Error -Message "Unsupported Rust '$RustTarget' web target."
    Exit 1
}

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
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
    $RustTargetList = & $RustUpCommand "+$RustToolchain" target list --installed
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
        If($WASMTarget -Eq $Null) {
            Write-Error -Message 'ASSERT: Variable $WASMTarget is not set.'
            Exit 1
        }
        & $RustUpCommand run $RustToolchain $WASMPackCommand build --target $WASMTarget @($WASMBuildTypeFlag | Where-Object { $_ }) --out-dir pkg/$BuildType --out-name PosemeshNetworking
    } Else {
        & $CargoCommand "+$RustToolchain" build --target $RustTarget @($RustBuildTypeFlag | Where-Object { $_ })
    }
    If($LastExitCode -Ne 0) {
        Write-Error -Message 'Failed to build Posemesh Networking library.'
        Exit 1
    }
} Finally {
    Pop-Location
}
