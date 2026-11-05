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
  private delegate bool WindowCallback(IntPtr window, IntPtr data);
  [StructLayout(LayoutKind.Sequential)] private struct Rect { public int left, top, right, bottom; }
  [DllImport("user32.dll")] private static extern bool EnumWindows(WindowCallback callback, IntPtr data);
  [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
  [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out Rect rect);
  public static IntPtr MainWindow(int process) {
    IntPtr selected=IntPtr.Zero; long largest=0;
    // Inspect only this disposable test app. The overlay is smaller and cannot take focus.
    EnumWindows((window,unused)=>{
      uint owner; Rect rect; GetWindowThreadProcessId(window,out owner);
      if(owner==(uint)process && IsWindowVisible(window) && GetWindowRect(window,out rect)) {
        long area=(long)(rect.right-rect.left)*(rect.bottom-rect.top);
        if(area>largest) { largest=area; selected=window; }
      }
      return true;
    },IntPtr.Zero);
    return selected;
  }
  [StructLayout(LayoutKind.Sequential)] public struct KeyInput { public ushort vk; public ushort scan; public uint flags; public uint time; public UIntPtr extra; }
  [StructLayout(LayoutKind.Explicit, Size=40)] public struct Input { [FieldOffset(0)] public uint type; [FieldOffset(8)] public KeyInput key; }
  [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, Input[] input, int size);
  [DllImport("user32.dll")] public static extern uint MapVirtualKeyW(uint code, uint mapType);
  public static void Press(ushort vk, uint flags) {
    var input = new Input { type=1, key=new KeyInput { vk=vk, scan=(ushort)MapVirtualKeyW(vk,0), flags=flags|8 } };
    if(SendInput(1, new[]{input}, 40)!=1) throw new Exception("Synthetic input was rejected.");
  }
}
'@
$apps = @(Get-Process -Name aidoo-whisper-lite)
if ($apps.Count -ne 1) { throw 'Expected exactly one disposable test application.' }
$app = $apps[0]
$app.Refresh()
$main = [ShortcutInput]::MainWindow($app.Id)
if ($main -eq [IntPtr]::Zero) { throw 'Test app has no visible main window.' }
$legacySelectionDiffers = $app.MainWindowHandle -ne $main
[ShortcutInput]::ShowWindow($main, 9) | Out-Null
[ShortcutInput]::SetForegroundWindow($main) | Out-Null
Start-Sleep -Milliseconds 150
if ([ShortcutInput]::GetForegroundWindow() -ne $main) { throw 'Test app main window did not receive foreground focus.' }
$key = switch ($Code) { 'f8' { 0x77 } 'f9' { 0x78 } 'control_right' { 0xa3 } 'kp_0' { 0x60 } }
$flags = if ($Code -eq 'control_right') { 1 } else { 0 }
[ShortcutInput]::Press($key, $flags)
Start-Sleep -Milliseconds 200
[ShortcutInput]::Press($key, ($flags -bor 2))
@{ focusedMain = $true; legacySelectionDiffers = $legacySelectionDiffers } | ConvertTo-Json -Compress
