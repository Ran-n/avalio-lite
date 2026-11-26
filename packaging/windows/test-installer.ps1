# Tests the setup silently, in a scratch folder inside the build directory:
#   packaging\windows\test-installer.ps1 -Installer dist\<setup>.exe [-BuildDirectory target]
# It installs, checks the install, installs again over itself and checks that, uninstalls, and checks
# that nothing is left. The checks: the uninstall record's DisplayName is exactly "avalio lite" and its
# DisplayVersion is the version in Cargo.toml; the Start-menu and desktop shortcuts exist and point at the
# installed exe; the shortcuts earlier setups made under the hyphenated name are removed; and
# "avalio-lite.exe --version" prints "avalio lite <the record's version>".
# It refuses to run while a real installation exists on this machine, because the test uses the same
# uninstall record and shortcuts. Relative paths are taken from the repository root.

param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [string]$BuildDirectory = 'target'
)

. "$PSScriptRoot\..\..\scripts\release-common.ps1"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Set-Location $root

# the AppId of avalio-lite.iss; the app reads the uninstall record under it (src/update/windows.rs)
$AppId = '{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}'
$RecordKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\${AppId}_is1"
$LocalInstall = Join-Path $env:LOCALAPPDATA 'Programs\avalio-lite'
# {group} in the .iss is a folder named "avalio lite" inside the Start menu's Programs
$StartShortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'avalio lite\avalio lite.lnk'
$DesktopShortcut = Join-Path ([Environment]::GetFolderPath('Desktop')) 'avalio lite.lnk'
# what setups up to 0.10.0 made; [InstallDelete] removes them
$OldStartFolder = Join-Path ([Environment]::GetFolderPath('Programs')) 'avalio-lite'
$OldStartShortcut = Join-Path $OldStartFolder 'avalio-lite.lnk'
$OldDesktopShortcut = Join-Path ([Environment]::GetFolderPath('Desktop')) 'avalio-lite.lnk'

$setup = Resolve-FullPath -Root $root -Path $Installer
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) { throw "no installer at $setup" }
$buildDir = Resolve-FullPath -Root $root -Path $BuildDirectory
if (-not (Test-Path -LiteralPath $buildDir -PathType Container)) { throw "no build directory at $buildDir" }
$version = Get-AppVersion

# Refuses before anything is created or changed.
$traces = @()
if (Test-Path -LiteralPath $RecordKey) { $traces += "uninstall record $RecordKey" }
if (Test-Path -LiteralPath $LocalInstall) { $traces += "install folder $LocalInstall" }
if (Test-Path -LiteralPath $StartShortcut) { $traces += "Start-menu shortcut $StartShortcut" }
if (Test-Path -LiteralPath $DesktopShortcut) { $traces += "desktop shortcut $DesktopShortcut" }
if (Test-Path -LiteralPath $OldStartShortcut) { $traces += "Start-menu shortcut $OldStartShortcut" }
if (Test-Path -LiteralPath $OldDesktopShortcut) { $traces += "desktop shortcut $OldDesktopShortcut" }
if ($traces.Count -gt 0) {
    throw ('refusing to run: a real avalio-lite installation exists on this machine (' + ($traces -join '; ') +
        '). Uninstall it first; this test would share its uninstall record and shortcuts.')
}

$scratch = Join-Path $buildDir ('installer-test-' + [guid]::NewGuid().ToString('N'))
Assert-InsideDirectory -Path $scratch -Directory $buildDir
$app = Join-Path $scratch 'app'
$logs = Join-Path $scratch 'logs'
$config = Join-Path $scratch 'config'
New-Item -ItemType Directory -Path $logs, $config -ErrorAction Stop | Out-Null

$script:Failures = New-Object System.Collections.Generic.List[string]

function Check {
    # Prints one line per check and keeps going, so every failure is reported, not only the first.
    param([bool]$Passed, [string]$Description)
    if ($Passed) {
        Write-Host "  ok    $Description"
    } else {
        Write-Host "  FAIL  $Description"
        $script:Failures.Add($Description)
    }
}

