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
  [DllImport("user32.dll")]
  public static extern IntPtr GetForegroundWindow();
}
'@
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
function Write-InputEvidence([string]$Failure = '') {
    @{ text = $field.Text; keyboardCulture = [System.Windows.Forms.InputLanguage]::CurrentInputLanguage.Culture.Name; error = $Failure } |
        ConvertTo-Json | Set-Content -Encoding utf8 $ResultPath
}
$timer.Add_Tick({
    if ($field.Text -or [DateTime]::UtcNow -gt $deadline) {
        # Closing a WinForms form disposes its controls and clears TextBox.Text.
        Write-InputEvidence
        $form.Close()
    }
})
$form.Add_Shown({
    try {
        $form.Activate()
        [InputDesktop]::SetForegroundWindow($form.Handle) | Out-Null
        $field.Focus() | Out-Null
        if ([InputDesktop]::GetForegroundWindow() -ne $form.Handle -or !$field.Focused) {
            throw 'The Windows test field did not receive keyboard focus.'
        }
        # Windows 8+ requires this process to own keyboard focus before loading a layout.
        $layout = [InputDesktop]::LoadKeyboardLayout('00000402', 1)
        if ($layout -eq [IntPtr]::Zero) { throw 'Could not load the Bulgarian keyboard layout.' }
        if ([System.Windows.Forms.InputLanguage]::CurrentInputLanguage.Culture.Name -ne 'bg-BG') {
            throw 'Could not activate the Bulgarian keyboard layout.'
        }
        [IO.File]::WriteAllText($ReadyPath, 'ready')
        $timer.Start()
    } catch {
        Write-InputEvidence $_.Exception.Message
        $form.Close()
    }
})
[System.Windows.Forms.Application]::Run($form)
$timer.Stop()
if (!(Test-Path $ResultPath)) { Write-InputEvidence 'The test field closed before receiving text.' }
$form.Dispose()
