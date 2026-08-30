; Inno Setup installer for the Rust OMT OBS plugin.
; Builds a 64-bit setup that does not overwrite official C# omtplugin.

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "OMT OBS Plugin (Rust)"
#define MyAppPublisher "MikanseiLaboratory"
#define MyAppURL "https://github.com/MikanseiLaboratory/omt-obs-plugin"
#define OfficialPluginURL "https://github.com/openmediatransport/omtplugin"

[Setup]
AppId={{8F3A2C1D-9E47-4B6A-A1D0-5C7E2F8B4A91}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppComments=Coexists with the official C# omtplugin ({#OfficialPluginURL})
DefaultDirName={commonappdata}\obs-studio\plugins\omt-obs-plugin
DisableDirPage=no
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=omt-obs-plugin-{#MyAppVersion}-windows-x64-setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=admin
MinVersion=10.0
UninstallDisplayName={#MyAppName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\..\target\release\omt_obs_plugin.dll"; DestDir: "{app}\bin\64bit"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
