; The Windows setup for the Headquarter app (add-windows-installer D1). Built in the release by
; .github/workflows/build-windows-installer.yml:
;   ISCC /DAppVersion=0.4.0 /DFileVersion=0.4.0.0 /DSourceExe=<headquarter-app.exe> /DOutputDir=<dir> headquarter.iss
; It installs for the current user, with no administrator prompt, and keeps the app's data in
; %APPDATA%\Headquarter when it uninstalls.

#ifndef AppVersion
  #error AppVersion is required (/DAppVersion=...)
#endif
#ifndef FileVersion
  #error FileVersion is required (/DFileVersion=x.y.z.0)
#endif
#ifndef SourceExe
  #error SourceExe is required (/DSourceExe=...)
#endif
#ifndef OutputDir
  #define OutputDir "."
#endif

[Setup]
; Never change AppId: Windows matches an installed copy to its update by it.
AppId={{6F1D4C2E-8B7A-4E59-9C3D-2A41B7E8F0C5}
AppName=Headquarter
AppVersion={#AppVersion}
AppVerName=Headquarter {#AppVersion}
AppPublisher=Headquarter
AppPublisherURL=https://github.com/alfonsusenrico/headquarter
AppSupportURL=https://github.com/alfonsusenrico/headquarter/issues
AppUpdatesURL=https://github.com/alfonsusenrico/headquarter/releases
VersionInfoVersion={#FileVersion}
VersionInfoProductName=Headquarter
VersionInfoDescription=Headquarter setup
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\Headquarter
DefaultGroupName=Headquarter
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SetupIconFile=..\..\assets\icon\headquarter.ico
UninstallDisplayIcon={app}\headquarter-app.exe
UninstallDisplayName=Headquarter
CloseApplications=yes
RestartApplications=no
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
OutputDir={#OutputDir}
OutputBaseFilename=headquarter-setup-x86_64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "headquarter-app.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Headquarter"; Filename: "{app}\headquarter-app.exe"
Name: "{autodesktop}\Headquarter"; Filename: "{app}\headquarter-app.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\headquarter-app.exe"; Description: "{cm:LaunchProgram,Headquarter}"; Flags: nowait postinstall skipifsilent
