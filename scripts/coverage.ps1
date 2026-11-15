# Runs the default test suite under coverage and fails when line coverage drops below the gate.
#   .\scripts\coverage.ps1             the gate, listing each file's uncovered lines
#   .\scripts\coverage.ps1 -Branches   also branch coverage (needs the nightly toolchain:
#                                      rustup toolchain install nightly --component llvm-tools-preview)
# Needs cargo-llvm-cov (cargo install cargo-llvm-cov; rustup component add llvm-tools-preview).
# Raise MinLines as coverage grows; never lower it.
#
# Lines left uncovered on purpose (everything else the report lists needs a test):
#   src\main.rs; src\app\mod.rs run(); src\window\dialogs.rs NativeDialogs pick_folder, pick_file,
#   save_file, confirm and show_fatal
#       the real exe and the moment a Windows dialog is on screen waiting for the user; every other
#       test swaps in fake dialogs. What goes into each dialog, and open_link, are tested in
#       dialogs.rs; the dialogs themselves by the desktop test the_real_dialogs_open_and_close,
#       which isn't part of this measurement.
#   src\app\mod.rs launch: the daily licence check not starting, and an update pending at exit
#       the real exe's start and end: its licence client (src\licence\mod.rs windows_client: DPAPI,
#       HTTPS to Clavren), the real GitHub address, and starting the installer or the new exe as the
#       app exits. tests\entry_point*.rs run launch with automatic update checks off, so no test
#       goes online and none leaves with an update; update\windows.rs start is tested by itself.
#   src\update\ui.rs ready: hiding the window failing, and the event loop being there to leave
#       the headless tests have a window that always hides and no event loop; the real exe has both.
#   src\update\windows.rs this_installation, "couldn't tell where the app runs from"
#       Windows always knows a running exe's path.
#   src\window\worker.rs: a thread Windows won't create, a result of the wrong type, a job's
#   thread panicking
#       run() pairs each job with its own result type and catches the job's panics, so the last two
#       can't happen; the first needs Windows to run out of threads (the screens' handling of it is
#       tested with a worker that refuses).
#   src\app\placement.rs work_area, GetCursorPos failing (it falls back to the primary screen)
#       Windows only refuses it outside an interactive desktop, where the window can't open anyway.
#   src\window\language.rs, the error log of select_bundled_translation
#       the translations are compiled into the exe for every language the picker offers.
#   src\gradebook\model.rs ColumnType::code for the name columns
#       insert_column refuses a name column before it asks for its code; the two a class starts
#       with are written by the class's own SQL.
#   src\gradebook\classes.rs and sheet.rs, a row SQLite can't read into its type; src\platform\db\
#   migrations.rs, pragma_foreign_key_check failing
#       the schema gives every column read there its type, and the pragma only fails on a broken
#       connection, which the statement before it would have met.
#   A closing brace after a check that always holds: src\app\mod.rs "if let Ok(checks)",
#   src\app\settings.rs and src\backup\ui\restore.rs and src\spreadsheet\ui.rs after a reload that
#   worked, src\gradebook\ui\view.rs a pupil the sheet shows, src\licence\ui.rs a valid licence's
#   standing, src\shortcuts\set.rs a clash's entry, src\platform\db\migrations.rs "if record"
#       the report counts the brace as the line of the branch never taken; each check guards a
#       value that is there whenever the code before it succeeded.
#   src\state\mod.rs close_connection and replace_connection, src\state\locations.rs move_database
#   and src\backup\open.rs: "did not close cleanly"
#       SQLite only refuses to close with statements still open, and the app never keeps one.
#   src\platform\db\mod.rs discard, "could not remove", and a part-made copy before an update;
#   src\spreadsheet\export\mod.rs "could not remove the incomplete export"; src\platform\storage.rs
#   move_file, "could not remove the copy"; src\state\locations.rs move_logs, "could not move the
#   log back"; src\state\backups.rs "couldn't forget the class"; src\state\protection.rs "couldn't
#   remove ... after the PIN change", "couldn't remove ... yet", and put_back
#       a cleanup failing right after an earlier failure (a double fault), on a file the app
#       created or opened a moment before.
#   src\platform\db\mod.rs is_encrypted and open, and src\platform\logging.rs: a file that exists
#   but can't be read at all
#       reading the first bytes of a file that was just opened only fails on a failing disk.
#   src\state\locations.rs move_database, the path in "the moved database's original ... couldn't be
#   deleted"
#       the branch is tested (the status says the old file stays); the test has no logger, and a
#       log line's arguments are only worked out when something is listening.
#   src\platform\storage.rs copy_complete, a copy of another size than its original
#       fs::copy reports any write it can't finish; the check is there for a disk that says it did.
#   src\spreadsheet\import\read.rs read_table, the reader panicking
#       no file is known to make the workbook reader panic; the guard is for one that would.
#   src\state\protection.rs, reopening fails after the files were swapped (and the
#   src\lock\ui.rs and src\backup\ui\restore.rs branches that then ask for the PIN again)
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
# The nightly cargo puts every dependency in a folder of its own, which makes rustc's command line
# longer than Windows allows. Cargo then passes the arguments in a file, where cargo-llvm-cov's
# wrapper can't see the crate's name, so it instruments nothing and the report can't be read
# ("no coverage data found"). This asks cargo for the layout the pinned toolchain uses.
$layoutOptOut = '__CARGO_TEMPORARY_BUILD_DIR_NEW_LAYOUT_OPT_OUT'
$previousLayout = [Environment]::GetEnvironmentVariable($layoutOptOut)
[Environment]::SetEnvironmentVariable($layoutOptOut, '1')
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
    [Environment]::SetEnvironmentVariable($layoutOptOut, $previousLayout)
}
