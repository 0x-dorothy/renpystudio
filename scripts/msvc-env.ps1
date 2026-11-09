# Imports the Visual Studio + Windows SDK environment into the current process.
# Dot-source this file, then call Import-MsvcEnv.
#
# This machine keeps Visual Studio on a non-default drive and the primary
# install lacks the full C++ toolset, so we locate a complete MSVC toolset and
# add its headers/libs explicitly instead of trusting VsDevCmd alone.

function Find-VsInstall {
    $vswherePaths = @(
        "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
        "C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe"
    )
    foreach ($vswhere in $vswherePaths) {
        if (Test-Path $vswhere) {
            $path = & $vswhere -latest -products * -property installationPath 2>$null
            if ($path) {
                $first = @($path)[0]
                if ($first) { return $first.ToString().Trim() }
            }
        }
    }
    foreach ($root in @("C:\Program Files", "C:\Program Files (x86)", "D:\", "E:\")) {
        if (-not (Test-Path $root)) { continue }
        $hit = Get-ChildItem -Path $root -Filter "VsDevCmd.bat" -Recurse -Depth 5 -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like "*Common7\Tools*" } |
            Select-Object -First 1
        if ($hit) {
            return (Split-Path (Split-Path (Split-Path $hit.FullName -Parent) -Parent) -Parent)
        }
    }
    return $null
}

function Test-MsvcComplete($installPath) {
    if (-not $installPath) { return $null }
    $tools = Join-Path $installPath "VC\Tools\MSVC"
    if (-not (Test-Path $tools)) { return $null }
    foreach ($candidate in (Get-ChildItem $tools -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending)) {
        if ((Test-Path (Join-Path $candidate.FullName "include\excpt.h")) -and
            (Test-Path (Join-Path $candidate.FullName "lib\x64\msvcrt.lib"))) {
            return $candidate.FullName
        }
    }
    return $null
}

function Import-MsvcEnv {
    $vs = Find-VsInstall
    $candidates = @()
    if ($vs) { $candidates += $vs }
    $candidates += @(
        "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools",
        "C:\Program Files\Microsoft Visual Studio\2022\Community",
        "C:\Program Files\Microsoft Visual Studio\2022\Professional",
        "C:\Program Files\Microsoft Visual Studio\2022\Enterprise"
    )

    $msvc = $null
    foreach ($candidate in $candidates) {
        $found = Test-MsvcComplete $candidate
        if ($found) {
            $vs = $candidate
            $msvc = $found
            break
        }
    }
    if (-not $vs) {
        throw "Visual Studio / Build Tools with the C++ workload was not found. Install 'Desktop development with C++'."
    }
    if (-not $msvc) {
        throw "No complete MSVC x64 toolset (headers + libs) was found. Install the 'Desktop development with C++' workload."
    }

    $devCmd = Join-Path $vs "Common7\Tools\VsDevCmd.bat"
    if (Test-Path $devCmd) {
        $envDump = cmd.exe /c "`"$devCmd`" -arch=x64 -host_arch=x64 >nul 2>&1 && set"
        foreach ($line in $envDump) {
            if ($line -match '^([^=]+)=(.*)$') {
                [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
            }
        }
    }

    $env:PATH = "$(Join-Path $msvc 'bin\Hostx64\x64');$env:PATH"

    $includeParts = @((Join-Path $msvc "include"))
    $libParts = @((Join-Path $msvc "lib\x64"))

    $kitsRoot = "C:\Program Files (x86)\Windows Kits\10"
    if (-not (Test-Path $kitsRoot)) {
        $kitsRoot = (Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots" -ErrorAction SilentlyContinue).KitsRoot10
    }
    if ($kitsRoot -and (Test-Path $kitsRoot)) {
        $sdkVersionDir = Get-ChildItem (Join-Path $kitsRoot "Include") -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending | Select-Object -First 1
        if ($sdkVersionDir) {
            $ver = $sdkVersionDir.Name
            foreach ($part in @("ucrt", "um", "shared", "winrt", "cppwinrt")) {
                $inc = Join-Path $kitsRoot "Include\$ver\$part"
                if (Test-Path $inc) { $includeParts += $inc }
            }
            foreach ($part in @("ucrt", "um")) {
                $lib = Join-Path $kitsRoot "Lib\$ver\$part\x64"
                if (Test-Path $lib) { $libParts += $lib }
            }
        }
    }

    if ($env:INCLUDE) { $includeParts += $env:INCLUDE }
    if ($env:LIB) { $libParts += $env:LIB }
    $env:INCLUDE = ($includeParts -join ";")
    $env:LIB = ($libParts -join ";")

    Write-Host "MSVC: $msvc" -ForegroundColor DarkGray
    Write-Host "SDK : $kitsRoot" -ForegroundColor DarkGray
}
