#!/usr/bin/env pwsh

Param(
    [Parameter(Position = 0)]
    [ArgumentCompleter({
        $PossibleValues = @('macOS', 'Mac-Catalyst', 'iOS-Simulator', 'All')
        return $PossibleValues | ForEach-Object { $_ }
    })]
    [String]$Platform,

    [Parameter(Position = 1)]
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

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}

$Platforms = $Null
Switch($Platform) {
    'macOS' { $Platforms = @($Platform) }
    'Mac-Catalyst' { $Platforms = @($Platform) }
    'iOS-Simulator' { $Platforms = @($Platform) }
    'All' { $Platforms = @('macOS', 'Mac-Catalyst', 'iOS-Simulator') }
    Default {
        Write-Error -Message "Invalid or unsupported '$Platform' platform."
        Exit 1
    }
}
If($Platforms -Eq $Null) {
    Write-Error -Message 'ASSERT: Variable $Platforms is not set.'
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

$LipoCommand = (Get-Command -Name 'lipo') 2> $Null
If(-Not $LipoCommand) {
    Write-Error -Message "Could not find 'lipo' command. Is Xcode installed on your machine?"
    Exit 1
}

ForEach($PlatformFromList In $Platforms) {
    ForEach($BuildTypeFromList In $BuildTypes) {
        $FrameworkAMD64Path = "$PSScriptRoot/../out-$PlatformFromList-AMD64-$BuildTypeFromList/Posemesh.framework"
        If(-Not (Test-Path -Path $FrameworkAMD64Path -PathType Container)) {
            Write-Error -Message "Apple framework for '$PlatformFromList' platform, 'AMD64' architecture and '$BuildTypeFromList' build type does not exist."
            Exit 1
        }
        $FrameworkARM64Path = "$PSScriptRoot/../out-$PlatformFromList-ARM64-$BuildTypeFromList/Posemesh.framework"
        If(-Not (Test-Path -Path $FrameworkARM64Path -PathType Container)) {
            Write-Error -Message "Apple framework for '$PlatformFromList' platform, 'ARM64' architecture and '$BuildTypeFromList' build type does not exist."
            Exit 1
        }
        $FrameworkBundleParentPath = "$PSScriptRoot/../out-$PlatformFromList-$BuildTypeFromList"
        $FrameworkBundlePath = "$FrameworkBundleParentPath/Posemesh.framework"
        If(Test-Path -Path $FrameworkBundlePath -PathType Container) {
            Remove-Item -Force -Recurse -Path $FrameworkBundlePath 2> $Null
            If(Test-Path -Path $FrameworkBundlePath -PathType Container) {
                Write-Error -Message "Failed to remove '$FrameworkBundlePath' directory."
                Exit 1
            }
        }
        If(-Not (Test-Path -Path $FrameworkBundleParentPath -PathType Container)) {
            New-Item -Path $FrameworkBundleParentPath -ItemType Directory 2>&1 | Out-Null
            If(-Not (Test-Path -Path $FrameworkBundleParentPath -PathType Container)) {
                Write-Error -Message "Failed to create '$FrameworkBundleParentPath' directory."
                Exit 1
            }
        }
        $FrameworkAMD64Lib = "$FrameworkAMD64Path/Posemesh"
        If(-Not (Test-Path -Path $FrameworkAMD64Lib -PathType Leaf)) {
            Write-Error -Message "Apple framework for '$PlatformFromList' platform, 'AMD64' architecture and '$BuildTypeFromList' build type is invalid."
            Exit 1
        }
        $FrameworkARM64Lib = "$FrameworkARM64Path/Posemesh"
        If(-Not (Test-Path -Path $FrameworkARM64Lib -PathType Leaf)) {
            Write-Error -Message "Apple framework for '$PlatformFromList' platform, 'ARM64' architecture and '$BuildTypeFromList' build type is invalid."
            Exit 1
        }
        $FrameworkBundleLib = "$FrameworkBundlePath/Posemesh"
        & cp -R $FrameworkAMD64Path $FrameworkBundlePath 2>&1 | Out-Null # Use native cp instead of Copy-Item command because we need to respect symlinks
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Failed to copy '$FrameworkAMD64Path' directory over to '$FrameworkBundlePath' destination."
            Exit 1
        }
        & cp -R $FrameworkARM64Path "$FrameworkBundlePath/.." 2>&1 | Out-Null # Use native cp instead of Copy-Item command because we need to respect symlinks
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Failed to copy '$FrameworkARM64Path' directory over to '$FrameworkBundlePath' destination."
            Exit 1
        }
        & $LipoCommand -create $FrameworkAMD64Lib $FrameworkARM64Lib -output $FrameworkBundleLib
        If($LastExitCode -Ne 0) {
            Write-Error -Message "Failed to bundle Posemesh framework for '$PlatformFromList' platform and '$BuildTypeFromList' build type."
            Exit 1
        }
    }
}
