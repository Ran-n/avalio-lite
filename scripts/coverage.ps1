# Runs the default test suite under coverage and fails when line coverage drops below the gate.
#   .\scripts\coverage.ps1             the gate, listing each file's uncovered lines
#   .\scripts\coverage.ps1 -Branches   also branch coverage (needs the nightly toolchain:
#                                      rustup toolchain install nightly --component llvm-tools-preview)
# Needs cargo-llvm-cov (cargo install cargo-llvm-cov; rustup component add llvm-tools-preview).
# Raise MinLines as coverage grows; never lower it.
#
# Lines left uncovered on purpose (everything else the report lists needs a test):
#   src\main.rs, src\app\mod.rs run(), src\app\dialogs.rs NativeDialogs
#       the real exe and its Windows dialogs; every other test swaps in fake dialogs. Covered by the
#       desktop test the_real_dialogs_open_and_close, which isn't part of this measurement.
#   src\app\language.rs, the error log of select_bundled_translation
#       the translations are compiled into the exe for every language the picker offers.
#   src\app\state\mod.rs, "did not close cleanly" (twice); src\backup.rs, "the backup copy did not
#   close cleanly"
#       SQLite only refuses to close with statements still open, and the app never keeps one.
#   src\db\mod.rs discard, "could not remove"; src\spreadsheet\export.rs "could not remove the
#   incomplete export"
#       a cleanup failing right after an earlier failure (a double fault), on a file the app
#       created a moment before.
#   src\app\state\protection.rs, reopening fails after the files were swapped (and the
#   lock_screen.rs and src\app\backup.rs restore branches that then ask for the PIN again)
#       the swapped-in file passed its check a moment earlier; only another program grabbing it in
#       between can make it fail.
param([switch]$Branches)
$MinLines = 98
# checked with -Branches only (release.ps1 passes it)
$MinBranches = 92

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\use-perl.ps1"

# branch data needs the nightly toolchain; asking for it without one is an error, never a skip
if ($Branches -and -not (rustup toolchain list | Select-String -Quiet '^nightly')) {
    throw 'branch coverage needs the nightly toolchain: rustup toolchain install nightly --component llvm-tools-preview'
}
# RUSTUP_TOOLCHAIN outranks the rust-toolchain.toml pin, for this script only.
# Restored at the end, so a caller (release.ps1) builds the release with the pinned toolchain.
$previousToolchain = $env:RUSTUP_TOOLCHAIN
if ($Branches) { $env:RUSTUP_TOOLCHAIN = 'nightly' }
try {
    # @(...) keeps it an array: an if expression unwraps a one-item array into a string, which
    # splatting would then pass one character at a time
    $flags = @(if ($Branches) { '--branch' })

    # A file reported with more executable lines than it has means the coverage build kept stale
    # instrumented code from an earlier build, and the numbers mix two versions of the source.
    function Find-StaleFile {
        $summary = cargo llvm-cov report @flags --json --summary-only | ConvertFrom-Json
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
        cargo llvm-cov @flags --no-report
        if ($LASTEXITCODE -ne 0) { throw 'tests failed' }
        $stale = Find-StaleFile
        if (-not $stale) { break }
        if ($attempt -eq 2) { throw "coverage data is still inconsistent after a clean build: $stale" }
        Write-Warning "stale coverage build ($stale); cleaning it and measuring again"
        cargo llvm-cov clean
        if ($LASTEXITCODE -ne 0) { throw 'cargo llvm-cov clean failed' }
    }

    cargo llvm-cov report @flags --show-missing-lines --fail-under-lines $MinLines
    if ($LASTEXITCODE -ne 0) { throw "line coverage is under $MinLines %" }

    # cargo-llvm-cov can only fail on lines, functions and regions, so the branch total is read here
    if ($Branches) {
        $summary = cargo llvm-cov report @flags --json --summary-only | ConvertFrom-Json
        if ($LASTEXITCODE -ne 0) { throw 'could not read the branch coverage' }
        $branchPercent = [math]::Round($summary.data[0].totals.branches.percent, 2)
        if ($branchPercent -lt $MinBranches) { throw "branch coverage is $branchPercent %, under the gate of $MinBranches %" }
        Write-Host "branch coverage $branchPercent % (gate $MinBranches %)"
    }
} finally {
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
