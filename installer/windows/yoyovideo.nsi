!ifndef PACKAGE_DIR
  !error "PACKAGE_DIR is required"
!endif

!ifndef OUTPUT_EXE
  !error "OUTPUT_EXE is required"
!endif

!ifndef APP_VERSION
  !define APP_VERSION "dev"
!endif

!ifndef ICON_FILE
  !error "ICON_FILE is required"
!endif

; The install identity also permits side-by-side smoke installs without touching
; an existing user's installation, shortcuts or uninstall registration.
!ifndef APP_ID
  !define APP_ID "YoYoVideo"
!endif

Name "${APP_ID}"
OutFile "${OUTPUT_EXE}"
; The installer's own icon, and the icon the Add/Remove entry uses, so neither
; shows the default NSIS box.
Icon "${ICON_FILE}"
UninstallIcon "${ICON_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_ID}"
RequestExecutionLevel user
Unicode true

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${PACKAGE_DIR}\*"
  CreateDirectory "$SMPROGRAMS\${APP_ID}"
  CreateShortcut "$SMPROGRAMS\${APP_ID}\YoYoVideo.lnk" "$INSTDIR\bin\yoyovideo-desktop.exe" "" "$INSTDIR\bin\yoyovideo-desktop.exe" 0
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayName" "${APP_ID}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}" "DisplayIcon" "$INSTDIR\bin\yoyovideo-desktop.exe"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\${APP_ID}\YoYoVideo.lnk"
  RMDir "$SMPROGRAMS\${APP_ID}"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
  RMDir /r "$INSTDIR"
SectionEnd
