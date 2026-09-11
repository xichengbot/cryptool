[CmdletBinding()]
param([switch]$Help)

$ErrorActionPreference = 'Stop'

if ($Help) {
    @'
Usage: .\scripts\app\windows\build.ps1 [-Help]

Build the Windows desktop application and create an NSIS installer in dist\windows.
'@
    exit 0
}

if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'Windows desktop builds must run on Windows.'
}

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$DxTool = if ($env:DX) { $env:DX } else { 'dx' }
$CargoTarget = if ($env:CARGO_TARGET_DIR) {
    if ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    } else {
        [IO.Path]::GetFullPath((Join-Path $ProjectRoot $env:CARGO_TARGET_DIR))
    }
} else {
    Join-Path $ProjectRoot 'target'
}
New-Item -ItemType Directory -Force $CargoTarget | Out-Null
$env:CARGO_TARGET_DIR = $CargoTarget

foreach ($Command in @('cargo', $DxTool)) {
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $Command"
    }
}

$GeneratedPaths = @(
    (Join-Path $CargoTarget 'dx\exchange-data\release\windows'),
    (Join-Path $CargoTarget 'dx\exchange-data\bundle\windows'),
    (Join-Path $ProjectRoot 'dist\windows')
)
foreach ($Path in $GeneratedPaths) {
    if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Recurse -Force }
}

Write-Host '==> Building Windows desktop application'
Push-Location (Join-Path $ProjectRoot 'apps\exchange-ui')
try {
    & $DxTool bundle --windows --release --no-default-features --features desktop `
        --cargo-args=--locked --package-types nsis --out-dir (Join-Path $ProjectRoot 'dist\windows')
    if ($LASTEXITCODE -ne 0) { throw "Dioxus bundle failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}

$Installers = @(Get-ChildItem (Join-Path $ProjectRoot 'dist\windows') -Filter '*.exe' -File -Recurse |
    Where-Object Length -GT 0)
if ($Installers.Count -eq 0) { throw 'Dioxus did not produce a non-empty Windows installer.' }

foreach ($Installer in $Installers) {
    $Hash = Get-FileHash -LiteralPath $Installer.FullName -Algorithm SHA256
    Write-Host "Installer: $($Installer.FullName)"
    Write-Host "Size: $($Installer.Length) bytes"
    Write-Host "SHA256: $($Hash.Hash)"
}
