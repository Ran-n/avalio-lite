# Runs the default test suite under coverage and fails when line coverage drops below the gate.
#   .\scripts\coverage.ps1             the gate, listing each file's uncovered lines
#   .\scripts\coverage.ps1 -Branches   the branch gate instead (needs the nightly toolchain:
#                                      rustup toolchain install nightly --component llvm-tools-preview)
# The line gate is measured with the pinned toolchain only: which lines count as code (closing
# braces, lines ending in ?) changes from one compiler to the next, and nightly is a moving target.
# release-check.ps1 runs both.
# Needs cargo-llvm-cov (cargo install cargo-llvm-cov; rustup component add llvm-tools-preview).
# Raise MinLines as coverage grows; never lower it.
#
# Lines left uncovered on purpose (everything else the report lists needs a test):
#   src\main.rs; src\app\lifecycle.rs run(); src\window\dialogs.rs NativeDialogs pick_folder,
#   pick_file, save_file, confirm and show_fatal
#       the real exe and the moment a Windows dialog is on screen waiting for the user; every other
#       test swaps in fake dialogs. What goes into each dialog, and open_link, are tested in
#       dialogs.rs; the dialogs themselves by the desktop test the_real_dialogs_open_and_close,
#       which isn't part of this measurement.
#   src\app\lifecycle.rs launch: the daily licence check not starting, and an update pending at exit;
#   print_version with no output to write to; real_updates without a key
#       the real exe's start and end: its licence client (src\licence.rs windows_client: DPAPI,
#       HTTPS to Clavren), the real GitHub address, and starting the new exe as the app exits.
#       tests\entry_point*.rs run launch with automatic update checks off, so no test goes online and
#       none leaves with an update. A test's output always goes somewhere, and tests\update\ proves
#       the built-in keys.
#   src\bin\avalio-lite-sign.rs: the key asked for in a terminal, and a good signature by a built-in key
#       a test has no terminal, and never the real key: release.ps1 checks each release with it.
#   src\app\placement.rs apply, a saved place whose screen is too small or can't be read;
#   src\app\placement\work_area.rs, GetCursorPos or AdjustWindowRectExForDpi failing
#       the test machine's screens always fit the window and can be read (restore on a small screen
#       is tested by itself); Windows only refuses the calls outside an interactive desktop, where
#       the window can't open anyway.
#   src\window\worker.rs and src\update\check.rs: a thread Windows won't create, a result of the
#   wrong type, a job's thread panicking
#       run() pairs each job with its own result type and catches the job's panics, so the last two
#       can't happen; the first needs Windows to run out of threads (the screens' handling of it is
#       tested with a worker that refuses).
#   src\window\language.rs, the error log of select_bundled_translation
#       the translations are compiled into the exe for every language the picker offers.
#   src\i18n\english\errors.rs and galician\errors.rs, unreachable!
#       each group's function only gets the errors error() sends to it.
#   src\gradebook\model.rs ColumnType::code for the name columns
#       insert_column refuses a name column before it asks for its code; the two a class starts
#       with are written by the class's own SQL.
#   src\platform\db\migrations.rs, src\gradebook\ui\view.rs show_pupil_name, src\licence\ui.rs and
#   src\backup\ui\restore.rs: a closing brace after a check that always holds
#       the report counts the brace as the line of the branch never taken; each check guards a
#       value that is there whenever the code before it succeeded.
#   src\gradebook\columns\settings.rs require_round_source, the query failing
#       the statement before it reads the same table.
#   src\state.rs close_connection and replace_connection, and src\backup\open.rs: "did not close cleanly"
#       SQLite only refuses to close with statements still open, and the app never keeps one.
#   src\platform\db.rs discard, "could not remove", and a part-made copy before an update;
#   src\spreadsheet\export.rs "could not remove the incomplete export"; src\platform\storage.rs
#   move_file, "could not remove the copy"; src\state\locations.rs move_logs, "could not move the
#   log back"; src\state\backups.rs "couldn't forget the class"; src\state\protection.rs "couldn't
#   remove ... yet"; src\state\protection\swap.rs put_back and "couldn't remove ... after the PIN
#   change"; src\platform\config\legacy.rs remove_half_made; src\update\install.rs "couldn't put ...
#   back" and the removal of 0.10.0's temp folder; src\update\ui\installing.rs, the cancelled
#   download that can't be removed
#       a cleanup failing right after an earlier failure (a double fault), on a file the app
#       created or opened a moment before.
#   src\update\install.rs swap: the new program refused its name after the old one gave it up, and a
#   deletion Windows can't plan
#       the files are the app's own, made or renamed a moment before.
#   src\platform\config\legacy.rs copy_checked, a copy that differs; src\platform\storage.rs
#   copy_complete, a copy of another size than its original
#       a write that can't finish is already an error; the check is there for a disk that says it did.
#   src\platform\db.rs is_encrypted and open, and src\platform\logging.rs: a file that exists
#   but can't be read at all
#       reading the first bytes of a file that was just opened only fails on a failing disk.
#   src\spreadsheet\import\read.rs read_table, the reader panicking
#       no file is known to make the workbook reader panic; the guard is for one that would.
#   src\lock\ui.rs and src\backup\ui\restore.rs, asking for the PIN again after a failed swap
#       the swapped-in file passed its check a moment earlier; only another program grabbing it in
#       between can make reopening fail.
#   src\update\download.rs: cancelled during the hash check, a body cut short or over the limit that
#   ureq itself let through
#       a moment no test can hit on purpose (the same Cancel is tested in installing.rs place), and
#       two second checks behind ureq's own.
#   src\update\ui: schedule.rs, the uninstall entry's registry failing and a timer of an older plan;
#   updater.rs, a download that outlives the 5 s wait; view.rs status_line of a source build;
#   installing.rs, the event loop being there to leave
#       the registry is the real one; a timer is dropped with its plan; a download's thread always
#       sees its cancel; a source build's line is written before a status exists; and the headless
#       tests have no event loop, while the real exe does.
param([switch]$Branches)
$MinLines = 98
# checked with -Branches only
$MinBranches = 92

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. "$PSScriptRoot\use-perl.ps1"

