# The test gate before a release is tagged: Clippy and rustfmt, then the default suite under its line and branch coverage
# gates, then the desktop tests, which open the real window. Run it on the commit to release, before
# bumping the version and tagging (docs/development.md, Releases).
#   .\scripts\release-check.ps1

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

. "$PSScriptRoot\use-perl.ps1"

cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'clippy found problems' }
cargo fmt --check
if ($LASTEXITCODE -ne 0) { throw 'the code is not formatted; run cargo fmt' }

& "$PSScriptRoot\coverage.ps1"
& "$PSScriptRoot\coverage.ps1" -Branches
cargo test --test desktop -- --ignored --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'desktop tests failed' }

Write-Host 'Release checks passed. Next: bump the version, update CHANGELOG.md, tag vX.Y.Z and push.'
