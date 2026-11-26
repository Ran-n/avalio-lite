# Builds the Windows setup (installer) into a folder, named by avalio-lite-asset-name, with the version
# from Cargo.toml:
#   packaging\windows\build-installer.ps1
#   packaging\windows\build-installer.ps1 -Binary target\release\avalio-lite.exe -OutputDirectory dist
# The setup installs that exe per user, under the name avalio-lite.exe; it is the same binary as the
# portable release. Needs Inno Setup 6: https://jrsoftware.org/isinfo.php (winget install JRSoftware.InnoSetup).
# Relative paths are taken from the repository root.

param(
    [string]$Binary = 'target\release\avalio-lite.exe',
    [string]$OutputDirectory = 'dist'
)

. "$PSScriptRoot\..\..\scripts\release-common.ps1"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root

$exe = Resolve-FullPath -Root $root -Path $Binary
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "no exe at $exe; build it first: cargo build --release --locked"
}

$version = Get-AppVersion
$name = Get-AssetName -Form setup
$outDir = Resolve-FullPath -Root $root -Path $OutputDirectory
New-Item -ItemType Directory -Force -Path $outDir -ErrorAction Stop | Out-Null

# ISCC from PATH, else where the Inno Setup installer puts it
$iscc = $null
$onPath = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if ($onPath) {
    $iscc = $onPath.Source
} else {
    $iscc = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    ) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
}
if (-not $iscc) {
    throw 'Inno Setup 6 (ISCC.exe) not found: install it from https://jrsoftware.org/isinfo.php (winget install JRSoftware.InnoSetup)'
}

Write-Host "Building $name (avalio-lite $version) with $iscc"
# ISCC adds .exe itself, so it gets the name without it
$outputName = [System.IO.Path]::GetFileNameWithoutExtension($name)
& $iscc /Q "/DAppVersion=$version" "/DOutputName=$outputName" "/DOutputDir=$outDir" "/DSourceExe=$exe" (Join-Path $PSScriptRoot 'avalio-lite.iss')
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed with exit code $LASTEXITCODE" }

$setup = Join-Path $outDir $name
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) {
    throw "Inno Setup finished, but $setup isn't there"
}
Write-Host "Installer: $setup"
