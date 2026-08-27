; Inno Setup script for Browser Picker — per-user install, no admin required.
; Version is passed by CI:  ISCC /DMyAppVersion=x.y.z installer\browser-picker.iss
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0-dev"
#endif
#define MyAppName "Browser Picker"
#define MyAppPublisher "Soneji"
#define MyAppURL "https://github.com/Soneji/browser-picker"
#define MyAppExeName "browser-picker.exe"

[Setup]
AppId={{8F2C1E7A-4B93-4E51-9C2A-1D7F6B0A3E45}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
; Per-user install: no UAC prompt, and matches the app's HKCU registration.
PrivilegesRequired=lowest
OutputDir=.
OutputBaseFilename=browser-picker-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

[Files]
Source: "..\browser-picker.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"

[Run]
; Register as a selectable browser (writes HKCU, no admin needed).
Filename: "{app}\{#MyAppExeName}"; Parameters: "--register"; Flags: runhidden; StatusMsg: "Registering Browser Picker..."
; Offer to open the app (its home window has a 'Set as default browser' button).
Filename: "{app}\{#MyAppExeName}"; Description: "Set {#MyAppName} as your default browser now"; Flags: postinstall nowait skipifsilent

[UninstallRun]
; Remove the browser registration before the files are deleted.
Filename: "{app}\{#MyAppExeName}"; Parameters: "--unregister"; Flags: runhidden; RunOnceId: "UnregisterBrowserPicker"
