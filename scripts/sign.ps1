# Signs the built installer with a Windows code-signing certificate.
#
# The certificate is not part of the repository: pass either a .pfx file or the
# thumbprint of a certificate installed in the current user's store.
#
#   powershell -File scripts\sign.ps1 -Pfx C:\certs\renpystudio.pfx -Password "***"
#   powershell -File scripts\sign.ps1 -Thumbprint "AB12...EF"
#
# A trusted certificate (OV/EV) is required to remove the SmartScreen warning;
# a self-signed one only helps with local testing.

param(
    [string]$Installer = "",
    [string]$Pfx = "",
    [string]$Password = "",
    [string]$Thumbprint = "",
    [string]$Timestamp = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$studio = Split-Path $PSScriptRoot -Parent

if (-not $Installer) {
    # Сначала релизная сборка (Inno), потом NSIS из tauri build.
    $dirs = @(
        (Join-Path $studio "dist-release"),
        (Join-Path $studio "src-tauri\target\release\bundle\nsis")
    )
    $latest = $null
    foreach ($dir in $dirs) {
        $candidate = Get-ChildItem $dir -Filter "*-setup.exe" -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1
        if ($candidate) { $latest = $candidate; break }
    }
    if (-not $latest) {
        Write-Error "Установщик не найден. Сначала: powershell -File scripts\build-release.ps1"
    }
    $Installer = $latest.FullName
}

if (-not (Test-Path $Installer)) {
    Write-Error "Файл не найден: $Installer"
}

$signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Directory -ErrorAction SilentlyContinue |
    Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName "x64\signtool.exe" } |
    Where-Object { Test-Path $_ } |
    Select-Object -First 1
if (-not $signtool) {
    Write-Error "signtool.exe не найден. Установите Windows SDK (Signing Tools)."
}

if ($Pfx) {
    if (-not (Test-Path $Pfx)) { Write-Error "PFX не найден: $Pfx" }
    & $signtool sign /f $Pfx /p $Password /fd SHA256 /tr $Timestamp /td SHA256 $Installer
} elseif ($Thumbprint) {
    & $signtool sign /sha1 $Thumbprint /fd SHA256 /tr $Timestamp /td SHA256 $Installer
} else {
    Write-Host "Сертификат не задан — подписывать нечем." -ForegroundColor Yellow
    Write-Host "Передайте -Pfx <файл> -Password <пароль> или -Thumbprint <отпечаток>." -ForegroundColor Yellow
    Write-Host "Без доверенного сертификата SmartScreen продолжит предупреждать." -ForegroundColor Yellow
    exit 1
}

if ($LASTEXITCODE -ne 0) {
    Write-Error "signtool завершился с кодом $LASTEXITCODE"
}
& $signtool verify /pa $Installer
Write-Host "Подписано: $Installer" -ForegroundColor Green
