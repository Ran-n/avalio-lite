# What to free when Codeberg refuses a release file for the account's quota: the decisions only, so
# tests/release_scripts.rs can run them on sample answers. scripts/release.ps1 makes the calls.
# The rule: only this repository's files go, oldest release first and one file at a time, only until
# the missing uploads fit; the release being made keeps every file; each release keeps its tag and
# notes, which gain a line pointing to the same tag on GitHub, where every file stays. If even every
# older file wouldn't make room, nothing is deleted.
# Dot-source it: . "$PSScriptRoot\codeberg-quota.ps1". Windows PowerShell 5.1 compatible.

function Get-CodebergShortfall {
    # The bytes to free before $ToUpload more bytes fit: what release files use, less the limit, plus
    # the uploads. Zero or less means they fit. $Quota is GET /api/v1/user/quota's answer.
    param(
        [Parameter(Mandatory = $true)]$Quota,
        [Parameter(Mandatory = $true)][long]$ToUpload
    )
    $used = [long]$Quota.used.size.assets.attachments.releases
    $rule = @($Quota.groups | ForEach-Object { $_.rules } | Where-Object { $_.subjects -contains 'size:assets:all' }) |
        Select-Object -First 1
    if (-not $rule) { throw 'the Codeberg quota answer has no limit for release files (size:assets:all)' }
    return $used - [long]$rule.limit + $ToUpload
}

function Select-CodebergFilesToFree {
    # The files to delete, in order: the oldest release's first, each release's in upload order,
    # stopping at the first that frees $Needed bytes. $Releases is the repository's release list;
    # $KeepTag is the release being made. Throws, choosing nothing, when all older files fall short.
    param(
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$Releases,
        [Parameter(Mandatory = $true)][string]$KeepTag,
        [Parameter(Mandatory = $true)][long]$Needed
    )
    $chosen = @()
    if ($Needed -le 0) { return , $chosen }
    $older = @($Releases | Where-Object { $_.tag_name -ne $KeepTag } | Sort-Object { [datetime]$_.created_at })
    $freed = [long]0
    foreach ($release in $older) {
        foreach ($asset in @($release.assets | Sort-Object { [long]$_.id })) {
            if ($freed -ge $Needed) { break }
            $chosen += [pscustomobject]@{
                ReleaseId = $release.id
                Tag = $release.tag_name
                AssetId = $asset.id
                Name = $asset.name
                Size = [long]$asset.size
            }
            $freed += [long]$asset.size
        }
    }
    if ($freed -lt $Needed) {
        throw "Codeberg needs $Needed more bytes, but this repository's older release files hold only $freed; nothing was deleted"
    }
    return , $chosen
}

function Get-NotesWithGitHubPointer {
    # A stripped release's notes with the line that sends readers to GitHub, added once.
    param(
        [AllowEmptyString()][string]$Body,
        [Parameter(Mandatory = $true)][string]$Tag,
        [Parameter(Mandatory = $true)][string]$GitHubRepo
    )
    $line = "Some files of this release were removed from Codeberg for space; every file is on GitHub: https://github.com/$GitHubRepo/releases/tag/$Tag"
    if ($Body -and $Body.Contains($line)) { return $Body }
    if (-not $Body -or -not $Body.Trim()) { return $line + "`n" }
    return $Body.TrimEnd() + "`n`n" + $line + "`n"
}
