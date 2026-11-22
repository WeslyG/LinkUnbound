!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $0 SHCTX "Software\${MANUFACTURER}\${PRODUCTNAME}" ""
  ${If} $0 == ""
  ${AndIf} $INSTDIR == "$LOCALAPPDATA\${PRODUCTNAME}"
    StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${PRODUCTNAME}"
    SetOutPath $INSTDIR
  ${EndIf}
  !insertmacro CheckIfAppIsRunning "linkunbound-weslyg-shell.exe" "${PRODUCTNAME}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --register' $0
  ${If} $0 <> 0
    SetErrorLevel 1
    Abort "LinkUnbound could not finish setup."
  ${EndIf}
  nsis_tauri_utils::RunAsUser "$INSTDIR\linkunbound-weslyg-shell.exe" "--hushed"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  !insertmacro CheckIfAppIsRunning "linkunbound-weslyg-shell.exe" "${PRODUCTNAME}"
  ${If} $UpdateMode <> 1
    ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --unregister'
    ${EndIf}

    DeleteRegKey HKCU "Software\Classes\LinkUnboundWeslyGURL"
    DeleteRegKey HKCU "Software\Classes\linkunbound-weslyg"
    DeleteRegKey HKCU "Software\Clients\StartMenuInternet\LinkUnbound (WeslyG)"
    DeleteRegKey HKCU "Software\LinkUnbound (WeslyG)"
    DeleteRegValue HKCU "Software\RegisteredApplications" "LinkUnbound (WeslyG)"
    DeleteRegValue HKCU "Software\Classes\.htm\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.html\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.xhtml\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.xht\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.pdf\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.svg\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.mhtml\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.mht\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.shtml\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Classes\.webp\OpenWithProgIds" "LinkUnboundWeslyGURL"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "LinkUnboundWeslyG"
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "LinkUnboundWeslyG"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} $DeleteAppDataCheckboxState = 1
    RMDir /r "$LOCALAPPDATA\LinkUnboundWeslyG"
  ${EndIf}
!macroend
