Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "LogicLib.nsh"

Name "randomon"
OutFile "installer.exe"
InstallDir "$LOCALAPPDATA\Programs\randomon"
RequestExecutionLevel user
VIProductVersion "${APPVERSION}.0"
VIAddVersionKey "ProductName" "randomon"
VIAddVersionKey "FileVersion" "${APPVERSION}"
VIAddVersionKey "ProductVersion" "${APPVERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"
VIAddVersionKey "FileDescription" "randomon installer"

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
  ${If} ${IsNativeARM64}
    File "arm64\randomon.exe"
    File "arm64\randomon-cli.exe"
  ${Else}
    File "x86_64\randomon.exe"
    File "x86_64\randomon-cli.exe"
  ${EndIf}
  File "icon.ico"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\randomon.lnk" "$INSTDIR\randomon.exe" "" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\randomon" "DisplayName" "randomon"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\randomon" "DisplayVersion" "${APPVERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\randomon" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\randomon" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\randomon.exe"
  Delete "$INSTDIR\randomon-cli.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\randomon.lnk"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\randomon"
SectionEnd
