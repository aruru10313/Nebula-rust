; Nebulya Launcher — Windows 설치 스크립트 (Inno Setup 6)
; CI에서 Minionguyjpro/Inno-Setup-Action 으로 빌드됨.
; 로컬: iscc installer/windows/nebulya-setup.iss /DBinaryDir=target\x86_64-pc-windows-msvc\release

#define MyAppName "Nebulya Launcher"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "Nebulya"
#define MyAppURL "https://github.com/aruru10313/Nebula-rust"
#define MyAppExeName "nebulya-launcher.exe"

#ifndef BinaryDir
  #define BinaryDir "..\..\target\x86_64-pc-windows-msvc\release"
#endif

[Setup]
AppId={{3B9E7C2A-6F1A-4E2B-9C4D-NEBULYA02}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\Nebulya Launcher
DefaultGroupName=Nebulya Launcher
OutputDir=..\..\dist
OutputBaseFilename=Nebulya-Launcher-Setup-{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#BinaryDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
