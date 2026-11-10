# Собирает релиз RenPy Studio: exe, установщик Inno Setup, портативный zip и контрольные суммы.
#
#   powershell -File scripts\build-release.ps1                  # вся цепочка
#   powershell -File scripts\build-release.ps1 -SkipBuild       # только установщик из готового exe
#   powershell -File scripts\build-release.ps1 -Version 1.0.0   # поднять версию и собрать
#
# Результат: studio\dist-release\
#   RenPyStudio-<версия>-setup.exe     — установщик (per-user, WebView2, ассоциация .rpyproj)
#   RenPyStudio-<версия>-portable.zip  — exe + README без установки
#   SHA256SUMS.txt                     — контрольные суммы для релиза на GitHub

param(
    [string]$Version = "",
    [switch]$SkipBuild,
    [switch]$SkipInstaller,
    [switch]$SkipPortable
)

$ErrorActionPreference = "Stop"
$studio = Split-Path $PSScriptRoot -Parent
$outDir = Join-Path $studio "dist-release"
$exe = Join-Path $studio "src-tauri\target\release\renpy-studio.exe"
$webviewCache = Join-Path $studio "installer\cache\MicrosoftEdgeWebview2Setup.exe"

function Find-Iscc {
    # В этом окружении переменная ProgramFiles(x86) может быть не определена,
    # поэтому ищем по известным путям, PATH и записи в реестре.
    foreach ($path in @(
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe"
    )) {
        if (Test-Path $path) { return $path }
    }
    $found = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    foreach ($key in @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1",
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1"
    )) {
        $location = (Get-ItemProperty $key -ErrorAction SilentlyContinue).InstallLocation
        if ($location) {
            $candidate = Join-Path $location "ISCC.exe"
            if (Test-Path $candidate) { return $candidate }
        }
    }
    return $null
}

if (-not $Version) {
    $Version = (Get-Content (Join-Path $studio "package.json") -Raw | ConvertFrom-Json).version
}
Write-Host "== RenPy Studio $Version ==" -ForegroundColor Cyan

Write-Host "== версия во всех файлах ==" -ForegroundColor Cyan
Push-Location $studio
try {
    node scripts\set-version.mjs $Version
    if ($LASTEXITCODE -ne 0) { throw "set-version.mjs завершился с кодом $LASTEXITCODE" }
} finally {
    Pop-Location
}

if (-not $SkipBuild) {
    Write-Host "== сборка exe (tauri build --no-bundle) ==" -ForegroundColor Cyan
    Push-Location $studio
    try {
        powershell -NoProfile -ExecutionPolicy Bypass -File scripts\tauri.ps1 build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "tauri build завершился с кодом $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $exe)) { throw "Не найден $exe — сначала соберите exe" }
$exeVersion = (Get-Item $exe).VersionInfo.ProductVersion
Write-Host ("exe: {0} ({1:N0} байт, версия {2})" -f $exe, (Get-Item $exe).Length, $exeVersion)

New-Item -ItemType Directory -Path $outDir -Force | Out-Null

if (-not $SkipInstaller) {
    $iscc = Find-Iscc
    if (-not $iscc) { throw "Не найден Inno Setup 6 (ISCC.exe): установите Inno Setup или укажите путь в PATH" }

    # WebView2 bootstrapper кладём рядом один раз: установщик ставит его только
    # если рантайма нет (Win11 и большинство Win10 его уже содержат).
    if (-not (Test-Path $webviewCache)) {
        Write-Host "== загрузка WebView2 bootstrapper ==" -ForegroundColor Cyan
        New-Item -ItemType Directory -Path (Split-Path $webviewCache) -Force | Out-Null
        Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $webviewCache -UseBasicParsing
    }
    Write-Host ("WebView2 bootstrapper: {0:N0} байт" -f (Get-Item $webviewCache).Length)

    Write-Host "== установщик Inno Setup ==" -ForegroundColor Cyan
    & $iscc "/DAppVersion=$Version" "/DSourceExe=$exe" "/DOutputDir=$outDir" (Join-Path $studio "installer\renpy-studio.iss")
    if ($LASTEXITCODE -ne 0) { throw "ISCC завершился с кодом $LASTEXITCODE" }
}

if (-not $SkipPortable) {
    Write-Host "== портативная версия ==" -ForegroundColor Cyan
    $stage = Join-Path $outDir "portable"
    if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item $exe (Join-Path $stage "RenPyStudio.exe")
    Copy-Item (Join-Path $studio "README.md") (Join-Path $stage "README.md")
    $zip = Join-Path $outDir "RenPyStudio-$Version-portable.zip"
    if (Test-Path $zip) { Remove-Item -Force $zip }
    Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -CompressionLevel Optimal
    Remove-Item -Recurse -Force $stage
}

Write-Host "== SHA256 ==" -ForegroundColor Cyan
$sums = Join-Path $outDir "SHA256SUMS.txt"
$lines = Get-ChildItem $outDir -File | Where-Object { $_.Name -ne "SHA256SUMS.txt" } | ForEach-Object {
    $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
    "{0}  {1}" -f $hash, $_.Name
}
$lines | Set-Content -Path $sums -Encoding ascii
$lines

Write-Host ""
Write-Host "Готово: $outDir" -ForegroundColor Green
Get-ChildItem $outDir -File | Select-Object Name, @{n = "MB"; e = { [math]::Round($_.Length / 1MB, 2) } } | Format-Table -AutoSize
