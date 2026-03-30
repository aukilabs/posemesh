#!/usr/bin/env pwsh

$ErrorActionPreference = 'Stop'

Write-Host "Running smoke tests for Build-protobuf-Library.ps1 (issue #81)..."

$ScriptPath = Join-Path $PSScriptRoot "Build-protobuf-Library.ps1"

# Test 1: Incomplete arguments (only lowercase platform) should fail and not create bogus dirs
Write-Host "Test 1: Incomplete args with lowercase platform..."

$BogusBuildDir = Join-Path $PSScriptRoot "../build-protobuf-web--"
$BogusInstallDir = Join-Path $PSScriptRoot "../out-protobuf-web--"

if (Test-Path $BogusBuildDir) {
    Remove-Item -Recurse -Force $BogusBuildDir
}
if (Test-Path $BogusInstallDir) {
    Remove-Item -Recurse -Force $BogusInstallDir
}

pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath web -DryRun
$ExitCode = $LASTEXITCODE

if ($ExitCode -eq 0) {
    Write-Error "Expected non-zero exit code when called with missing arguments, got 0."
    exit 1
}

$HasBogusDirs = (Test-Path $BogusBuildDir) -or (Test-Path $BogusInstallDir)
if ($HasBogusDirs) {
    Write-Error "Bogus directories were created: $BogusBuildDir or $BogusInstallDir."
    exit 1
}

Write-Host "Test 1 passed."

# Test 2: Lowercase, fully specified args should be accepted and normalized
Write-Host "Test 2: Lowercase, fully specified args are normalized..."

$ExpectedBuildDir = Join-Path $PSScriptRoot "../build-protobuf-Web-WASM32-Release"
$ExpectedInstallDir = Join-Path $PSScriptRoot "../out-protobuf-Web-WASM32-Release"

if (Test-Path $ExpectedBuildDir) {
    Remove-Item -Recurse -Force $ExpectedBuildDir
}
if (Test-Path $ExpectedInstallDir) {
    Remove-Item -Recurse -Force $ExpectedInstallDir
}

pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath web wasm32 release -DryRun
$ExitCode = $LASTEXITCODE

if ($ExitCode -ne 0) {
    Write-Error "Expected zero exit code for valid, case-insensitive args, got $ExitCode."
    exit 1
}

$MissingExpected = (-not (Test-Path $ExpectedBuildDir)) -or (-not (Test-Path $ExpectedInstallDir))
if ($MissingExpected) {
    Write-Error "Expected canonical directories were not created: $ExpectedBuildDir or $ExpectedInstallDir."
    exit 1
}

$HasBogusDirs = (Test-Path $BogusBuildDir) -or (Test-Path $BogusInstallDir)
if ($HasBogusDirs) {
    Write-Error "Bogus directories were unexpectedly created in Test 2: $BogusBuildDir or $BogusInstallDir."
    exit 1
}

Write-Host "Test 2 passed."

# Test 3: Canonical, correctly cased args behave as control
Write-Host "Test 3: Canonical args behave correctly..."

if (Test-Path $ExpectedBuildDir) {
    Remove-Item -Recurse -Force $ExpectedBuildDir
}
if (Test-Path $ExpectedInstallDir) {
    Remove-Item -Recurse -Force $ExpectedInstallDir
}

pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath Web WASM32 Release -DryRun
$ExitCode = $LASTEXITCODE

if ($ExitCode -ne 0) {
    Write-Error "Expected zero exit code for canonical args, got $ExitCode."
    exit 1
}

$MissingExpected = (-not (Test-Path $ExpectedBuildDir)) -or (-not (Test-Path $ExpectedInstallDir))
if ($MissingExpected) {
    Write-Error "Expected canonical directories were not created for canonical args: $ExpectedBuildDir or $ExpectedInstallDir."
    exit 1
}

Write-Host "Test 3 passed."

# Test 4: Each parameter missing individually (and all missing) should fail
Write-Host "Test 4: Missing or ambiguous arguments fail..."

$missingCases = @(
    @{ Description = "Only platform" ; Args = @('web', '-DryRun') },
    @{ Description = "Platform + one extra token" ; Args = @('web', 'release', '-DryRun') },
    @{ Description = "No positional args" ; Args = @('-DryRun') }
)

foreach ($case in $missingCases) {
    Write-Host "  Case: $($case.Description)..."
    pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath @($case.Args)
    $ExitCode = $LASTEXITCODE
    if ($ExitCode -eq 0) {
        Write-Error "Expected non-zero exit code for missing/ambiguous args case '$($case.Description)', got 0."
        exit 1
    }
}

Write-Host "Test 4 passed."

# Test 5: Invalid values for platform/architecture/build type should fail
Write-Host "Test 5: Invalid values are rejected..."

$invalidCases = @(
    @{ Description = "Invalid platform" ; Args = @('Windows', 'AMD64', 'Release', '-DryRun') },
    @{ Description = "Invalid architecture" ; Args = @('Web', 'X86', 'Release', '-DryRun') },
    @{ Description = "Invalid build type" ; Args = @('Web', 'AMD64', 'Fast', '-DryRun') }
)

foreach ($case in $invalidCases) {
    Write-Host "  Case: $($case.Description)..."
    pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath @($case.Args)
    $ExitCode = $LASTEXITCODE
    if ($ExitCode -eq 0) {
        Write-Error "Expected non-zero exit code for invalid value case '$($case.Description)', got 0."
        exit 1
    }
}

Write-Host "Test 5 passed."

# Test 6: Non-Web platform sanity check (Linux ARM64 Release)
Write-Host "Test 6: Non-Web platform arguments behave correctly..."

$LinuxBuildDir = Join-Path $PSScriptRoot "../build-protobuf-Linux-ARM64-Release"
$LinuxInstallDir = Join-Path $PSScriptRoot "../out-protobuf-Linux-ARM64-Release"

if (Test-Path $LinuxBuildDir) {
    Remove-Item -Recurse -Force $LinuxBuildDir
}
if (Test-Path $LinuxInstallDir) {
    Remove-Item -Recurse -Force $LinuxInstallDir
}

pwsh -NoLogo -NoProfile -NonInteractive -File $ScriptPath linux arm64 release -DryRun
$ExitCode = $LASTEXITCODE

if ($ExitCode -ne 0) {
    Write-Error "Expected zero exit code for Linux ARM64 Release args, got $ExitCode."
    exit 1
}

$MissingLinuxDirs = (-not (Test-Path $LinuxBuildDir)) -or (-not (Test-Path $LinuxInstallDir))
if ($MissingLinuxDirs) {
    Write-Error "Expected Linux canonical directories were not created: $LinuxBuildDir or $LinuxInstallDir."
    exit 1
}

Write-Host "Test 6 passed."

# Final regression check: bogus web directories must not exist
$HasBogusDirs = (Test-Path $BogusBuildDir) -or (Test-Path $BogusInstallDir)
if ($HasBogusDirs) {
    Write-Error "Final regression check failed: bogus directories exist: $BogusBuildDir or $BogusInstallDir."
    exit 1
}

Write-Host "All smoke tests passed." -ForegroundColor Green
exit 0

