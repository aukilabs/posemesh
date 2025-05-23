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

$PlatformArchitectureCombinations = @{
    'macOS'         = @('AMD64', 'ARM64')
    'Mac-Catalyst'  = @('AMD64', 'ARM64')
    'iOS'           = @('ARM64')
    'iOS-Simulator' = @('AMD64', 'ARM64')
}

ForEach($BuildTypeFromList In $BuildTypes) {
    ForEach($Platform In $PlatformArchitectureCombinations.Keys) {
        $Architectures = $PlatformArchitectureCombinations[$Platform]
        ForEach($Architecture In $Architectures) {
            & "$PSScriptRoot/Build-Library.ps1" $Platform $Architecture $BuildTypeFromList
            If($LastExitCode -Ne 0) {
                Write-Error -Message "Failed to build Posemesh library for '$Platform' platform, '$Architecture' architecture and '$BuildTypeFromList' build type."
                Exit 1
            }
        }
    }
}
