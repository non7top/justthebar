; Per-user installer. Build with: makensis -DVERSION=x.y.z -DVERSION_NUMERIC=x.y.z.0 -DSTAGE=<dir> -DOUTFILE=<file> jtb.nsi
; Release builds add -DPROVENANCE_REPO=owner/repo, which is the claim the Sigstore attestation is checked against.
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!ifndef VERSION
  !error "pass -DVERSION=x.y.z"
!endif
!ifndef VERSION_NUMERIC
  !error "pass -DVERSION_NUMERIC=x.y.z.w"
!endif
!ifndef STAGE
  !error "pass -DSTAGE=<dir with the files to install>"
!endif
!ifndef OUTFILE
  !define OUTFILE "jtb-setup.exe"
!endif

!define NAME "JustTheBar"
!define UNKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\JustTheBar"
!define CONTROL_CLASS "JustTheBarControl"

Name "${NAME}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\${NAME}"
InstallDirRegKey HKCU "Software\${NAME}" "InstallDir"
SetCompressor /SOLID lzma
BrandingText "${NAME} ${VERSION}"

VIProductVersion "${VERSION_NUMERIC}"
VIFileVersion "${VERSION_NUMERIC}"
VIAddVersionKey /LANG=1033 "ProductName" "${NAME}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Apache-2.0"
VIAddVersionKey /LANG=1033 "FileDescription" "${NAME} installer"
!ifdef PROVENANCE_REPO
  VIAddVersionKey /LANG=1033 "ProvenanceRepo" "${PROVENANCE_REPO}"
!endif

; Set when a bar was running at install time, so the finish page and silent installs bring it back.
Var WasRunning

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "Start ${NAME}"
!define MUI_FINISHPAGE_RUN_FUNCTION StartBar
!define MUI_PAGE_CUSTOMFUNCTION_SHOW FinishShow
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; Pushes nothing, pops the path of an exe that can signal the running bar. Sets $WasRunning.
!macro StopBarFn un
Function ${un}StopBar
  Exch $R0
  Push $0
  Push $1
  StrCpy $WasRunning 0
  FindWindow $0 "${CONTROL_CLASS}"
  ${If} $0 != 0
    StrCpy $WasRunning 1
    ExecWait '"$R0" --quit'
    StrCpy $1 0
    ${DoWhile} $1 < 30
      FindWindow $0 "${CONTROL_CLASS}"
      ${If} $0 == 0
        ${Break}
      ${EndIf}
      Sleep 500
      IntOp $1 $1 + 1
    ${Loop}
    Sleep 1000
  ${EndIf}
  Pop $1
  Pop $0
  Pop $R0
FunctionEnd
!macroend
!insertmacro StopBarFn ""
!insertmacro StopBarFn "un."

Function StartBar
  SetOutPath "$INSTDIR"
  Exec '"$INSTDIR\jtb-app.exe"'
FunctionEnd

Function FinishShow
  ${If} $WasRunning == 1
    SendMessage $mui.FinishPage.Text ${WM_SETTEXT} 0 "STR:${NAME} was stopped to update it. Restart it to run the new version."
    SendMessage $mui.FinishPage.Run ${WM_SETTEXT} 0 "STR:Restart ${NAME}"
  ${EndIf}
FunctionEnd

Function .onInstSuccess
  ${If} ${Silent}
  ${AndIf} $WasRunning == 1
    Call StartBar
  ${EndIf}
FunctionEnd

Section "Install"
  ; The running bar locks its exe, and any copy of the exe can signal it to quit.
  InitPluginsDir
  File "/oname=$PLUGINSDIR\jtb-app.exe" "${STAGE}\jtb-app.exe"
  Push "$PLUGINSDIR\jtb-app.exe"
  Call StopBar

  SetOutPath "$INSTDIR"
  SetOverwrite on
  File "${STAGE}\jtb-app.exe"
  File "${STAGE}\jtb_hook.dll"
  File "${STAGE}\*.cmd"
  File "${STAGE}\jtb.example.json"
  File "${STAGE}\LICENSE"
  File "${STAGE}\NOTICE"

  CreateShortcut "$SMPROGRAMS\${NAME}.lnk" "$INSTDIR\jtb-app.exe" "" "$INSTDIR\jtb-app.exe" 0
  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKCU "Software\${NAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNKEY}" "DisplayName" "${NAME}"
  WriteRegStr HKCU "${UNKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNKEY}" "Publisher" "non7top"
  WriteRegStr HKCU "${UNKEY}" "URLInfoAbout" "https://github.com/non7top/justthebar"
  WriteRegStr HKCU "${UNKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNKEY}" "DisplayIcon" "$INSTDIR\jtb-app.exe"
  WriteRegStr HKCU "${UNKEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNKEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNKEY}" "NoRepair" 1
SectionEnd

; jtb.json, jtb.log and the dated logs are the user's and stay.
Section "Uninstall"
  Push "$INSTDIR\jtb-app.exe"
  Call un.StopBar

  Delete "$INSTDIR\jtb-app.exe"
  Delete "$INSTDIR\jtb_hook.dll"
  Delete "$INSTDIR\*.cmd"
  Delete "$INSTDIR\jtb.example.json"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\NOTICE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${NAME}.lnk"
  DeleteRegKey HKCU "${UNKEY}"
  DeleteRegKey HKCU "Software\${NAME}"
SectionEnd
