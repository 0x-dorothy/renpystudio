# End-to-end check: create a project, export the fixture story, run Ren'Py lint.
param(
    [string]$Sdk = "E:\projai\RenPy Studio"
)

$ErrorActionPreference = "Stop"
$studio = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $studio "src-tauri\target\debug\renpy-studio.exe"
if (-not (Test-Path $exe)) {
    Write-Error "Build the app first: scripts\cargo.ps1 build --features cli"
}

$sandbox = Join-Path $studio ".e2e"
if (Test-Path $sandbox) { Remove-Item -Recurse -Force $sandbox }
New-Item -ItemType Directory -Path $sandbox | Out-Null
$project = Join-Path $sandbox "novella"

Write-Host "== new project ==" -ForegroundColor Cyan
& $exe --new $project "Тестовая новелла" --sdk $Sdk
if ($LASTEXITCODE -ne 0) { Write-Error "project creation failed" }

Copy-Item (Join-Path $studio "tests\fixture\project.json") (Join-Path $project "renpy-studio\project.json") -Force

Write-Host "== fixture assets ==" -ForegroundColor Cyan
python (Join-Path $studio "scripts\make-fixture-assets.py") $project $Sdk
if ($LASTEXITCODE -ne 0) { Write-Error "fixture assets failed" }

# A couple of placeholder assets so the scaffold mirrors a real project.
$bgDir = Join-Path $project "game\images\bg"
New-Item -ItemType Directory -Path $bgDir -Force | Out-Null
$spriteDir = Join-Path $project "game\images\sprites"
New-Item -ItemType Directory -Path $spriteDir -Force | Out-Null

Write-Host "== export ==" -ForegroundColor Cyan
& $exe --export $project
if ($LASTEXITCODE -ne 0) { Write-Error "export failed" }

Write-Host "== generated files ==" -ForegroundColor Cyan
Get-ChildItem (Join-Path $project "game") -Filter "*.rpy" | Select-Object -ExpandProperty Name

Write-Host "== renpy lint ==" -ForegroundColor Cyan
Push-Location $Sdk
try {
    $lintReport = Join-Path $project "renpy-studio\lint-report.txt"
    & (Join-Path $Sdk "renpy.exe") $project lint $lintReport --error-code --all-problems | Out-Null
    $lintExit = $LASTEXITCODE
} finally {
    Pop-Location
}
Write-Host "lint exit code: $lintExit"
if (Test-Path $lintReport) {
    Get-Content $lintReport
}
