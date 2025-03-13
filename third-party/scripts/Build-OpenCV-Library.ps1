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
    [String]$BuildType
)

$RequiredEmscriptenVersion = '3.1.69'

If(-Not $Platform) {
    Write-Error -Message "Parameter '-Platform' is not specified."
    Exit 1
}

$UseCMakeDirectly = $False
$CMakeGenerator = $Null
$CMakeConfigureArgs = $Null
$UseBuildPythonScript = $False
$BuildPythonScriptFile = $Null
$BuildPythonScriptArgs = $Null
$InvokeWithEmscripten = $False
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
        $UseBuildPythonScript = $True
        $BuildPythonScriptFile = 'opencv/platforms/js/build_js.py'
        $BuildPythonScriptArgs = @()
        $InvokeWithEmscripten = $True
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
            Write-Error -Message "The current running platform should be using x86_64 architecture however is actually using '$UNameResult' architecture."
            Exit 1
        }
        If(-Not $Architecture) {
            Write-Error -Message "Parameter '-Architecture' is not specified for 'Linux' platform."
            Exit 1
        }
        $UseCMakeDirectly = $True
        $CMakeGenerator = ''
        $CMakeConfigureArgs = @()
        Switch($Architecture) {
            'AMD64' { }
            'ARM64' {
                $CMakeToolchainFilePath = (Resolve-Path "$PSScriptRoot/../opencv/platforms/linux/aarch64-gnu.toolchain.cmake").Path
                $CMakeConfigureArgs += "-DCMAKE_TOOLCHAIN_FILE=$CMakeToolchainFilePath"
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
If(($UseCMakeDirectly -And ($CMakeGenerator -Eq 'Xcode')) -Or ($UseBuildPythonScript -And ($Platform -Match '^macOS|Mac-Catalyst|iOS|iOS-Simulator$'))) {
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

$EmSDKCommand = $Null
$EmMakeCommand = $Null
$EmCMakeCommand = $Null
If($InvokeWithEmscripten) {
    $EmSDKCommand = (Get-Command -Name 'emsdk') 2> $Null
    If(-Not $EmSDKCommand) {
        Write-Error -Message "Could not find 'emsdk' command. Is Emscripten installed on your machine?"
        Exit 1
    }
    $SelectStringResult = (& $EmSDKCommand list) | Select-String -Pattern '([0-9]+\.[0-9]+\.[0-9]+)\s+INSTALLED'
    If(-Not $SelectStringResult) {
        Write-Error -Message 'Failed to determine Emscripten version.'
        Exit 1
    }
    $EmscriptenVersion = $SelectStringResult.Matches[0].Groups[1].Value
    If($EmscriptenVersion -Ne $RequiredEmscriptenVersion) {
        Write-Error -Message "Required Emscripten version is $RequiredEmscriptenVersion but the installed version is $EmscriptenVersion. Please run the 'emsdk install $RequiredEmscriptenVersion && emsdk activate $RequiredEmscriptenVersion' command."
        Exit 1
    }

    $EmCMakeCommand = (Get-Command -Name 'emcmake') 2> $Null
    If(-Not $EmCMakeCommand) {
        Write-Error -Message "Could not find 'emcmake' command. Is Emscripten installed on your machine?"
        Exit 1
    }

    $EmMakeCommand = (Get-Command -Name 'emmake') 2> $Null
    If(-Not $EmMakeCommand) {
        Write-Error -Message "Could not find 'emmake' command. Is Emscripten installed on your machine?"
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

Function Unpack-Framework {
    Param(
        [Parameter(Position = 0, Mandatory = $True)]
        [String]$FrameworkPath,

        [Parameter(Position = 1, Mandatory = $True)]
        [String]$OutputPath
    )

    If(-Not (Test-Path -Path $FrameworkPath -PathType Container)) {
        Write-Error -Message "Directory '$FrameworkPath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $OutputPath -PathType Container)) {
        Write-Error -Message "Directory '$OutputPath' does not exist."
        Exit 1
    }

    $HeadersFrameworkPath = "$FrameworkPath/Headers"
    $ArchiveFrameworkPath = "$FrameworkPath/opencv2"
    If(-Not (Test-Path -Path $HeadersFrameworkPath -PathType Container)) {
        Write-Error -Message "Directory '$HeadersFrameworkPath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $ArchiveFrameworkPath -PathType Leaf)) {
        Write-Error -Message "File '$ArchiveFrameworkPath' does not exist."
        Exit 1
    }

    $IncludeOutputPath = "$OutputPath/include"
    $LibraryOutputPath = "$OutputPath/lib"
    If(Test-Path -Path $IncludeOutputPath -PathType Container) {
        Remove-Item -Force -Recurse -Path $IncludeOutputPath 2> $Null
        If(Test-Path -Path $IncludeOutputPath -PathType Container) {
            Write-Error -Message "Failed to remove '$IncludeOutputPath' directory."
            Exit 1
        }
    }
    If(Test-Path -Path $LibraryOutputPath -PathType Container) {
        Remove-Item -Force -Recurse -Path $LibraryOutputPath 2> $Null
        If(Test-Path -Path $LibraryOutputPath -PathType Container) {
            Write-Error -Message "Failed to remove '$LibraryOutputPath' directory."
            Exit 1
        }
    }
    $NewItemResult = (New-Item -Path $IncludeOutputPath -ItemType Directory) 2> $Null
    If(-Not $NewItemResult) {
        Write-Error -Message "Failed to create '$IncludeOutputPath' directory."
        Exit 1
    }
    $NewItemResult = (New-Item -Path $LibraryOutputPath -ItemType Directory) 2> $Null
    If(-Not $NewItemResult) {
        Write-Error -Message "Failed to create '$LibraryOutputPath' directory."
        Exit 1
    }

    $CopyItemResult = $(Copy-Item -Path $HeadersFrameworkPath -Destination "$IncludeOutputPath/opencv2" -Recurse) 2>&1
    If($CopyItemResult) {
        Write-Error -Message "Failed to copy '$HeadersFrameworkPath' directory over to '$IncludeOutputPath/opencv2' destination."
        Exit 1
    }

    $CopyItemResult = $(Copy-Item -Path $ArchiveFrameworkPath -Destination "$LibraryOutputPath/libopencv2.a") 2>&1
    If($CopyItemResult) {
        Write-Error -Message "Failed to copy '$ArchiveFrameworkPath' file over to '$LibraryOutputPath/libopencv2.a' destination."
        Exit 1
    }
}

Function Install-Library {
    Param(
        [Parameter(Position = 0, Mandatory = $True)]
        [String]$BuildPath,

        [Parameter(Position = 1, Mandatory = $True)]
        [String]$OutputPath
    )

    If(-Not (Test-Path -Path $BuildPath -PathType Container)) {
        Write-Error -Message "Directory '$BuildPath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $OutputPath -PathType Container)) {
        Write-Error -Message "Directory '$OutputPath' does not exist."
        Exit 1
    }

    $OpenCV2SourcePath = 'opencv/include/opencv2'
    $ModulesSourcePath = 'opencv/modules'
    If(-Not (Test-Path -Path $OpenCV2SourcePath -PathType Container)) {
        Write-Error -Message "Directory '$OpenCV2SourcePath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $ModulesSourcePath -PathType Container)) {
        Write-Error -Message "Directory '$ModulesSourcePath' does not exist."
        Exit 1
    }

    $OpenCV2BuildPath = "$BuildPath/opencv2"
    $ModulesBuildPath = "$BuildPath/modules"
    $LibraryBuildPath = "$BuildPath/lib"
    If(-Not (Test-Path -Path $OpenCV2BuildPath -PathType Container)) {
        Write-Error -Message "Directory '$OpenCV2BuildPath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $ModulesBuildPath -PathType Container)) {
        Write-Error -Message "Directory '$ModulesBuildPath' does not exist."
        Exit 1
    }
    If(-Not (Test-Path -Path $LibraryBuildPath -PathType Container)) {
        Write-Error -Message "Directory '$LibraryBuildPath' does not exist."
        Exit 1
    }

    $IncludeOutputPath = "$OutputPath/include"
    $LibraryOutputPath = "$OutputPath/lib"
    If(Test-Path -Path $IncludeOutputPath -PathType Container) {
        Remove-Item -Force -Recurse -Path $IncludeOutputPath 2> $Null
        If(Test-Path -Path $IncludeOutputPath -PathType Container) {
            Write-Error -Message "Failed to remove '$IncludeOutputPath' directory."
            Exit 1
        }
    }
    If(Test-Path -Path $LibraryOutputPath -PathType Container) {
        Remove-Item -Force -Recurse -Path $LibraryOutputPath 2> $Null
        If(Test-Path -Path $LibraryOutputPath -PathType Container) {
            Write-Error -Message "Failed to remove '$LibraryOutputPath' directory."
            Exit 1
        }
    }
    $NewItemResult = (New-Item -Path $IncludeOutputPath -ItemType Directory) 2> $Null
    If(-Not $NewItemResult) {
        Write-Error -Message "Failed to create '$IncludeOutputPath' directory."
        Exit 1
    }

    $CopyItemResult = $(Copy-Item -Path $OpenCV2SourcePath -Destination $IncludeOutputPath -Recurse) 2>&1
    If($CopyItemResult) {
        Write-Error -Message "Failed to copy '$OpenCV2SourcePath' directory over to '$IncludeOutputPath' destination."
        Exit 1
    }
    $ModulesSource = Get-ChildItem -Path $ModulesSourcePath -Directory
    ForEach($ModuleSource In $ModulesSource) {
        $ModulePath = $ModuleSource.FullName + '/include/opencv2'
        If(-Not (Test-Path -Path $ModulePath -PathType Container)) {
            Continue
        }
        $CopyItemResult = $(Copy-Item -Path $ModulePath -Destination $IncludeOutputPath -Recurse -Force) 2>&1
        If($CopyItemResult) {
            Write-Error -Message "Failed to copy '$ModulePath' directory over to '$IncludeOutputPath' destination."
            Exit 1
        }
    }

    $CopyItemResult = $(Copy-Item -Path $OpenCV2BuildPath -Destination $IncludeOutputPath -Recurse -Force) 2>&1
    If($CopyItemResult) {
        Write-Error -Message "Failed to copy '$OpenCV2BuildPath' directory over to '$IncludeOutputPath' destination."
        Exit 1
    }

    $CopyItemResult = $(Copy-Item -Path $LibraryBuildPath -Destination $LibraryOutputPath -Recurse) 2>&1
    If($CopyItemResult) {
        Write-Error -Message "Failed to copy '$LibraryBuildPath' directory over to '$LibraryOutputPath' destination."
        Exit 1
    }
}

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
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
        If($InvokeWithEmscripten) {
            & $EmCMakeCommand $CMakeCommand $CMakeGeneratorFlags $CMakeConfigureArgs @($CMakeBuildTypeFlagForConfiguring | Where-Object { $_ }) "-DCMAKE_INSTALL_PREFIX=$OutDirectoryName" -B $BuildDirectoryName -S opencv
        } Else {
            & $CMakeCommand $CMakeGeneratorFlags $CMakeConfigureArgs @($CMakeBuildTypeFlagForConfiguring | Where-Object { $_ }) "-DCMAKE_INSTALL_PREFIX=$OutDirectoryName" -B $BuildDirectoryName -S opencv
        }
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to configure CMake project.'
            Exit 1
        }
        If($InvokeWithEmscripten) {
            & $EmMakeCommand $CMakeCommand --build $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
        } ElseIf(($CMakeGenerator -Eq '') -Or ($CMakeGenerator -Eq 'Unix Makefiles')) {
            & $CMakeCommand --build $BuildDirectoryName --parallel 4 @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
        } Else {
            & $CMakeCommand --build $BuildDirectoryName @($CMakeBuildTypeFlagForBuildingAndInstalling | Where-Object { $_ })
        }
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
        If($LastExitCode -Ne 0) {
            Write-Error -Message 'Failed to set up Python virtual environment.'
            Exit 1
        }
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
            If($InvokeWithEmscripten) {
                & $EmCMakeCommand $Python3CommandFromVEnv $BuildPythonScriptFile $BuildDirectoryName $BuildPythonScriptArgs @($BuildPythonScriptBuildTypeFlag | Where-Object { $_ })
            } Else {
                & $Python3CommandFromVEnv $BuildPythonScriptFile $BuildDirectoryName $BuildPythonScriptArgs @($BuildPythonScriptBuildTypeFlag | Where-Object { $_ })
            }
            If($LastExitCode -Ne 0) {
                Write-Error -Message 'Failed to build OpenCV library.'
                Exit 1
            }
            If($BuildPythonScriptFile -Like '*framework*') {
                Unpack-Framework "$BuildDirectoryName/opencv2.framework" $OutDirectoryName
            } ElseIf($BuildPythonScriptFile -Like '*js*') {
                Install-Library $BuildDirectoryName $OutDirectoryName
            }
        } Finally {
            $env:VIRTUAL_ENV = $VirtualEnvBackup
            $env:PATH = $PathBackup
        }
    }
} Finally {
    Pop-Location
}
