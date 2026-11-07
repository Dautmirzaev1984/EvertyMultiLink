#define AppName "Everty MultiLink"
#ifdef TestInstallMode
  #define SetupId "{{26D64584-2A76-423F-AD30-CA0EAD6305C4}"
  #define InstallName "Everty MultiLink Setup QA"
#else
  #define SetupId "{{8E20FF9A-17EB-42AA-B671-C5F119510BD4}"
  #define InstallName AppName
#endif

[Setup]
AppId={#SetupId}
AppName={#InstallName}
AppVersion={#AppVersion}
AppPublisher=Arthur Valiev
AppPublisherURL=https://github.com/vaalimusic/EvertyMultiLink
AppSupportURL=https://github.com/vaalimusic/EvertyMultiLink/issues
DefaultDirName={autopf}\{#InstallName}
DefaultGroupName={#InstallName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\multilink-app.exe
SetupIconFile={#StageDir}\app.ico
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.17763
#ifdef TestInstallMode
PrivilegesRequired=lowest
AppMutex=Local\Everty.MultiLink.InstallerQA
#else
PrivilegesRequired=admin
AppMutex=Local\Everty.MultiLink.Application
#endif
CloseApplications=no
RestartApplications=no
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Messages]
english.SetupAppRunningError=Everty MultiLink is running.%n%nChoose Exit from its notification area menu, then click OK.
english.UninstallAppRunningError=Everty MultiLink is running.%n%nChoose Exit from its notification area menu, then click OK.
russian.SetupAppRunningError=Everty MultiLink запущен.%n%nВыберите «Выход» в меню значка в трее, затем нажмите OK.
russian.UninstallAppRunningError=Everty MultiLink запущен.%n%nВыберите «Выход» в меню значка в трее, затем нажмите OK.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#StageDir}\multilink-app.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\mihomo.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\Everty-MultiLink.LICENSE.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "{#StageDir}\Mihomo.LICENSE.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "{#StageDir}\Meta-Rules-Dat.LICENSE.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "{#StageDir}\Third-party.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "{#StageDir}\mihomo-v1.19.33-source.zip"; DestDir: "{app}\Licenses"; Flags: ignoreversion

[Icons]
Name: "{group}\{#InstallName}"; Filename: "{app}\multilink-app.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\{#InstallName}"; Filename: "{app}\multilink-app.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Code]
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Entry, InstalledCommand: String;
begin
  if CurUninstallStep = usUninstall then
  begin
    InstalledCommand := '"' + ExpandConstant('{app}\multilink-app.exe') + '" --start-hidden';
    if RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run',
      'Everty MultiLink', Entry) and (CompareText(Entry, InstalledCommand) = 0) then
      RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Everty MultiLink');
  end;
end;
