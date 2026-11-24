param([Parameter(Mandatory=$true)][string]$Exe)
$ErrorActionPreference='Stop'
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class StartupWindow {
 public delegate bool Callback(IntPtr h, IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(Callback c,IntPtr p);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 public struct Rect { public int Left,Top,Right,Bottom; }
 public static IntPtr Find(int pid) { IntPtr result=IntPtr.Zero; EnumWindows((h,p)=> {uint id;GetWindowThreadProcessId(h,out id); if(id==pid) {Rect r;GetClientRect(h,out r);if(r.Right>=900 && r.Bottom>=600)result=h;}return true;},IntPtr.Zero);return result; }
}
"@
$testRoot=Join-Path $PSScriptRoot '../startup-test-profile'
New-Item -ItemType Directory -Force -Path $testRoot | Out-Null
$testRoot=(Resolve-Path $testRoot).Path
$env:APPDATA=Join-Path $testRoot 'roaming'
$env:LOCALAPPDATA=Join-Path $testRoot 'local'
$env:DSS_TEST_PROFILE=Join-Path $testRoot 'database'
$env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $testRoot 'webview'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9224'
$config=$env:DSS_TEST_PROFILE
New-Item -ItemType Directory -Force -Path $config | Out-Null
$results=@()
foreach($case in @('normal','maximized','first-launch')) {
 $state=Join-Path $config '.window-state.json'
 if($case -eq 'first-launch') {if(Test-Path $state){Remove-Item -LiteralPath $state}} else {
  @{main=@{width=1100;height=700;x=90;y=80;prev_x=90;prev_y=80;maximized=($case -eq 'maximized');visible=$true;decorated=$true;fullscreen=$false}} | ConvertTo-Json | Set-Content -LiteralPath $state
 }
 $p=Start-Process -FilePath $Exe -PassThru -WindowStyle Hidden
 $timer=[Diagnostics.Stopwatch]::StartNew();$hiddenSeen=$false;$visible=$false;$observations=0
 while($timer.Elapsed.TotalSeconds -lt 45) {
  $h=[StartupWindow]::Find($p.Id)
  if($h -ne [IntPtr]::Zero) {
   $observations++
   if([StartupWindow]::IsWindowVisible($h)) {$visible=$true;break} else {$hiddenSeen=$true}
  }
  if($p.HasExited){throw "App exited during $case startup"}
  Start-Sleep -Milliseconds 5
 }
 if(!$visible){throw "No visible window in $case"}
 $rect=New-Object StartupWindow+Rect;$client=New-Object StartupWindow+Rect
 [void][StartupWindow]::GetWindowRect($h,[ref]$rect);[void][StartupWindow]::GetClientRect($h,[ref]$client)
 $max=[StartupWindow]::IsZoomed($h)
 if($case -eq 'normal' -and ($client.Right -ne 1100 -or $client.Bottom -ne 700 -or $rect.Left -ne 90 -or $rect.Top -ne 80)){throw "First visible bounds incorrect: $($rect|ConvertTo-Json -Compress), client $($client|ConvertTo-Json -Compress)"}
 if($case -in @('maximized','first-launch') -and !$max){throw "First visible window was not maximized in $case"}
 & node (Join-Path $PSScriptRoot 'startup_content.cjs')
 if($LASTEXITCODE -ne 0){throw 'Content verification failed'}
 $results+=@{case=$case;hiddenObserved=$hiddenSeen;observations=$observations;firstVisibleMs=$timer.ElapsedMilliseconds;outer=$rect;client=$client;maximized=$max}
 [void][StartupWindow]::PostMessage($h,0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
 if(!$p.WaitForExit(15000)){throw 'App did not close cleanly'}
 Start-Sleep -Milliseconds 500
}
$results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $PSScriptRoot '../docs/v3-startup-results.json')
$results | ConvertTo-Json -Depth 5
