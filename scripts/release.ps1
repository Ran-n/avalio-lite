# The maintainer's half of a release, run after the release workflow (.github/workflows/release.yml)
# has built the DRAFT GitHub release for the tag. In order:
#   1 downloads the draft's files            2 checks each .b3 and that the bridge copies match
#   3 checks each exe's build attestation    4 signs the four exes with the release key
#   5 verifies the four signatures           6 uploads the four signature files to the draft
#   7 creates the GitLab and Codeberg releases with all twelve files
#   8 publishes the GitHub draft             9 checks each forge lists every file at its size
# A file that fails a check is never signed. The signing key is GPG-encrypted in
# ~/.release-keys/avalio-lite-release-signing.key.gpg; gpg decrypts it straight into avalio-lite-sign
# through a pipe in Git Bash (PowerShell 5.1 would re-encode the bytes), so it is never written to a
# file or kept in a variable, and nothing is asked for.
#
#   .\scripts\release.ps1
#   .\scripts\release.ps1 -DryRun                the read-only steps run; the rest is printed
#   .\scripts\release.ps1 -FilesFrom <folder>    uses files already downloaded
#   .\scripts\release.ps1 -SigningKey <file>     another encrypted key file
#
# Needs: gpg holding the maintainer's key (the fingerprint on CONTRIBUTORS' GPG line), with no prompt;
# Git Bash; gh and glab, signed in; b3sum (cargo install b3sum --version 1.8.5 --locked); curl.exe;
# and, for Codeberg, the environment variable CODEBERG_TOKEN.

param(
    [string]$Version,
    [string]$SigningKey = (Join-Path $HOME '.release-keys\avalio-lite-release-signing.key.gpg'),
    [string]$GitBash = 'C:\Program Files\Git\bin\bash.exe',
    [string]$FilesFrom,
    [switch]$DryRun
)

$GitHubRepo = 'Ran-n/avalio-lite'
$GitLabRepo = 'Ran-n/avalio-lite'
$CodebergRepo = 'ran-n/avalio-lite'
$CodebergApi = "https://codeberg.org/api/v1/repos/$CodebergRepo"

. "$PSScriptRoot\use-perl.ps1"
. "$PSScriptRoot\release-common.ps1"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Step {
    param([string]$Title)
    Write-Host ''
    Write-Host "== $Title"
}

function Invoke-Change {
    # Runs an action that changes a forge or a signature. In -DryRun it prints what it would do.
    param([string]$Description, [scriptblock]$Action)
    if ($DryRun) {
        Write-Host "  dry run, not run: $Description"
        return
    }
    Write-Host "  $Description"
    & $Action
}

function Read-B3Hash {
    # The hash of a .b3 line: "<hash>  <name>"
    param([string]$Path)
    return ((Get-Content -LiteralPath $Path -TotalCount 1) -split '\s+')[0]
}

function Get-RemoteSize {
    # The Content-Length of a file's address after redirects. Only headers are asked for.
    param([string]$Url)
    $headers = & curl.exe -sSIL $Url
    if ($LASTEXITCODE -ne 0) { return $null }
    $size = $null
    foreach ($line in $headers) {
        if ($line -match '^content-length:\s*(\d+)') { $size = [long]$Matches[1] }
    }
    return $size
}

function Compare-Forge {
    # Returns the differences between what a forge lists and the files (name -> size); none when they match.
    param([hashtable]$Listed, [hashtable]$Expected)
    $problems = @()
    foreach ($name in $Expected.Keys) {
        if (-not $Listed.ContainsKey($name)) {
            $problems += "$name is missing"
        } elseif ($null -eq $Listed[$name]) {
            $problems += "$name has no size"
        } elseif ([long]$Listed[$name] -ne [long]$Expected[$name]) {
            $problems += "$name is $($Listed[$name]) bytes, expected $($Expected[$name])"
        }
    }
    foreach ($name in $Listed.Keys) {
        if (-not $Expected.ContainsKey($name)) { $problems += "$name is not one of the release's files" }
    }
    # callers wrap the call in @(), so an empty list gives zero items
    return $problems
}

function Invoke-Codeberg {
    # One Forgejo API call. The token goes in a header; the answer's body is returned, and a status
    # other than 2xx throws with that body.
    param([string]$Method, [string]$Url, [string]$JsonFile, [string]$Upload, [string]$ResponseFile)
    $curlArguments = @('-sS', '-X', $Method, '-H', "Authorization: token $env:CODEBERG_TOKEN", '-o', $ResponseFile, '-w', '%{http_code}')
    if ($JsonFile) { $curlArguments += @('-H', 'Content-Type: application/json', '--data-binary', "@$JsonFile") }
    if ($Upload) { $curlArguments += @('-F', "attachment=@$Upload") }
    $curlArguments += $Url
    $status = & curl.exe @curlArguments
    $body = Get-Content -LiteralPath $ResponseFile -Raw
    if ($LASTEXITCODE -ne 0 -or "$status" -notmatch '^2\d\d$') {
        throw "Codeberg answered '$status' to $Method $($Url -replace '\?.*$', ''): $body"
    }
    return $body
}

