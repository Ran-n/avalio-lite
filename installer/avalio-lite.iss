; Inno Setup script for avalio-lite. Built by scripts/release.ps1, which passes
; AppVersion and SourceExe on the command line:
;   ISCC.exe /DAppVersion=0.1.0 /DSourceExe=..\target\release\avalio-lite.exe avalio-lite.iss

#ifndef AppVersion
  #error AppVersion must be defined (/DAppVersion=x.y.z)
#endif
#ifndef SourceExe
  #error SourceExe must be defined (/DSourceExe=path\to\avalio-lite.exe)
#endif

[Setup]
AppId={{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}
AppName=avalio-lite
AppVersion={#AppVersion}
AppPublisher=Ran#
DefaultDirName={autopf}\avalio-lite
DefaultGroupName=avalio-lite
DisableProgramGroupPage=yes
; Installs for the current user without an administrator prompt; the wizard offers "all users" too.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=avalio-lite-{#AppVersion}-setup
LicenseFile=..\LICENSE
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\avalio-lite.exe

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "avalio-lite.exe"; Flags: ignoreversion

[Icons]
Name: "{group}\avalio-lite"; Filename: "{app}\avalio-lite.exe"
Name: "{autodesktop}\avalio-lite"; Filename: "{app}\avalio-lite.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\avalio-lite.exe"; Description: "{cm:LaunchProgram,avalio-lite}"; Flags: nowait postinstall skipifsilent

; The database, logs and settings are deliberately not removed on uninstall:
; they live in folders the user chose and hold their students' records.
