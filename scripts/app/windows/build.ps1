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
$OutputDir = Join-Path $ProjectRoot 'dist\windows'
$BundleLog = Join-Path $CargoTarget 'windows-bundle.log'

foreach ($Command in @('cargo', $DxTool)) {
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $Command"
    }
}

$GeneratedPaths = @(
    (Join-Path $CargoTarget 'dx\exchange-data\release\windows'),
    (Join-Path $CargoTarget 'dx\exchange-data\bundle\windows'),
    $OutputDir
)
foreach ($Path in $GeneratedPaths) {
    if (Test-Path -LiteralPath $Path) { Remove-Item -LiteralPath $Path -Recurse -Force }
}

Write-Host '==> Building Windows desktop application'
if (Test-Path -LiteralPath $BundleLog) { Clear-Content -LiteralPath $BundleLog }
Push-Location (Join-Path $ProjectRoot 'apps\exchange-ui')
try {
    & $DxTool bundle --windows --release --no-default-features --features desktop `
        --cargo-args=--locked --package-types nsis --out-dir $OutputDir --log-to-file $BundleLog
    $BundleExitCode = $LASTEXITCODE
    if ($BundleExitCode -ne 0) {
        $EncodingFailure = (Test-Path -LiteralPath $BundleLog) -and
            (Select-String -LiteralPath $BundleLog -SimpleMatch 'Bad text encoding:' -Quiet)
        if (-not $EncodingFailure) { throw "Dioxus bundle failed with exit code $BundleExitCode" }

        # Dioxus 0.7.10 writes UTF-8 without a BOM, but invokes NSIS using the system code page.
        $NsisDir = Join-Path $CargoTarget 'dx\exchange-data\bundle\windows\nsis'
        $NsisScripts = @(Get-ChildItem -LiteralPath $NsisDir -Filter '*.nsi' -File)
        if ($NsisScripts.Count -ne 1) { throw "Expected one generated NSIS script in $NsisDir" }
        $DxHome = if ($env:DX_HOME) { $env:DX_HOME } else { Join-Path $env:USERPROFILE '.dx' }
        $MakeNsis = Join-Path $DxHome 'tools\nsis-3.11\makensis.exe'
        if (-not (Test-Path -LiteralPath $MakeNsis -PathType Leaf)) {
            throw "Dioxus NSIS compiler not found: $MakeNsis"
        }

        Write-Host '==> Retrying NSIS with UTF-8 input encoding'
        & $MakeNsis /INPUTCHARSET UTF8 /NOCD $NsisScripts[0].FullName
        if ($LASTEXITCODE -ne 0) { throw "UTF-8 NSIS bundle failed with exit code $LASTEXITCODE" }

        $NsisInstallers = @(Get-ChildItem -LiteralPath $NsisDir -Filter '*-setup.exe' -File |
            Where-Object Length -GT 0)
        if ($NsisInstallers.Count -ne 1) { throw "Expected one non-empty NSIS installer in $NsisDir" }
        New-Item -ItemType Directory -Force $OutputDir | Out-Null
        Copy-Item -LiteralPath $NsisInstallers[0].FullName -Destination $OutputDir -Force
    }
} finally {
    Pop-Location
}

$Installers = @(Get-ChildItem -LiteralPath $OutputDir -Filter '*.exe' -File -Recurse |
    Where-Object Length -GT 0)
if ($Installers.Count -eq 0) { throw 'Dioxus did not produce a non-empty Windows installer.' }

foreach ($Installer in $Installers) {
    $Hash = Get-FileHash -LiteralPath $Installer.FullName -Algorithm SHA256
    Write-Host "Installer: $($Installer.FullName)"
    Write-Host "Size: $($Installer.Length) bytes"
    Write-Host "SHA256: $($Hash.Hash)"
}
