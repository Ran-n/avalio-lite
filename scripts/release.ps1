# Builds both release artifacts into dist\, named with the version from Cargo.toml:
#   avalio-lite-<version>-portable.exe   the app on its own, runs from any folder
#   avalio-lite-<version>-setup.exe      Inno Setup installer (Start menu, uninstall entry)
# No global ErrorActionPreference=Stop: on Windows PowerShell 5.1 it turns cargo's progress
# output on stderr into a failure whenever the script's output is redirected. Native tools are
# checked through $LASTEXITCODE instead, cmdlets through -ErrorAction Stop.
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version
Write-Host "Releasing avalio-lite $version"

. "$PSScriptRoot\use-perl.ps1"

# the default suite under its coverage gate, then the desktop tests, which open the real window
& "$PSScriptRoot\coverage.ps1"
cargo test --test desktop -- --ignored --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'desktop tests failed' }
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'release build failed' }

$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force $dist -ErrorAction Stop | Out-Null
$exe = Join-Path $root 'target\release\avalio-lite.exe'
Copy-Item $exe (Join-Path $dist "avalio-lite-$version-portable.exe") -Force -ErrorAction Stop

$iscc = @(
    (Get-Command iscc -ErrorAction SilentlyContinue).Source,
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 (ISCC.exe) not found; install it from https://jrsoftware.org/isinfo.php' }

& $iscc /Q "/DAppVersion=$version" "/DSourceExe=$exe" (Join-Path $root 'installer\avalio-lite.iss')
if ($LASTEXITCODE -ne 0) { throw 'installer build failed' }

Get-ChildItem $dist -Filter "avalio-lite-$version-*" | Format-Table Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
