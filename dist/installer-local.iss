#define MyAppName "剪贴板"
#define MyAppVersion "0.9.9.3"
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
Name: "chinesesimplified"; MessagesFile: "E:\rust\zsclip\.github\inno\ChineseSimplified.isl"

[Messages]
ButtonBack=< 上一步(&B)
ButtonNext=下一步(&N) >
ButtonInstall=安装(&I)
ButtonOK=确定
ButtonCancel=取消
ButtonYes=是(&Y)
ButtonNo=否(&N)
ButtonFinish=完成(&F)
ButtonBrowse=浏览(&B)...
ButtonWizardBrowse=浏览(&R)...
ClickNext=单击“下一步”继续，或单击“取消”退出安装程序。
WelcomeLabel1=欢迎使用 [name] 安装向导
WelcomeLabel2=这将在你的电脑上安装 [name/ver]。%n%n建议继续前关闭其他应用程序。
WizardSelectDir=选择安装位置
SelectDirDesc=要将 [name] 安装到哪里？
SelectDirLabel3=安装程序会将 [name] 安装到以下文件夹。
SelectDirBrowseLabel=单击“下一步”继续。如需选择其他文件夹，请单击“浏览”。
DiskSpaceMBLabel=至少需要 [mb] MB 可用磁盘空间。
DirExistsTitle=文件夹已存在
DirExists=文件夹：%n%n%1%n%n已经存在。是否仍安装到该文件夹？
DirDoesntExistTitle=文件夹不存在
DirDoesntExist=文件夹：%n%n%1%n%n不存在。是否创建该文件夹？
WizardSelectTasks=选择附加任务
SelectTasksDesc=要执行哪些附加任务？
SelectTasksLabel2=请选择安装 [name] 时要执行的附加任务，然后单击“下一步”。
WizardReady=准备安装
ReadyLabel1=安装程序已准备好开始将 [name] 安装到你的电脑。
ReadyLabel2a=单击“安装”继续，或单击“上一步”检查或修改设置。
ReadyLabel2b=单击“安装”继续。
ReadyMemoDir=安装位置：
ReadyMemoGroup=开始菜单文件夹：
ReadyMemoTasks=附加任务：
WizardPreparing=正在准备安装
PreparingDesc=安装程序正在准备将 [name] 安装到你的电脑。
WizardInstalling=正在安装
InstallingLabel=请稍候，安装程序正在将 [name] 安装到你的电脑。
FinishedHeadingLabel=[name] 安装向导完成
FinishedLabelNoIcons=安装程序已完成 [name] 的安装。
FinishedLabel=安装程序已完成 [name] 的安装。你可以通过已安装的快捷方式启动应用。
ClickFinish=单击“完成”退出安装程序。
RunEntryExec=运行 %1
StatusCreateDirs=正在创建目录...
StatusExtractFiles=正在解压文件...
StatusCreateIcons=正在创建快捷方式...
StatusCreateRegistryEntries=正在写入注册表...
StatusSavingUninstall=正在保存卸载信息...
StatusRunProgram=正在完成安装...

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"; Flags: unchecked
Name: "autostart"; Description: "启用开机自启"; GroupDescription: "启动选项:"; Flags: unchecked

[Files]
Source: "{#MySourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "\Output\*,\.git\*,\.github\*,\target\*,\__pycache__\*,\build\*,\dist\*,\installer\*,\venv\*,\.venv\*,\node_modules\*,\*.iss,*.pyc,*.pyo"

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
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "{#MyAppAutoStartValueName}"; Flags: deletevalue; Check: ShouldDisableAutostart
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "{#MyAppName}"; Flags: deletevalue; Check: ShouldDisableAutostart
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Clipboard"; Flags: deletevalue; Check: ShouldDisableAutostart
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "筑森剪贴"; Flags: deletevalue; Check: ShouldDisableAutostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "立即运行 {#MyAppName}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\logs"
Type: filesandordirs; Name: "{app}\temp"

[Code]
var
  AutoStartTaskInitialized: Boolean;

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

function RunValuePresent(ValueName: string): Boolean;
var
  Value: string;
begin
  Result := RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', ValueName, Value) and (Trim(Value) <> '');
end;

function ExistingAutostartEnabled(): Boolean;
begin
  Result := RunValuePresent('{#MyAppAutoStartValueName}') or RunValuePresent('{#MyAppName}') or RunValuePresent('Clipboard') or RunValuePresent('筑森剪贴');
end;

function ShouldDisableAutostart(): Boolean;
begin
  Result := not WizardIsTaskSelected('autostart');
end;

function FindTaskByCaption(Caption: string): Integer;
var
  I: Integer;
begin
  Result := -1;
  for I := 0 to WizardForm.TasksList.Items.Count - 1 do
  begin
    if WizardForm.TasksList.ItemCaption[I] = Caption then
    begin
      Result := I;
      Exit;
    end;
  end;
end;

procedure CurPageChanged(CurPageID: Integer);
var
  TaskIndex: Integer;
begin
  if (CurPageID = wpSelectTasks) and (not AutoStartTaskInitialized) then
  begin
    TaskIndex := FindTaskByCaption('启用开机自启');
    if TaskIndex >= 0 then
      WizardForm.TasksList.Checked[TaskIndex] := ExistingAutostartEnabled();
    AutoStartTaskInitialized := True;
  end;
end;