# branch data needs the nightly toolchain; asking for it without one is an error, never a skip
if ($Branches -and -not (rustup toolchain list | Select-String -Quiet '^nightly')) {
    throw 'branch coverage needs the nightly toolchain: rustup toolchain install nightly --component llvm-tools-preview'
}
# RUSTUP_TOOLCHAIN outranks the rust-toolchain.toml pin, for this script only.
# Restored at the end, so a caller (release-check.ps1) builds the release with the pinned toolchain.
$previousToolchain = $env:RUSTUP_TOOLCHAIN
if ($Branches) { $env:RUSTUP_TOOLCHAIN = 'nightly' }
# The nightly cargo puts every dependency in a folder of its own, which makes rustc's command line
# longer than Windows allows. Cargo then passes the arguments in a file, where cargo-llvm-cov's
# wrapper can't see the crate's name, so it instruments nothing and the report can't be read
# ("no coverage data found"). This asks cargo for the layout the pinned toolchain uses.
$layoutOptOut = '__CARGO_TEMPORARY_BUILD_DIR_NEW_LAYOUT_OPT_OUT'
$previousLayout = [Environment]::GetEnvironmentVariable($layoutOptOut)
# Each mode builds in a folder of its own: the report reads every test exe it finds, so one left by
# the other mode's toolchain (a run that failed halfway, for example) would be counted too.
$coverageDir = Join-Path $root $(if ($Branches) { 'target\llvm-cov-branches' } else { 'target\llvm-cov-target' })
$previousCoverageDir = $env:CARGO_LLVM_COV_TARGET_DIR
[Environment]::SetEnvironmentVariable($layoutOptOut, '1')
$env:CARGO_LLVM_COV_TARGET_DIR = $coverageDir
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
        # tests\update\ runs examples\replace_self.exe from beside its own folder. cargo test builds
        # examples, but cargo-llvm-cov builds only the test targets (and --all-targets builds examples
        # as tests), so it is built here, instrumented, so src\update\install.rs replace is measured.
        # llvm-cov only builds an example by running it, and it exits with an error when given no
        # file, so what counts is that the exe is there. Inside the loop because a clean below
        # removes it.
        $helper = Join-Path $coverageDir 'debug\examples\replace_self.exe'
        Remove-Item -LiteralPath $helper -ErrorAction Ignore
        cargo llvm-cov run @flags --no-report --example replace_self
        if (-not (Test-Path -LiteralPath $helper)) { throw 'could not build the replace_self example' }
        cargo llvm-cov @flags --no-report
        if ($LASTEXITCODE -ne 0) { throw 'tests failed' }
        $stale = Find-StaleFile
        if (-not $stale) { break }
        if ($attempt -eq 2) { throw "coverage data is still inconsistent after a clean build: $stale" }
        Write-Warning "stale coverage build ($stale); cleaning it and measuring again"
        cargo llvm-cov clean
        if ($LASTEXITCODE -ne 0) { throw 'cargo llvm-cov clean failed' }
    }

    $lineGate = @(if (-not $Branches) { '--show-missing-lines', '--fail-under-lines', $MinLines })
    cargo llvm-cov report @flags @lineGate
    if ($LASTEXITCODE -ne 0) { throw $(if ($Branches) { 'could not read the coverage report' } else { "line coverage is under $MinLines %" }) }

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
    [Environment]::SetEnvironmentVariable($layoutOptOut, $previousLayout)
    $env:CARGO_LLVM_COV_TARGET_DIR = $previousCoverageDir
}
