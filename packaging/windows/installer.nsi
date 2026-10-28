; The Windows installer. Build the release exe first, then:
;   makensis /DVERSION=1.0.0 packaging\windows\installer.nsi
; It installs for the current user (no admin rights), adds a Start menu entry and lists
; DystoPad under "Open with" for .txt and .md files without taking over the default app.

Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !error "Pass the version: makensis /DVERSION=x.y.z"
!endif
!define PROGID "DystoPad.Document"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\DystoPad"

Name "DystoPad"
OutFile "..\..\target\release\DystoPad_${VERSION}_x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\DystoPad"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "DystoPad"
VIAddVersionKey "FileDescription" "DystoPad setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"

!define MUI_ICON "..\..\assets\icon\icon.ico"
!define MUI_UNICON "..\..\assets\icon\icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\DystoPad.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "German"

!macro OpenWith EXT
  WriteRegStr HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}" ""
!macroend
!macro NoOpenWith EXT
  DeleteRegValue HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}"
!macroend

Section
  SetOutPath "$INSTDIR"
  File "/oname=DystoPad.exe" "..\..\target\release\dystopad.exe"
  File "..\..\LICENSE"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\DystoPad.lnk" "$INSTDIR\DystoPad.exe"

  WriteRegStr HKCU "Software\Classes\${PROGID}" "" "Text document"
  WriteRegStr HKCU "Software\Classes\${PROGID}\DefaultIcon" "" "$INSTDIR\DystoPad.exe,0"
  WriteRegStr HKCU "Software\Classes\${PROGID}\shell\open\command" "" '"$INSTDIR\DystoPad.exe" "%1"'
  !insertmacro OpenWith "txt"
  !insertmacro OpenWith "md"
  !insertmacro OpenWith "markdown"

  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "DystoPad"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "YodelingGoethe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\DystoPad.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ; Tells Explorer the file associations changed.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\DystoPad.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\DystoPad.lnk"
  !insertmacro NoOpenWith "txt"
  !insertmacro NoOpenWith "md"
  !insertmacro NoOpenWith "markdown"
  DeleteRegKey HKCU "Software\Classes\${PROGID}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