# --- checks before anything runs --------------------------------------------------------------

$manifestVersion = Get-AppVersion
if (-not $Version) { $Version = $manifestVersion }
if ($Version -ne $manifestVersion) {
    throw "-Version $Version isn't Cargo.toml's $manifestVersion; asset names come from Cargo.toml"
}
$tag = "v$Version"
$releaseTitle = "avalio lite $Version"

$tagCommit = & git rev-parse --verify --quiet "$tag^{commit}" 2>$null
$headCommit = & git rev-parse HEAD 2>$null
if (-not $tagCommit -or $tagCommit -ne $headCommit) {
    $message = "$tag isn't the commit checked out (or this isn't a git checkout): sign from the tag's checkout (git checkout $tag)"
    if ($DryRun) { Write-Warning $message } else { throw $message }
}

function Assert-Tool {
    param([string]$Name, [string]$Hint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) { throw "$Name not found: $Hint" }
}
Assert-Tool -Name gh -Hint 'install the GitHub CLI (https://cli.github.com) and run gh auth login'
Assert-Tool -Name glab -Hint 'install the GitLab CLI (https://gitlab.com/gitlab-org/cli) and run glab auth login'
Assert-Tool -Name b3sum -Hint 'cargo install b3sum --version 1.8.5 --locked'
Assert-Tool -Name 'curl.exe' -Hint 'Windows 10 and later ship it'
$b3sumVersion = (& b3sum --version) -join ' '
if ($b3sumVersion -notmatch '1.8.5') { throw "b3sum is '$b3sumVersion'; this script needs 1.8.5: cargo install b3sum --version 1.8.5 --locked" }

Assert-Tool -Name gpg -Hint 'install GnuPG (Git for Windows and Gpg4win both bring one) holding the maintainer''s key'
if (-not (Test-Path -LiteralPath $GitBash -PathType Leaf)) { throw "Git Bash not found at $GitBash; pass -GitBash" }
# checked in a dry run too, so a missing key shows before anything else
if (-not (Test-Path -LiteralPath $SigningKey -PathType Leaf)) {
    throw "no signing key at ${SigningKey}: the GPG-encrypted release key (docs/development.md, Signing keys); pass -SigningKey"
}
if (-not $DryRun) {
    if (-not $env:CODEBERG_TOKEN) { throw 'CODEBERG_TOKEN is not set: a Codeberg token that can create releases' }
}

