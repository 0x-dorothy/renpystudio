# Runs the Tauri CLI with the MSVC environment imported.
# Usage: powershell -File scripts/tauri.ps1 dev
#        powershell -File scripts/tauri.ps1 build
$ErrorActionPreference = "Stop"

$command = @($args)
if ($command.Count -eq 0) { $command = @("build") }

. "$PSScriptRoot\msvc-env.ps1"
Import-MsvcEnv

Push-Location (Join-Path $PSScriptRoot "..")
try {
    & npx tauri @command
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
