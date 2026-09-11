$Script = Join-Path $PSScriptRoot 'app\windows\build.ps1'
& $Script @args
exit $LASTEXITCODE
