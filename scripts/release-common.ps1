# Shared by the Windows release scripts (packaging/windows, scripts/release.ps1, scripts/release-check.ps1).
# The version comes from Cargo.toml and every asset name from avalio-lite-asset-name, never from a name
# spelled out here. Dot-source it: . "$PSScriptRoot\release-common.ps1"
# Windows PowerShell 5.1 compatible. No global ErrorActionPreference=Stop: cargo writes progress to
# stderr, which 5.1 turns into a failure whenever output is redirected. Native tools are checked
# through $LASTEXITCODE.

# The one target a release is built for; the updater looks for exactly this in asset names.
$AvalioTarget = 'windows-x86_64'

function Get-AppVersion {
    # The version in Cargo.toml, as cargo reads it.
    $metadata = & cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed; run the script from the repository' }
    $package = $metadata.packages | Where-Object { $_.name -eq 'avalio-lite' } | Select-Object -First 1
    if (-not $package) { throw 'cargo metadata lists no avalio-lite package' }
    return [string]$package.version
}

function Get-AssetName {
    # The name the updater looks for, printed by avalio-lite-asset-name (src/bin/avalio-lite-asset-name.rs).
    param(
        [Parameter(Mandatory = $true)][ValidateSet('portable', 'setup')][string]$Form,
        [switch]$Bridge
    )
    $arguments = @('run', '--release', '--quiet', '--bin', 'avalio-lite-asset-name', '--', $AvalioTarget, $Form)
    if ($Bridge) { $arguments += '--bridge' }
    $output = & cargo @arguments
    if ($LASTEXITCODE -ne 0) { throw "avalio-lite-asset-name failed for $Form$(if ($Bridge) { ' (bridge)' })" }
    $name = @($output) | Select-Object -Last 1
    if (-not $name) { throw 'avalio-lite-asset-name printed no name' }
    return ([string]$name).Trim()
}

function Resolve-FullPath {
    # A path as given; a relative one is taken from $Root.
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Path
    )
    if ([System.IO.Path]::IsPathRooted($Path)) { return [System.IO.Path]::GetFullPath($Path) }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Path))
}

function Assert-InsideDirectory {
    # Throws unless $Path is strictly inside $Directory, so no script deletes anything outside it.
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Directory
    )
    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
    $fullDirectory = [System.IO.Path]::GetFullPath($Directory).TrimEnd('\') + '\'
    if (-not $fullPath.StartsWith($fullDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to touch ${fullPath}: it is not inside $Directory"
    }
}
