; RenPy Studio — установщик Inno Setup 6.
;
; Собирается скриптом scripts\build-release.ps1, который передаёт версию:
;   ISCC.exe /DAppVersion=0.1.1 renpy-studio.iss
;
; Что делает установщик:
;   * ставит приложение в профиль пользователя (права администратора не нужны);
;   * проверяет WebView2 Runtime и при отсутствии ставит его из вложенного
;     MicrosoftEdgeWebview2Setup.exe (скачивается один раз при сборке);
;   * связывает файлы .rpyproj с RenPy Studio;
;   * добавляет ярлыки в меню «Пуск» и (по желанию) на рабочий стол.
;
; Ren'Py SDK в установщик не входит: студия находит его сама или просит путь.

#define AppName "RenPy Studio"
#define AppExe "renpy-studio.exe"
#define AppPublisher "RenPy Studio"
#define AppUrl "https://github.com/0x-dorothy/renpystudio"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\src-tauri\target\release\renpy-studio.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\dist"
#endif
#define WebView2Setup "cache\MicrosoftEdgeWebview2Setup.exe"

[Setup]
; AppId нельзя менять между версиями: по нему Windows понимает, что это обновление.
AppId={{7C4E2B1A-9D3F-4A62-8E51-5B0C7D9A1F24}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
; per-user установка по умолчанию; «для всех» — только если запустили от админа
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=RenPyStudio-{#AppVersion}-setup
SetupIconFile=..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName} {#AppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
ChangesAssociations=yes
CloseApplications=yes
RestartApplications=no
DisableWelcomePage=no
VersionInfoVersion={#AppVersion}.0
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} installer
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}
#ifdef SignedUninstaller
SignedUninstaller=yes
#endif

[Languages]
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
FileAssocGroup=File association
en.FileAssocGroup=File association
ru.FileAssocGroup=Связь с файлами проектов

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "associate"; Description: "{cm:AssocFileExtension,{#AppName},.rpyproj}"; GroupDescription: "{cm:FileAssocGroup}"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
#if FileExists(AddBackslash(SourcePath) + WebView2Setup)
Source: "{#WebView2Setup}"; DestDir: "{tmp}"; Flags: dontcopy deleteafterinstall
#endif

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Ассоциация .rpyproj — маркер проекта RenPy Studio.
Root: HKA; Subkey: "Software\Classes\.rpyproj"; ValueType: string; ValueName: ""; ValueData: "RenPyStudio.Project"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RenPyStudio.Project"; ValueType: string; ValueName: ""; ValueData: "RenPy Studio Project"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RenPyStudio.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: associate
Root: HKA; Subkey: "Software\Classes\RenPyStudio.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: associate
; Чтобы «Открыть с помощью» показывал студию, даже если ассоциация снята.
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "{#AppName}"; Flags: uninsdeletekey; Tasks: associate
Root: HKA; Subkey: "Software\Classes\Applications\{#AppExe}\SupportedTypes"; ValueType: string; ValueName: ".rpyproj"; ValueData: ""; Tasks: associate

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[Code]
const
  SHCNE_ASSOCCHANGED = $08000000;
  SHCNF_IDLIST = $0000;

procedure SHChangeNotify(EventId: LongWord; Flags: LongWord; Item1, Item2: LongWord);
  external 'SHChangeNotify@shell32.dll stdcall';

function RuntimeUnder(BaseDir: String): Boolean;
var
  FindRec: TFindRec;
  Dir: String;
begin
  Result := False;
  if not DirExists(BaseDir) then
    Exit;
  if FindFirst(AddBackslash(BaseDir) + '*', FindRec) then
  begin
    try
      repeat
        if (FindRec.Attributes and FILE_ATTRIBUTE_DIRECTORY <> 0) and
           (FindRec.Name <> '.') and (FindRec.Name <> '..') then
        begin
          Dir := AddBackslash(AddBackslash(BaseDir) + FindRec.Name);
          if FileExists(Dir + 'msedgewebview2.exe') then
          begin
            Result := True;
            Exit;
          end;
        end;
      until not FindNext(FindRec);
    finally
      FindClose(FindRec);
    end;
  end;
end;

function WebView2Installed(): Boolean;
begin
  Result :=
    RuntimeUnder(ExpandConstant('{commonpf32}\Microsoft\EdgeWebView\Application')) or
    RuntimeUnder(ExpandConstant('{commonpf}\Microsoft\EdgeWebView\Application')) or
    RuntimeUnder(ExpandConstant('{localappdata}\Microsoft\EdgeWebView\Application'));
end;

function Msg(EN, RU: String): String;
begin
  if ActiveLanguage = 'ru' then
    Result := RU
  else
    Result := EN;
end;

// Возвращает '' если можно продолжать, иначе текст ошибки для установщика.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
  Setup: String;
begin
  Result := '';
  if WebView2Installed() then
    Exit;

  Setup := ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe');
  if FileExists(Setup) then
  begin
    ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
    WizardForm.StatusLabel.Caption :=
      Msg('Installing Microsoft Edge WebView2 Runtime...',
          'Устанавливается компонент Microsoft Edge WebView2...');
    if (not Exec(Setup, '/silent /install', '', SW_SHOW, ewWaitUntilTerminated, Code)) or (Code <> 0) then
    begin
      if not WebView2Installed() then
        Result := Msg('WebView2 Runtime could not be installed automatically. ' +
                      'Install it from https://developer.microsoft.com/microsoft-edge/webview2/ and run setup again.',
                      'Не удалось установить WebView2 автоматически. Установите его со страницы ' +
                      'https://developer.microsoft.com/microsoft-edge/webview2/ и запустите установку снова.');
    end;
    Exit;
  end;

  if MsgBox(Msg('RenPy Studio needs the Microsoft Edge WebView2 Runtime, which is missing on this PC.' + #13#10 +
                'Open the download page now?',
                'Для RenPy Studio нужен компонент Microsoft Edge WebView2, а его нет на этом компьютере.' + #13#10 +
                'Открыть страницу загрузки?'),
            mbConfirmation, MB_YESNO) = IDYES then
  begin
    ShellExec('open', 'https://developer.microsoft.com/microsoft-edge/webview2/', '', '', SW_SHOWNORMAL, ewNoWait, Code);
    Result := Msg('Install WebView2 and run this setup again.',
                  'Установите WebView2 и запустите установку снова.');
  end
  else
    Result := Msg('WebView2 Runtime is required to run RenPy Studio.',
                  'Для работы RenPy Studio необходим компонент WebView2.');
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    if WizardIsTaskSelected('associate') then
      SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, 0, 0);
end;
