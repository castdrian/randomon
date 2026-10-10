Unicode true
!include "MUI2.nsh"

Name "Randomon"
OutFile "installer.exe"
InstallDir "$LOCALAPPDATA\Programs\Randomon"
RequestExecutionLevel user
VIProductVersion "${APPVERSION}.0"
VIAddVersionKey "ProductName" "Randomon"
VIAddVersionKey "FileVersion" "${APPVERSION}"
VIAddVersionKey "ProductVersion" "${APPVERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"
VIAddVersionKey "FileDescription" "Randomon installer"

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\randomon.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "randomon.exe"
  File "randomon-cli.exe"
  File "icon.ico"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\Randomon.lnk" "$INSTDIR\randomon.exe" "" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Randomon" "DisplayName" "Randomon"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Randomon" "DisplayVersion" "${APPVERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Randomon" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Randomon" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\randomon.exe"
  Delete "$INSTDIR\randomon-cli.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\Randomon.lnk"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Randomon"
SectionEnd
