#!/usr/bin/env pwsh

Param()

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    $Content = Get-Content -Path './CMakeLists.txt'
    If(-Not $Content) {
        Write-Error -Message 'Could not read the contents of CMakeLists.txt file.'
        Exit 1
    }
    $SelectStringResult = $Content | Select-String -Pattern 'Posemesh\s+VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)'
    If(-Not $SelectStringResult) {
        Write-Error -Message 'Failed to extract the declared Posemesh SDK version from CMakeLists.txt file.'
        Exit 1
    }
    Write-Output $SelectStringResult.Matches[0].Groups[1].Value
} Finally {
    Pop-Location
}
