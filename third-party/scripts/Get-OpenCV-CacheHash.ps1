#!/usr/bin/env pwsh

Param()

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/../.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    $HashContent = ''
    Try {
        $PushLocationResult = (Push-Location -Path "third-party/opencv" -PassThru) 2> $Null
        If(-Not $PushLocationResult) {
            Write-Error -Message 'Failed to push the required working directory.'
            Exit 1
        }
        $GitCommitId = & git log -1 --format=%H
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to get Git commit ID.'
            Exit 1
        }
        $GitCommitId = $GitCommitId.ToLower()
        $HashContent += "git_commit_id=$GitCommitId;"
    } Finally {
        Pop-Location
    }

    $PosemeshSDKWorkflowHash = Get-FileHash -Path '.github/workflows/posemesh-sdk.yml' -Algorithm MD5 2> $Null
    If(-Not $PosemeshSDKWorkflowHash) {
        Write-Error -Message "Could not get MD5 hash of '.github/workflows/posemesh-sdk.yml' file."
        Exit 1
    }
    $PosemeshSDKWorkflowHash = $PosemeshSDKWorkflowHash.Hash.ToLower()
    $HashContent += "posemesh_sdk_workflow_hash=$PosemeshSDKWorkflowHash;"

    $BuildOpenCVAppleScriptHash = Get-FileHash -Path 'third-party/scripts/Build-OpenCV-Apple.ps1' -Algorithm MD5 2> $Null
    If(-Not $BuildOpenCVAppleScriptHash) {
        Write-Error -Message "Could not get MD5 hash of 'third-party/scripts/Build-OpenCV-Apple.ps1' file."
        Exit 1
    }
    $BuildOpenCVAppleScriptHash = $BuildOpenCVAppleScriptHash.Hash.ToLower()
    $HashContent += "build_opencv_apple_script_hash=$BuildOpenCVAppleScriptHash;"

    $BuildOpenCVLibraryScriptHash = Get-FileHash -Path 'third-party/scripts/Build-OpenCV-Library.ps1' -Algorithm MD5 2> $Null
    If(-Not $BuildOpenCVLibraryScriptHash) {
        Write-Error -Message "Could not get MD5 hash of 'third-party/scripts/Build-OpenCV-Library.ps1' file."
        Exit 1
    }
    $BuildOpenCVLibraryScriptHash = $BuildOpenCVLibraryScriptHash.Hash.ToLower()
    $HashContent += "build_opencv_library_script_hash=$BuildOpenCVLibraryScriptHash;"

    $GetOpenCVCacheHashScriptHash = Get-FileHash -Path 'third-party/scripts/Get-OpenCV-CacheHash.ps1' -Algorithm MD5 2> $Null
    If(-Not $GetOpenCVCacheHashScriptHash) {
        Write-Error -Message "Could not get MD5 hash of 'third-party/scripts/Get-OpenCV-CacheHash.ps1' file."
        Exit 1
    }
    $GetOpenCVCacheHashScriptHash = $GetOpenCVCacheHashScriptHash.Hash.ToLower()
    $HashContent += "get_opencv_cache_hash_script_hash=$GetOpenCVCacheHashScriptHash;"

    $HashContentBytes = [System.Text.Encoding]::UTF8.GetBytes($HashContent)
    $MD5 = [System.Security.Cryptography.MD5]::Create()
    $HashBytes = $MD5.ComputeHash($HashContentBytes)
    $HashString = [BitConverter]::ToString($HashBytes).Replace('-', '').ToLower()
    Write-Output $HashString
} Finally {
    Pop-Location
}
