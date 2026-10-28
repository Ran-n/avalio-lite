# Puts a full Perl first on PATH for this PowerShell session. SQLCipher's OpenSSL is compiled from
# source, and its build needs modules Git's bundled Perl lacks. Uses $env:AVALIO_PERL_DIR, or
# Strawberry Perl in its usual place. Dot-source it: . "$PSScriptRoot\use-perl.ps1"
$perlDir = @($env:AVALIO_PERL_DIR, 'C:\Strawberry\perl\bin') | Where-Object { $_ -and (Test-Path (Join-Path $_ 'perl.exe')) } | Select-Object -First 1
if (-not $perlDir) { throw 'Strawberry Perl not found: install it (winget install StrawberryPerl.StrawberryPerl) or set AVALIO_PERL_DIR to its perl\bin folder' }
$env:PATH = "$perlDir;$env:PATH"
