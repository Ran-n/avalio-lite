# Runs the default test suite under coverage and fails when line coverage drops below the gate.
#   .\scripts\coverage.ps1             the gate, listing each file's uncovered lines
#   .\scripts\coverage.ps1 -Branches   also branch coverage (needs the nightly toolchain:
#                                      rustup toolchain install nightly --component llvm-tools-preview)
# Needs cargo-llvm-cov (cargo install cargo-llvm-cov; rustup component add llvm-tools-preview).
# Raise MinLines as coverage grows; never lower it.
param([switch]$Branches)
$MinLines = 96

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\use-perl.ps1"

$toolchain = if ($Branches) { @('+nightly') } else { @() }
$flags = if ($Branches) { @('--branch') } else { @() }

# A file reported with more executable lines than it has means the coverage build kept stale
# instrumented code from an earlier build, and the numbers mix two versions of the source.
function Find-StaleFile {
    $summary = cargo @toolchain llvm-cov report @flags --json --summary-only | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'could not read the coverage report' }
    foreach ($file in $summary.data[0].files) {
        $physical = @(Get-Content -LiteralPath $file.filename).Count
        if ($file.summary.lines.count -gt $physical) {
            return "$($file.filename) reports $($file.summary.lines.count) executable lines but has $physical"
        }
    }
    return $null
}

foreach ($attempt in 1, 2) {
    cargo @toolchain llvm-cov @flags --no-report
    if ($LASTEXITCODE -ne 0) { throw 'tests failed' }
    $stale = Find-StaleFile
    if (-not $stale) { break }
    if ($attempt -eq 2) { throw "coverage data is still inconsistent after a clean build: $stale" }
    Write-Warning "stale coverage build ($stale); cleaning it and measuring again"
    cargo @toolchain llvm-cov clean
    if ($LASTEXITCODE -ne 0) { throw 'cargo llvm-cov clean failed' }
}

cargo @toolchain llvm-cov report @flags --show-missing-lines --fail-under-lines $MinLines
if ($LASTEXITCODE -ne 0) { throw "line coverage is under $MinLines %" }