function ConvertTo-BashWord {
    # A Windows path as one single-quoted Git Bash word: forward slashes, any ' escaped
    param([string]$Path)
    return "'" + ($Path.Replace('\', '/').Replace("'", "'\''")) + "'"
}

# the twelve files: four exes, their four .b3 sums and four signatures (a .minisig for each new name, the
# 0.10.0-format .sig for each bridge name); the names come from the helper
$portable = Get-AssetName -Form portable
$setup = Get-AssetName -Form setup
$portableBridge = Get-AssetName -Form portable -Bridge
$setupBridge = Get-AssetName -Form setup -Bridge
$exes = @($portable, $setup, $portableBridge, $setupBridge)
$b3Files = @($exes | ForEach-Object { "$_.b3" })
$sigFiles = @("$portable.minisig", "$setup.minisig", "$portableBridge.sig", "$setupBridge.sig")
$draftFiles = @($exes + $b3Files)
$allFiles = @($exes + $b3Files + $sigFiles)

$scratch = Join-Path $root "target\release-$Version"
Assert-InsideDirectory -Path $scratch -Directory (Join-Path $root 'target')
Write-Host "Releasing $releaseTitle ($tag)$(if ($DryRun) { ' as a dry run' })"

# --- 1 download ---------------------------------------------------------------------------------

Step '1. the draft release'
if ($FilesFrom) {
    $work = Resolve-FullPath -Root $root -Path $FilesFrom
    Write-Host "  using the files in $work; nothing is downloaded"
} else {
    $work = $scratch
    if (Test-Path -LiteralPath $work) { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction Stop }
    New-Item -ItemType Directory -Path $work -ErrorAction Stop | Out-Null
    & gh release download $tag --repo $GitHubRepo --dir $work
    if ($LASTEXITCODE -ne 0) { throw "gh couldn't download ${tag}: is the draft there, and are you signed in to gh?" }
}

$present = @(Get-ChildItem -LiteralPath $work -File | ForEach-Object { $_.Name })
$missing = @($draftFiles | Where-Object { $present -notcontains $_ })
# the files this script wrote in an earlier run in the same folder are allowed too
$written = @('release-notes.md', 'codeberg-release.json', 'codeberg-response.json')
$unexpected = @($present | Where-Object { ($draftFiles + $sigFiles + $written) -notcontains $_ })
if ($missing.Count -gt 0 -or $unexpected.Count -gt 0) {
    throw "the files in $work are wrong; missing: [$($missing -join ', ')]; unexpected: [$($unexpected -join ', ')]"
}
Write-Host "  $($draftFiles.Count) files, as the workflow makes them, in $work"

# --- 2 BLAKE3 sums -------------------------------------------------------------------------------

Step '2. BLAKE3 sums'
Push-Location $work
try {
    foreach ($file in $b3Files) {
        & b3sum --check $file
        if ($LASTEXITCODE -ne 0) { throw "$file doesn't match its file; nothing is signed" }
    }
} finally {
    Pop-Location
}
foreach ($pair in @(@($portableBridge, $portable), @($setupBridge, $setup))) {
    $bridgeHash = Read-B3Hash -Path (Join-Path $work "$($pair[0]).b3")
    $primaryHash = Read-B3Hash -Path (Join-Path $work "$($pair[1]).b3")
    if ($bridgeHash -ne $primaryHash) { throw "$($pair[0]) isn't a copy of $($pair[1]); nothing is signed" }
}
Write-Host '  the sums match, and the bridge copies are copies of the current files'

# --- 3 attestations ------------------------------------------------------------------------------

Step '3. build attestations'
foreach ($exe in $exes) {
    & gh attestation verify (Join-Path $work $exe) --repo $GitHubRepo
    if ($LASTEXITCODE -ne 0) { throw "no valid build attestation for $exe; nothing is signed" }
}

# --- 4 sign, 5 verify -----------------------------------------------------------------------------

$exePaths = @($exes | ForEach-Object { Join-Path $work $_ })
$sigPaths = @($sigFiles | ForEach-Object { Join-Path $work $_ })
$allPaths = @($allFiles | ForEach-Object { Join-Path $work $_ })

$signer = Join-Path $root 'target\release\avalio-lite-sign.exe'
$exeWords = ($exePaths | ForEach-Object { ConvertTo-BashWord $_ }) -join ' '
$signCommand = "gpg --batch -q --pinentry-mode error -d $(ConvertTo-BashWord $SigningKey) | $(ConvertTo-BashWord $signer) sign - $exeWords"

Step '4. signing the four exes with the release key'
Invoke-Change -Description 'cargo build --release --bin avalio-lite-sign' -Action {
    & cargo build --release --quiet --bin avalio-lite-sign
    if ($LASTEXITCODE -ne 0) { throw 'avalio-lite-sign didn''t build' }
}
Invoke-Change -Description "bash -c `"$signCommand`"" -Action {
    # pipefail: a failed decrypt fails the step instead of signing with an empty key
    & $GitBash -c "set -o pipefail; $signCommand"
    if ($LASTEXITCODE -ne 0) {
        throw 'signing failed: gpg couldn''t decrypt the key (no GPG key, or it gained a passphrase), or a file isn''t this checkout''s version'
    }
}

Step '5. verifying the signatures'
Invoke-Change -Description 'avalio-lite-sign verify <the four exes>' -Action {
    & $signer verify @exePaths
    if ($LASTEXITCODE -ne 0) { throw 'a signature doesn''t verify; nothing is uploaded' }
}

# --- 6 upload the signatures -------------------------------------------------------------------

Step '6. uploading the four signature files to the draft'
Invoke-Change -Description "gh release upload $tag <the two .minisig and two .sig files>" -Action {
    & gh release upload $tag @sigPaths --repo $GitHubRepo --clobber
    if ($LASTEXITCODE -ne 0) { throw 'gh release upload failed' }
}

# --- 7 GitLab and Codeberg ----------------------------------------------------------------------

Step '7. GitLab and Codeberg releases'
# the notes are the GitHub draft's, which the workflow made from the CHANGELOG section
$notesFile = Join-Path $work 'release-notes.md'
$notes = (& gh release view $tag --repo $GitHubRepo --json body --jq .body) -join "`n"
if ($LASTEXITCODE -ne 0 -or -not $notes.Trim()) { throw "couldn't read the notes of the draft $tag" }
[System.IO.File]::WriteAllText($notesFile, $notes + "`n", (New-Object System.Text.UTF8Encoding $false))

Invoke-Change -Description "glab release create $tag --repo $GitLabRepo --name `"$releaseTitle`" --notes-file <notes> <the twelve files>" -Action {
    & glab release create $tag --repo $GitLabRepo --name $releaseTitle --notes-file $notesFile @allPaths
    if ($LASTEXITCODE -ne 0) { throw 'glab release create failed' }
}

$codebergPayload = Join-Path $work 'codeberg-release.json'
$codebergResponse = Join-Path $work 'codeberg-response.json'
$payload = [ordered]@{
    tag_name = $tag
    name = $releaseTitle
    body = $notes
    draft = $false
    prerelease = $false
} | ConvertTo-Json
[System.IO.File]::WriteAllText($codebergPayload, $payload, (New-Object System.Text.UTF8Encoding $false))

Invoke-Change -Description "POST $CodebergApi/releases, then POST .../releases/<id>/assets for each of the twelve files" -Action {
    $created = Invoke-Codeberg -Method POST -Url "$CodebergApi/releases" -JsonFile $codebergPayload -ResponseFile $codebergResponse
    $releaseId = ($created | ConvertFrom-Json).id
    if (-not $releaseId) { throw 'Codeberg created the release but gave no id' }
    foreach ($path in $allPaths) {
        $leaf = Split-Path -Leaf $path
        $url = "$CodebergApi/releases/$releaseId/assets?name=$([System.Uri]::EscapeDataString($leaf))"
        Invoke-Codeberg -Method POST -Url $url -Upload $path -ResponseFile $codebergResponse | Out-Null
        Write-Host "  uploaded $leaf to Codeberg"
    }
}

# --- 8 publish the GitHub draft ------------------------------------------------------------------

Step '8. publishing the GitHub draft'
Invoke-Change -Description "gh release edit $tag --repo $GitHubRepo --draft=false" -Action {
    & gh release edit $tag --repo $GitHubRepo --draft=false
    if ($LASTEXITCODE -ne 0) { throw 'publishing the GitHub draft failed' }
}

# --- 9 each forge lists every file at its size ----------------------------------------------------

Step '9. what each forge lists'
if ($DryRun) {
    Write-Host '  dry run, not checked: the forges have nothing to list yet'
} else {
    $expected = @{}
    foreach ($leaf in $allFiles) { $expected[$leaf] = (Get-Item -LiteralPath (Join-Path $work $leaf)).Length }
    $problems = @()

    $githubJson = (& gh release view $tag --repo $GitHubRepo --json assets,isDraft) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'gh release view failed' }
    if ($githubJson.isDraft) { $problems += 'GitHub: the release is still a draft' }
    $listed = @{}
    foreach ($asset in $githubJson.assets) { $listed[$asset.name] = [long]$asset.size }
    $problems += @(Compare-Forge -Listed $listed -Expected $expected | ForEach-Object { "GitHub: $_" })

    $gitlabJson = (& glab api "projects/$([System.Uri]::EscapeDataString($GitLabRepo))/releases/$tag") | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'glab api failed for the GitLab release' }
    $listed = @{}
    foreach ($link in $gitlabJson.assets.links) {
        $address = $link.direct_asset_url
        if (-not $address) { $address = $link.url }
        $listed[$link.name] = Get-RemoteSize -Url $address
    }
    $problems += @(Compare-Forge -Listed $listed -Expected $expected | ForEach-Object { "GitLab: $_" })

    $codebergBody = Invoke-Codeberg -Method GET -Url "$CodebergApi/releases/tags/$tag" -ResponseFile $codebergResponse
    $codebergJson = $codebergBody | ConvertFrom-Json
    $listed = @{}
    foreach ($asset in $codebergJson.assets) { $listed[$asset.name] = [long]$asset.size }
    $problems += @(Compare-Forge -Listed $listed -Expected $expected | ForEach-Object { "Codeberg: $_" })

    if ($problems.Count -gt 0) {
        $problems | ForEach-Object { Write-Host "  PROBLEM $_" }
        throw 'a forge doesn''t list the release as it should; check it by hand'
    }
    Write-Host "  GitHub, GitLab and Codeberg each list all $($expected.Count) files at the right sizes"
}

Write-Host ''
if ($DryRun) {
    Write-Host "Dry run finished. Nothing was signed, uploaded, created or published. Files checked: $work"
} else {
    Write-Host "Released $releaseTitle on GitHub, GitLab and Codeberg. Files: $work"
}
