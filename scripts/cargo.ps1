# Runs cargo with the MSVC environment imported.
# Usage: powershell -File scripts/cargo.ps1 check
#        powershell -File scripts/cargo.ps1 build --features cli
$ErrorActionPreference = "Stop"

$cargoArgs = @($args)
if ($cargoArgs.Count -eq 0) { $cargoArgs = @("check") }

. "$PSScriptRoot\msvc-env.ps1"
Import-MsvcEnv

Push-Location (Join-Path $PSScriptRoot "..\src-tauri")
try {
    & cargo @cargoArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
