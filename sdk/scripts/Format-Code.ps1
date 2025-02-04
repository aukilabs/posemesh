#!/usr/bin/env pwsh

Param()

$ClangFormatCommand = (Get-Command -Name 'clang-format') 2> $Null
If(-Not $ClangFormatCommand) {
    Write-Error -Message "Could not find 'clang-format' command. Is Clang-Format installed on your machine?"
    Exit 1
}

$PushLocationResult = (Push-Location -Path "$PSScriptRoot/.." -PassThru) 2> $Null
If(-Not $PushLocationResult) {
    Write-Error -Message 'Failed to push the required working directory.'
    Exit 1
}
Try {
    $AllSourceCodeFiles = Get-ChildItem -Path './include', './platform', './src' -Recurse -Include '*.h', '*.hh', '*.hpp', '*.hxx', '*.c', '*.cc', '*.cpp', '*.cxx', '*.m', '*.mm' -File | Select-Object -ExpandProperty FullName
    & $ClangFormatCommand -i $AllSourceCodeFiles
} Finally {
    Pop-Location
}
