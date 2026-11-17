; handpic.iss: the per-user Inno Setup installer of the native app, compiled by
; tools/package/windows.sh, which passes these defines:
;   AppVersion  the version, tools/package/version.sh's
;   Binary      the executable's file name, e.g. riffle-native.exe
;   ReleaseDir  the folder holding the built executable
;   LicensesDir the folder notices.sh filled

#ifndef AppVersion
  #error Pass /DAppVersion=<version>
#endif

[Setup]
; Never change AppId: upgrades and the uninstall entry
; (HKCU\...\Uninstall\dev.minodisk.handpic_is1) are found by it.
AppId=dev.minodisk.handpic
AppName=Handpic
AppVersion={#AppVersion}
AppVerName=Handpic {#AppVersion}
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
