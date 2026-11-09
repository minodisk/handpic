; Tauri keys the uninstall entry on the product name, and an updater run
; (`/UPDATE`) never uninstalls a previous version, so without this hook an
; update under a new product name installs next to Riffle. While the product
; is still named Riffle the macro bodies are empty. The comparison sits inside
; the bodies because the template includes this file before it defines
; `PRODUCTNAME`; it is only set by the time `!insertmacro` expands them.
!define LEGACY_PRODUCTNAME "Riffle"

!define LEGACY_UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${LEGACY_PRODUCTNAME}"

Var LegacyStartMenuShortcut
Var LegacyDesktopShortcut

; `/UPDATE` also makes the old uninstaller keep its shortcuts and the new
; installer skip creating its own, which would leave a dead Riffle shortcut
; and none for the new name. Clobbers $0-$3, as `IsShortcutTarget` does.
!macro RemoveLegacyShortcut shortcut target flag
  !insertmacro IsShortcutTarget "${shortcut}" "${target}"
  Pop $R4
  ${If} $R4 = 1
    !insertmacro UnpinShortcut "${shortcut}"
    Delete "${shortcut}"
    StrCpy ${flag} 1
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
!if "${PRODUCTNAME}" != "${LEGACY_PRODUCTNAME}"
  ReadRegStr $R0 SHCTX "${LEGACY_UNINSTKEY}" "UninstallString"
  ${If} $R0 != ""
    ReadRegStr $R1 SHCTX "${LEGACY_UNINSTKEY}" "InstallLocation"
    ReadRegStr $R2 SHCTX "${LEGACY_UNINSTKEY}" "MainBinaryName"
    StrCpy $R3 $R1 1
    ${If} $R3 == '"'
      StrCpy $R1 $R1 -1 1
    ${EndIf}
    ClearErrors
    ExecWait '$R0 /P /UPDATE _?=$R1' $R3
    ${IfNot} ${Errors}
    ${AndIf} $R3 = 0
      !insertmacro RemoveLegacyShortcut "$SMPROGRAMS\${LEGACY_PRODUCTNAME}.lnk" "$R1\$R2" $LegacyStartMenuShortcut
      !insertmacro RemoveLegacyShortcut "$DESKTOP\${LEGACY_PRODUCTNAME}.lnk" "$R1\$R2" $LegacyDesktopShortcut
      ; `_?=` runs the uninstaller in place, so it cannot delete itself.
      Delete "$R1\uninstall.exe"
      ${If} $R1 != $INSTDIR
        RMDir "$R1"
      ${EndIf}
    ${EndIf}
  ${EndIf}
!endif
!macroend

!macro NSIS_HOOK_POSTINSTALL
!if "${PRODUCTNAME}" != "${LEGACY_PRODUCTNAME}"
  ${If} $LegacyStartMenuShortcut = 1
    CreateShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  ${EndIf}
  ${If} $LegacyDesktopShortcut = 1
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
!endif
!macroend
