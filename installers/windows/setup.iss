[Setup]
AppName=ExeOutput Studio
AppVersion=1.0.0
DefaultDirName={pf}\ExeOutput Studio
DefaultGroupName=ExeOutput Studio
OutputDir=..\..\dist
OutputBaseFilename=exeoutput-setup
SetupIconFile=..\..\src-tauri\icons\icon.ico
Compression=lzma
SolidCompression=yes

[Files]
Source: "..\..\src-tauri\target\release\exeoutput-studio.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\src-tauri\target\release\*.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ExeOutput Studio"; Filename: "{app}\exeoutput-studio.exe"
Name: "{commondesktop}\ExeOutput Studio"; Filename: "{app}\exeoutput-studio.exe"
