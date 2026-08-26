#define MyAppName "剪贴板"
#define MyAppVersion "0.9.9"
#define MyAppPublisher "qiu7824"
#define MyAppURL "https://github.com/qiu7824/zsclip"
#define MyAppExeName "剪贴板.exe"
#define MyAppInstallDirName "剪贴板"
#define MyAppAutoStartValueName "ZSClip"
#define MySourceDir "E:\rust\zsclip\dist\installer-source"
#define MyOutputDir "E:\rust\zsclip\dist"
#define MySetupIcon "E:\rust\zsclip\assets\icons\icon.ico"

[Setup]
AppId={{8D6C8C7D-1F0A-4E5A-9A24-7F4F5C1E9A11}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={code:GetDefaultInstallDir}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir={#MyOutputDir}
OutputBaseFilename=zsclip-v{#MyAppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupLogging=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
CloseApplications=yes
CloseApplicationsFilter={#MyAppExeName}
RestartApplications=no
AppMutex=Global\ZSClipSingleInstance
VersionInfoVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoProductName={#MyAppName}
VersionInfoDescription={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}
SetupIconFile={#MySetupIcon}

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: unchecked
Name: "autostart"; Description: "启用开机自启"; GroupDescription: "启动选项:"; Flags: checkedonce

[Files]
Source: "{#MySourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Dirs]
Name: "{app}\data"
Name: "{app}\data\temp"
Name: "{app}\data\logs"

[InstallDelete]
Type: files; Name: "{autoprograms}\ZSClip.lnk"
Type: files; Name: "{autodesktop}\ZSClip.lnk"

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#MyAppAutoStartValueName}"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "立即运行 {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\logs"
Type: filesandordirs; Name: "{app}\temp"

[Code]
function GetDefaultInstallDir(Param: string): string;
begin
  if DirExists('D:\') then
    Result := 'D:\{#MyAppInstallDirName}'
  else if DirExists('E:\') then
    Result := 'E:\{#MyAppInstallDirName}'
  else if DirExists('F:\') then
    Result := 'F:\{#MyAppInstallDirName}'
  else
    Result := ExpandConstant('{autopf}\{#MyAppInstallDirName}');
end;
