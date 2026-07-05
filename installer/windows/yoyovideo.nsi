!ifndef PACKAGE_DIR
  !error "PACKAGE_DIR is required"
!endif

!ifndef OUTPUT_EXE
  !error "OUTPUT_EXE is required"
!endif

!ifndef APP_VERSION
  !define APP_VERSION "dev"
!endif

Name "YoYoVideo"
OutFile "${OUTPUT_EXE}"
InstallDir "$LOCALAPPDATA\Programs\YoYoVideo"
RequestExecutionLevel user
Unicode true

Page directory
Page instfiles
UninstPage uninstConfirm
UninstPage instfiles

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "${PACKAGE_DIR}\*"
  CreateDirectory "$SMPROGRAMS\YoYoVideo"
  CreateShortcut "$SMPROGRAMS\YoYoVideo\YoYoVideo.lnk" "$INSTDIR\bin\yoyovideo-desktop.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "DisplayName" "YoYoVideo"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "UninstallString" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo" "InstallLocation" "$INSTDIR"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\YoYoVideo\YoYoVideo.lnk"
  RMDir "$SMPROGRAMS\YoYoVideo"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\YoYoVideo"
  RMDir /r "$INSTDIR"
SectionEnd
