#!/usr/bin/env pwsh

Param(
    [Parameter(Position = 0)]
    [ArgumentCompleter({
        $PossibleValues = @('Debug', 'Release', 'Both')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$BuildType
)

If(-Not $IsMacOS) {
    Write-Error -Message "Your machine needs to be running macOS for this script."
    Exit 1
}

If(-Not $BuildType) {
    $BuildType = 'Release'
    Write-Warning -Message "Using the implicit '$BuildType' build type."
}

$BuildTypes = $Null
If(($BuildType -Eq 'Debug') -Or ($BuildType -Eq 'Release')) {
    $BuildTypes = @($BuildType)
} ElseIf($BuildType -Eq 'Both') {
    $BuildTypes = @('Debug', 'Release')
} Else {
    Write-Error -Message "Invalid or unsupported '$BuildType' build type."
    Exit 1
}
If($BuildTypes -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $BuildTypes is not set.'
    Exit 1
}

$XcodeBuildCommand = (Get-Command -Name 'xcodebuild') 2> $Null
If(-Not $XcodeBuildCommand) {
    Write-Error -Message "Could not find 'xcodebuild' command. Is Xcode installed on your machine?"
    Exit 1
}

ForEach($BuildTypeFromList In $BuildTypes) {
    $FrameworkMacOSPath = "$PSScriptRoot/../out-macOS-$BuildTypeFromList/Posemesh.framework"
    If(-Not (Test-Path -Path $FrameworkMacOSPath -PathType Container)) {
        Write-Error -Message "Apple bundled framework for 'macOS' platform and '$BuildTypeFromList' build type does not exist."
        Exit 1
    }
    $FrameworkMacCatalystPath = "$PSScriptRoot/../out-Mac-Catalyst-$BuildTypeFromList/Posemesh.framework"
    If(-Not (Test-Path -Path $FrameworkMacCatalystPath -PathType Container)) {
        Write-Error -Message "Apple bundled framework for 'Mac-Catalyst' platform and '$BuildTypeFromList' build type does not exist."
        Exit 1
    }
    $FrameworkIOSPath = "$PSScriptRoot/../out-iOS-ARM64-$BuildTypeFromList/Posemesh.framework"
    If(-Not (Test-Path -Path $FrameworkIOSPath -PathType Container)) {
        Write-Error -Message "Apple framework for 'iOS' platform, 'ARM64' architecture and '$BuildTypeFromList' build type does not exist."
        Exit 1
    }
    $FrameworkIOSSimulatorPath = "$PSScriptRoot/../out-iOS-Simulator-$BuildTypeFromList/Posemesh.framework"
    If(-Not (Test-Path -Path $FrameworkIOSSimulatorPath -PathType Container)) {
        Write-Error -Message "Apple bundled framework for 'iOS-Simulator' platform and '$BuildTypeFromList' build type does not exist."
        Exit 1
    }
    $XCFrameworkPath = "$PSScriptRoot/../out-Apple-$BuildTypeFromList/Posemesh.xcframework"
    If(Test-Path -Path $XCFrameworkPath -PathType Container) {
        Remove-Item -Force -Recurse -Path $XCFrameworkPath 2> $Null
        If(Test-Path -Path $XCFrameworkPath -PathType Container) {
            Write-Error -Message "Failed to remove '$XCFrameworkPath' directory."
            Exit 1
        }
    }
    & $XcodeBuildCommand -create-xcframework -framework $FrameworkMacOSPath -framework $FrameworkMacCatalystPath -framework $FrameworkIOSPath -framework $FrameworkIOSSimulatorPath -output $XCFrameworkPath
    If($LastExitCode -Ne 0) {
        Write-Error -Message "Failed to bundle Posemesh XC framework for '$BuildTypeFromList' build type."
        Exit 1
    }
}
