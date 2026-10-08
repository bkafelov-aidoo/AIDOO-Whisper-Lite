param([Parameter(Mandatory)][string]$ResultPath, [Parameter(Mandatory)][string]$ReadyPath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class InputDesktop {
  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern IntPtr LoadKeyboardLayout(string name, uint flags);
  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr window);
}
'@
$layout = [InputDesktop]::LoadKeyboardLayout('00000402', 1)
if ($layout -eq [IntPtr]::Zero) { throw 'Could not activate the Bulgarian keyboard layout.' }
$form = New-Object System.Windows.Forms.Form
$form.Text = 'AIDOO Windows input test'
$form.Width = 640
$form.Height = 180
$field = New-Object System.Windows.Forms.TextBox
$field.Dock = 'Fill'
$field.Multiline = $true
$form.Controls.Add($field)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 100
$deadline = [DateTime]::UtcNow.AddSeconds(45)
$timer.Add_Tick({
    if ($field.Text -or [DateTime]::UtcNow -gt $deadline) { $form.Close() }
})
$form.Add_Shown({
    $form.Activate()
    [InputDesktop]::SetForegroundWindow($form.Handle) | Out-Null
    $field.Focus() | Out-Null
    [IO.File]::WriteAllText($ReadyPath, 'ready')
    $timer.Start()
})
[System.Windows.Forms.Application]::Run($form)
$timer.Stop()
@{ text = $field.Text; keyboardCulture = [System.Windows.Forms.InputLanguage]::CurrentInputLanguage.Culture.Name } |
    ConvertTo-Json | Set-Content -Encoding utf8 $ResultPath
$form.Dispose()