function Invoke-Program {
    # Runs a program and waits for it. One still running after the timeout is stopped, and the call throws.
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string]$Arguments,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [string]$OutputFile
    )
    $start = @{ FilePath = $FilePath; PassThru = $true }
    if ($Arguments) { $start.ArgumentList = $Arguments }
    if ($OutputFile) { $start.RedirectStandardOutput = $OutputFile }
    $process = Start-Process @start
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill()
        throw "$(Split-Path -Leaf $FilePath) did not finish within $TimeoutSeconds seconds and was stopped"
    }
    return $process.ExitCode
}

function Install-Setup {
    param([string]$Label)
    $log = Join-Path $logs "$Label.log"
    $arguments = "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /DIR=`"$app`" /TASKS=desktopicon /LOG=`"$log`""
    $code = Invoke-Program -FilePath $setup -Arguments $arguments -TimeoutSeconds 900
    if ($code -ne 0) { throw "$Label exited with code $code; its log is $log" }
}

function Uninstall-Setup {
    $uninstaller = Join-Path $app 'unins000.exe'
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) { throw "no uninstaller at $uninstaller" }
    $log = Join-Path $logs 'uninstall.log'
    $code = Invoke-Program -FilePath $uninstaller -Arguments "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG=`"$log`"" -TimeoutSeconds 300
    if ($code -ne 0) { throw "the uninstaller exited with code $code; its log is $log" }
    # The uninstaller copies itself to %TEMP% and that copy does the removing, so the process above can
    # exit first. Wait until the record, the files and the shortcuts are gone, for up to two minutes.
    $deadline = (Get-Date).AddSeconds(120)
    while (((Test-Path -LiteralPath $RecordKey) -or (Test-Path -LiteralPath $app) -or
            (Test-Path -LiteralPath $StartShortcut) -or (Test-Path -LiteralPath $DesktopShortcut)) -and
           ((Get-Date) -lt $deadline)) {
        Start-Sleep -Milliseconds 500
    }
}

function Test-Shortcut {
    param([string]$Label, [string]$Path, [string]$Target, [string]$What)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Check $false "${Label}: the $What exists"
        return
    }
    $shell = New-Object -ComObject WScript.Shell
    try {
        $actual = [string]$shell.CreateShortcut($Path).TargetPath
    } finally {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell)
    }
    Check ($actual -ne '' -and [System.IO.Path]::GetFullPath($actual) -ieq [System.IO.Path]::GetFullPath($Target)) `
        "${Label}: the $What points at the installed exe (it points at '$actual')"
}

function Test-Installed {
    param([string]$Label)
    $exe = Join-Path $app 'avalio-lite.exe'
    Check (Test-Path -LiteralPath $exe -PathType Leaf) "${Label}: avalio-lite.exe is in the install folder"

    $record = Get-ItemProperty -LiteralPath $RecordKey -ErrorAction SilentlyContinue
    Check ($null -ne $record) "${Label}: the uninstall record exists"
    if ($null -eq $record) { return }
    Check ($record.DisplayName -ceq 'avalio lite') "${Label}: the record's DisplayName is 'avalio lite' (it is '$($record.DisplayName)')"
    Check (-not (Test-Path -LiteralPath $OldStartShortcut)) "${Label}: the old Start-menu shortcut $OldStartShortcut is gone"
    Check (-not (Test-Path -LiteralPath $OldStartFolder)) "${Label}: the old Start-menu folder $OldStartFolder is gone"
    Check (-not (Test-Path -LiteralPath $OldDesktopShortcut)) "${Label}: the old desktop shortcut $OldDesktopShortcut is gone"
    Check ($record.DisplayVersion -ceq $version) "${Label}: the record's DisplayVersion is the Cargo.toml version $version (it is '$($record.DisplayVersion)')"

    Test-Shortcut -Label $Label -Path $StartShortcut -Target $exe -What 'Start-menu shortcut'
    Test-Shortcut -Label $Label -Path $DesktopShortcut -Target $exe -What 'desktop shortcut'

    # the exe is started with its settings folder in the scratch folder, so the real settings are never read
    $expected = "avalio lite $($record.DisplayVersion)"
    $output = Join-Path $scratch "$Label-version.txt"
    $printed = $null
    $previous = $env:AVALIO_LITE_CONFIG_DIR
    $env:AVALIO_LITE_CONFIG_DIR = $config
    try {
        Invoke-Program -FilePath $exe -Arguments '--version' -TimeoutSeconds 30 -OutputFile $output | Out-Null
        $printed = ([string](Get-Content -LiteralPath $output -Raw)).Trim()
    } catch {
        Check $false "${Label}: avalio-lite.exe --version: $($_.Exception.Message)"
    } finally {
        $env:AVALIO_LITE_CONFIG_DIR = $previous
    }
    if ($null -ne $printed) {
        Check ($printed -ceq $expected) "${Label}: avalio-lite.exe --version prints '$expected' (it printed '$printed')"
    }
}

