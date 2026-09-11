[CmdletBinding()]
param(
    [string]$Installer,
    [switch]$Silent,
    [switch]$Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @'
Usage: .\scripts\app\windows\install.ps1 [-Installer PATH] [-Silent] [-Help]

Install the newest NSIS package from dist\windows, or the explicitly selected package.
'@
    exit 0
}

if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'The Windows desktop installer must run on Windows.'
}

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
if (-not $Installer) {
    $Candidate = Get-ChildItem (Join-Path $ProjectRoot 'dist\windows') -Filter '*.exe' -File -Recurse |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $Candidate) { throw 'No installer found under dist\windows; run build.ps1 first.' }
    $Installer = $Candidate.FullName
}
$Installer = (Resolve-Path -LiteralPath $Installer).Path

$Arguments = if ($Silent) { @('/S') } else { @() }
$Process = Start-Process -FilePath $Installer -ArgumentList $Arguments -Wait -PassThru
if ($Process.ExitCode -ne 0) { throw "Installer exited with code $($Process.ExitCode)" }
Write-Host "Installation completed: $Installer"
