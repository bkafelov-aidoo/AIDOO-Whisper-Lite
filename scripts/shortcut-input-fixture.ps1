param([ValidateSet('f8','f9','control_right','kp_0')][string]$Code)
$ErrorActionPreference = 'Stop'
if (!$IsWindows -or $env:GITHUB_ACTIONS -ne 'true') { throw 'Disposable Windows CI only.' }
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ShortcutInput {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern void keybd_event(byte v, byte s, uint f, UIntPtr e);
}
'@
$app = @(Get-Process -Name aidoo-whisper-lite)[0]
$app.Refresh()
[ShortcutInput]::ShowWindow($app.MainWindowHandle, 9) | Out-Null
[ShortcutInput]::SetForegroundWindow($app.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 150
if ([ShortcutInput]::GetForegroundWindow() -ne $app.MainWindowHandle) { throw 'Test app did not receive foreground focus.' }
$key = switch ($Code) { 'f8' { 0x77 } 'f9' { 0x78 } 'control_right' { 0xa3 } 'kp_0' { 0x60 } }
$flags = if ($Code -eq 'control_right') { 1 } else { 0 }
[ShortcutInput]::keybd_event($key, 0, $flags, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 200
[ShortcutInput]::keybd_event($key, 0, ($flags -bor 2), [UIntPtr]::Zero)