function Test-Removed {
    Check (-not (Test-Path -LiteralPath $RecordKey)) 'uninstall: the uninstall record is gone'
    Check (-not (Test-Path -LiteralPath $StartShortcut)) 'uninstall: the Start-menu shortcut is gone'
    Check (-not (Test-Path -LiteralPath $DesktopShortcut)) 'uninstall: the desktop shortcut is gone'
    $left = @()
    if (Test-Path -LiteralPath $app) {
        $left = @(Get-ChildItem -LiteralPath $app -Force -Recurse | ForEach-Object { $_.FullName })
    }
    $detail = ''
    if ($left.Count -gt 0) { $detail = ': ' + ($left -join ', ') }
    Check ($left.Count -eq 0) "uninstall: nothing is left in the install folder$detail"
}

Write-Host "Testing $setup (avalio-lite $version)"
Write-Host "Scratch folder, removed at the end: $scratch"
$uninstallAttempted = $false
try {
    Write-Host '1. install silently, over the shortcuts an older setup left'
    # stand-ins for the hyphen-named shortcuts of setups up to 0.10.0; the install must remove them
    New-Item -ItemType Directory -Path $OldStartFolder -Force | Out-Null
    Set-Content -LiteralPath $OldStartShortcut -Value 'stand-in' -Encoding ascii
    Set-Content -LiteralPath $OldDesktopShortcut -Value 'stand-in' -Encoding ascii
    Install-Setup -Label 'install'
    Test-Installed -Label 'install'

    Write-Host '2. install again over the same folder'
    Install-Setup -Label 'reinstall'
    Test-Installed -Label 'reinstall'

    Write-Host '3. uninstall silently'
    $uninstallAttempted = $true
    Uninstall-Setup
    Test-Removed
} catch {
    Check $false "the test stopped: $($_.Exception.Message)"
} finally {
    # The stand-ins, if the install failed before removing them (none existed when the test began).
    foreach ($standIn in @($OldStartShortcut, $OldDesktopShortcut)) {
        if (Test-Path -LiteralPath $standIn) { Remove-Item -LiteralPath $standIn -Force }
    }
    if (Test-Path -LiteralPath $OldStartFolder) { Remove-Item -LiteralPath $OldStartFolder -Force }
    # Whatever happened, take back what this test installed; the record was absent when the test began.
    $installedHere = (Test-Path -LiteralPath $RecordKey) -or (Test-Path -LiteralPath $app)
    if ($installedHere -and -not $uninstallAttempted) {
        Write-Host 'cleaning up: uninstalling the copy this test installed'
        try {
            Uninstall-Setup
        } catch {
            Check $false "cleanup: $($_.Exception.Message)"
        }
    }
    if (Test-Path -LiteralPath $RecordKey) {
        Check $false "cleanup: the uninstall record $RecordKey is still there; delete that key by hand"
    }
    # The uninstaller's temp copy can still hold its log for a moment after the record is gone: retry for 30 seconds.
    $deadline = (Get-Date).AddSeconds(30)
    while ($true) {
        try {
            Assert-InsideDirectory -Path $scratch -Directory $buildDir
            if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction Stop }
            break
        } catch {
            if ((Get-Date) -ge $deadline) {
                Check $false "cleanup: the scratch folder $scratch couldn't be removed: $($_.Exception.Message)"
                break
            }
            Start-Sleep -Milliseconds 500
        }
    }
}

Write-Host ''
if ($script:Failures.Count -gt 0) {
    Write-Host "$($script:Failures.Count) check(s) failed:"
    foreach ($failure in $script:Failures) { Write-Host "  - $failure" }
    exit 1
}
Write-Host 'All installer checks passed.'
