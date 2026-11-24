; The updater starts setup before asking the main window to close normally.
; Wait for that exact process, including its database and window-state shutdown.
!macro NSIS_HOOK_PREINSTALL
  ClearErrors
  ${GetOptions} $CMDLINE "/DSS-WAIT-PID=" $R8
  ${IfNot} ${Errors}
    System::Call 'kernel32::OpenProcess(i 0x00100000, i 0, i R8) p .r8'
    ${If} $8 P<> 0
      System::Call 'kernel32::WaitForSingleObject(p r8, i 60000) i .r9'
      System::Call 'kernel32::CloseHandle(p r8)'
      ${If} $9 != 0
        Abort "Document Studio Spreadsheets is still open. Save your work, close the app, and run setup again."
      ${EndIf}
    ${EndIf}
  ${EndIf}
  ClearErrors

  ; Manual installs can also replace a running executable with an older name.
  ; Keep its workbooks open and stop setup if Windows reports that it is in use.
  ReadRegStr $R8 SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $R8 != ""
  ${AndIf} ${FileExists} "$INSTDIR\$R8"
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "$INSTDIR\$R8"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${If} $0 = 234 ; ERROR_MORE_DATA means at least one process is using the file.
          !insertmacro RestartManager_EndSession $R0
          Abort "Save your work and close Document Studio Spreadsheets before running setup again."
        ${EndIf}
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${EndIf}
!macroend
