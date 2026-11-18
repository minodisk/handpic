; handpic.iss: the per-user Inno Setup installer of the native app, compiled by
; tools/package/windows.sh, which passes these defines:
;   AppVersion  the version, tools/package/version.sh's SemVer form
;   FileVersion the version's Windows form (x.y.z.0), which Inno cannot
;               derive from a prerelease AppVersion
;   Binary      the executable's file name, e.g. handpic.exe
;   ReleaseDir  the folder holding the built executable
;   LicensesDir the folder notices.sh filled

#ifndef AppVersion
  #error Pass /DAppVersion=<version>
#endif
#ifndef FileVersion
  #error Pass /DFileVersion=<x.y.z.0>
#endif

[Setup]
; Never change AppId: upgrades and the uninstall entry
; (HKCU\...\Uninstall\dev.minodisk.handpic_is1) are found by it.
AppId=dev.minodisk.handpic
AppName=Handpic
AppVersion={#AppVersion}
AppVerName=Handpic {#AppVersion}
VersionInfoVersion={#FileVersion}
AppPublisher=minodisk
; Under PrivilegesRequired=lowest, {autopf} is the user's Programs folder,
; %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Handpic
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\..\icons\icon.ico
UninstallDisplayIcon={app}\{#Binary}
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Files]
Source: "{#ReleaseDir}\{#Binary}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#LicensesDir}\*"; DestDir: "{app}\licenses"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Handpic"; Filename: "{app}\{#Binary}"
