; Inno Setup script for the avalio-lite setup (installer) for Windows 64-bit.
; packaging/windows/build-installer.ps1 runs ISCC with these defines, and only these:
;   /DAppVersion=<version>       from Cargo.toml
;   /DOutputName=<name>          the asset name without .exe, from avalio-lite-asset-name
;   /DOutputDir=<folder>         where the setup is written
;   /DSourceExe=<path>           the exe to install, as a full path
; The output name is not built here, so the name rules live only in src/update/target.rs.
; Relative paths below are relative to this file, in packaging\windows\.

#ifndef AppVersion
  #error AppVersion must be defined (/DAppVersion=x.y.z, from Cargo.toml)
#endif
#ifndef OutputName
  #error OutputName must be defined (/DOutputName=..., from avalio-lite-asset-name)
#endif
#ifndef OutputDir
  #error OutputDir must be defined (/DOutputDir=folder)
#endif
#ifndef SourceExe
  #error SourceExe must be defined (/DSourceExe=path\to\avalio-lite.exe)
#endif

[Setup]
; The app reads this uninstall record (HKCU\...\Uninstall\{...}_is1) to tell where it was installed,
; so the AppId must never change. Inno reads "{{" as one "{".
AppId={{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}
AppName=avalio lite
AppVersion={#AppVersion}
AppPublisher=Breren
AppPublisherURL=https://breren.com
DefaultDirName={autopf}\avalio-lite
DefaultGroupName=avalio lite
DisableProgramGroupPage=yes
; Per user, no administrator rights: {autopf} is then %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
; "commandline": the wizard doesn't offer "all users"; the switch /ALLUSERS still does.
PrivilegesRequiredOverridesAllowed=commandline
; Default yes, stated here on purpose: an earlier all-users install is upgraded in place, so no
; second copy is left beside the per-user one. Upgrading it needs administrator rights (UAC).
UsePreviousPrivileges=yes
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
LicenseFile=..\..\LICENSE
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; 64-bit only; Windows 10 1809 (build 17763) or later
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
SetupIconFile=..\..\assets\logo.ico
; Plain name: Inno's default "avalio-lite version X" would be stale after an in-app update.
UninstallDisplayName=avalio lite
; the exe carries the logo too (build.rs embeds it), so the uninstall entry shows it
UninstallDisplayIcon={app}\avalio-lite.exe
; The in-app updater runs this setup silently right after the app closes: the Restart Manager
; closes a copy still exiting. RestartApplications=no: this setup never restarts the app itself;
; the [Run] entries below open it.
CloseApplications=force
RestartApplications=no

[Tasks]
; off by default: the desktop icon is opt-in
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; the installed exe is the same binary as the portable release
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "avalio-lite.exe"; Flags: ignoreversion

[InstallDelete]
; the shortcuts earlier setups made under the hyphenated name, so an upgrade leaves no second entry
Type: files; Name: "{autoprograms}\avalio-lite\avalio-lite.lnk"
Type: dirifempty; Name: "{autoprograms}\avalio-lite"
Type: files; Name: "{autodesktop}\avalio-lite.lnk"

[Icons]
Name: "{group}\avalio lite"; Filename: "{app}\avalio-lite.exe"
Name: "{autodesktop}\avalio lite"; Filename: "{app}\avalio-lite.exe"; Tasks: desktopicon

[Run]
; a normal install offers to open the app at the end; a silent install never does
Filename: "{app}\avalio-lite.exe"; Description: "{cm:LaunchProgram,avalio lite}"; Flags: nowait postinstall skipifsilent
; For the bridge: copies 0.6.0 to 0.10.0 run this setup silently with /RELAUNCH=1 and expect it to
; open the app again. Remove it with the bridge file names (after 2027-05-15).
Filename: "{app}\avalio-lite.exe"; Flags: nowait; Check: Relaunch

; The database, logs and settings are deliberately not removed on uninstall:
; they live in folders the user chose and hold their students' records.

[Code]
// The in-app updater passes /RELAUNCH=1; a silent install for any other reason doesn't open the app.
function Relaunch: Boolean;
begin
  Result := ExpandConstant('{param:RELAUNCH|0}') = '1';
end;
