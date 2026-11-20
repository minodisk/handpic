; handpic.iss: the per-user Inno Setup installer of the app, compiled by
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

; The app's updater runs this setup as the app quits, with /WAITPID=<pid> (the
; quitting app, waited for before anything is installed, so its exe is not in
; use) and, on Restart Now, /RELAUNCH, which starts the installed app at the
; end. A setup run by hand passes neither and starts nothing.
[Run]
Filename: "{app}\{#Binary}"; Flags: nowait; Check: RelaunchAsked

[Code]
const
  SYNCHRONIZE = $00100000;
  // At most this long; a hung app is then left to the Restart Manager.
  WaitForAppMs = 60000;

function OpenProcess(dwDesiredAccess: DWORD; bInheritHandle: BOOL; dwProcessId: DWORD): THandle;
  external 'OpenProcess@kernel32.dll stdcall';
function WaitForSingleObject(hHandle: THandle; dwMilliseconds: DWORD): DWORD;
  external 'WaitForSingleObject@kernel32.dll stdcall';
function CloseHandle(hObject: THandle): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';

function InitializeSetup(): Boolean;
var
  Pid: DWORD;
  Process: THandle;
begin
  Pid := StrToIntDef(ExpandConstant('{param:WAITPID|0}'), 0);
  if Pid <> 0 then
  begin
    Process := OpenProcess(SYNCHRONIZE, False, Pid);
    if Process <> 0 then
    begin
      WaitForSingleObject(Process, WaitForAppMs);
      CloseHandle(Process);
    end;
  end;
  Result := True;
end;

function RelaunchAsked(): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/RELAUNCH') = 0 then
      Result := True;
end;
