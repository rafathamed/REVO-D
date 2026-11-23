#define MyAppName "REVO D"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "REVO D Project"
#define MyAppExeName "revo-d-gui.exe"

[Setup]
AppId={{C2C8D5E1-73B9-4E7C-A8C8-0D5F41A9D301}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\REVO D
DefaultGroupName=REVO D
OutputDir=..\dist
OutputBaseFilename=REVO-D-Setup-{#MyAppVersion}-x64
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Files]
Source: "..\target\release\revo-d-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\revo-d.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\REVO D"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\REVO D"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch REVO D"; Flags: postinstall nowait skipifsilent
