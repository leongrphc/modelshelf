param(
    [Parameter(Mandatory = $true)][int]$ProcessId,
    [Parameter(Mandatory = $true)]
    [ValidateSet('inspect', 'choose-file', 'choose-folder', 'yes', 'no', 'cancel', 'close-window')]
    [string]$Mode,
    [string]$Path
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ModelShelfDialogNative {
    [DllImport("user32.dll")] public static extern bool IsChild(IntPtr parent, IntPtr child);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, IntPtr wParam, string lParam, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", SetLastError=true)] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
}
'@

if (-not $env:MODELSHELF_TEST_DATA_DIR) {
    throw 'MODELSHELF_TEST_DATA_DIR must identify the isolated test profile.'
}
$testRoot = (Resolve-Path -LiteralPath $env:MODELSHELF_TEST_DATA_DIR).ProviderPath.TrimEnd('\', '/')
$targetProcess = Get-Process -Id $ProcessId -ErrorAction Stop
if ($targetProcess.ProcessName -ne 'modelshelf') {
    throw "Refusing to automate process '$($targetProcess.ProcessName)'. Expected modelshelf."
}

if ($Mode -in @('choose-file', 'choose-folder')) {
    if (-not $Path -or -not [IO.Path]::IsPathRooted($Path)) {
        throw 'An absolute fixture Path is required.'
    }
    $fixturePath = (Resolve-Path -LiteralPath $Path).ProviderPath
    if (-not ($fixturePath.Equals($testRoot, [StringComparison]::OrdinalIgnoreCase) -or
        $fixturePath.StartsWith($testRoot + '\', [StringComparison]::OrdinalIgnoreCase))) {
        throw 'Fixture must be inside MODELSHELF_TEST_DATA_DIR.'
    }
    # Reject junctions/symlinks, including ancestors, so lexical containment is sufficient.
    $ancestor = Get-Item -LiteralPath $fixturePath -Force
    while ($null -ne $ancestor) {
        if ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Reparse-point fixture paths are not allowed: $($ancestor.FullName)"
        }
        $ancestor = if ($ancestor -is [IO.DirectoryInfo]) { $ancestor.Parent } else { $ancestor.Directory }
    }
    $fixture = Get-Item -LiteralPath $fixturePath
    if (($Mode -eq 'choose-folder') -ne $fixture.PSIsContainer) {
        throw 'Fixture type does not match the chosen mode.'
    }
}

$uia = [System.Windows.Automation.AutomationElement]
$scope = [System.Windows.Automation.TreeScope]
$pidCondition = [System.Windows.Automation.PropertyCondition]::new($uia::ProcessIdProperty, $ProcessId)
if ($Mode -eq 'close-window') {
    $titleCondition = [System.Windows.Automation.PropertyCondition]::new($uia::NameProperty, 'ModelShelf')
    $mainCondition = [System.Windows.Automation.AndCondition]::new($pidCondition, $titleCondition)
    $mainWindows = $uia::RootElement.FindAll($scope::Children, $mainCondition)
    if ($mainWindows.Count -ne 1 -or $mainWindows.Item(0).Current.ClassName -eq '#32770') {
        throw 'Expected one ModelShelf main window owned by the provided PID.'
    }
    $mainWindow = $mainWindows.Item(0).GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern)
    $mainWindow.Close()
    return
}
$classCondition = [System.Windows.Automation.PropertyCondition]::new($uia::ClassNameProperty, '#32770')
$dialogCondition = [System.Windows.Automation.AndCondition]::new($pidCondition, $classCondition)
$deadline = [DateTime]::UtcNow.AddSeconds(15)
$dialog = $null
do {
    $targetProcess.Refresh()
    if ($targetProcess.HasExited) { throw 'ModelShelf exited while waiting for a dialog.' }
    $dialogs = $uia::RootElement.FindAll($scope::Children, $dialogCondition)
    if ($dialogs.Count -gt 1) { throw 'Multiple native dialogs found; refusing an ambiguous action.' }
    if ($dialogs.Count -eq 1) { $dialog = $dialogs.Item(0); break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)
if ($null -eq $dialog) { throw 'No native ModelShelf dialog appeared within 15 seconds.' }

function Find-DialogElement([string]$AutomationId, [string]$ClassName) {
    $conditions = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new($uia::AutomationIdProperty, $AutomationId),
        [System.Windows.Automation.PropertyCondition]::new($uia::ClassNameProperty, $ClassName)
    )
    return $dialog.FindFirst($scope::Descendants, $conditions)
}

function Get-ValidatedHandle($Element) {
    $handle = [IntPtr]$Element.Current.NativeWindowHandle
    $dialogHandle = [IntPtr]$dialog.Current.NativeWindowHandle
    [uint32]$ownerProcessId = 0
    [void][ModelShelfDialogNative]::GetWindowThreadProcessId($handle, [ref]$ownerProcessId)
    if ($handle -eq [IntPtr]::Zero -or $ownerProcessId -ne $ProcessId -or
        -not [ModelShelfDialogNative]::IsChild($dialogHandle, $handle)) {
        throw 'Native control handle failed dialog ancestry/process validation.'
    }
    return $handle
}

function Get-DialogDiagnostic {
    $elements = $dialog.FindAll($scope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    $rows = foreach ($element in $elements) {
        if (-not $element.Current.AutomationId.StartsWith('System.') -and
            $element.Current.ClassName -in @('Edit', 'Button', 'ComboBox')) {
            '{0}: id={1}, name={2}, enabled={3}' -f $element.Current.ControlType.ProgrammaticName,
                $element.Current.AutomationId, $element.Current.Name, $element.Current.IsEnabled
        }
    }
    return (($rows | Select-Object -First 40) -join '; ')
}

function Invoke-DialogButton([string]$AutomationId) {
    do {
        $button = Find-DialogElement $AutomationId 'Button'
        $pattern = $null
        if ($null -ne $button -and $button.Current.IsEnabled -and
            $button.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
            $pattern.Invoke()
            return
        }
        if ($null -ne $button -and $button.Current.IsEnabled) {
            $handle = Get-ValidatedHandle $button
            if (-not [ModelShelfDialogNative]::PostMessage($handle, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)) {
                throw 'Failed to post BM_CLICK to the validated dialog button.'
            }
            return
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Enabled native dialog button '$AutomationId' was not ready within 15 seconds. Controls: $(Get-DialogDiagnostic)"
}

switch ($Mode) {
    'inspect' {
        $elements = $dialog.FindAll($scope::Subtree, [System.Windows.Automation.Condition]::TrueCondition)
        $rows = foreach ($element in $elements) {
            [pscustomobject]@{
                Name = $element.Current.Name
                ControlType = $element.Current.ControlType.ProgrammaticName
                AutomationId = $element.Current.AutomationId
                ClassName = $element.Current.ClassName
                IsEnabled = $element.Current.IsEnabled
            }
        }
        ConvertTo-Json -InputObject @($rows) -Depth 3
    }
    { $_ -in @('choose-file', 'choose-folder') } {
        $value = $null
        do {
            $edit = Find-DialogElement '1148' 'Edit'
            if ($null -eq $edit -and $Mode -eq 'choose-folder') { $edit = Find-DialogElement '1152' 'Edit' }
            if ($null -eq $edit) { $edit = Find-DialogElement '1001' 'Edit' }
            if ($null -ne $edit -and $edit.Current.IsEnabled) {
                if ($edit.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$value) -and
                    -not $value.Current.IsReadOnly) { break }
                $value = $null
                break
            }
            $value = $null
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        if ($null -eq $edit -or -not $edit.Current.IsEnabled) {
            throw "Native dialog filename/folder edit (1148 or 1001) was not ready within 15 seconds. Controls: $(Get-DialogDiagnostic)"
        }
        if ($null -ne $value) { $value.SetValue($fixturePath) }
        else {
            $handle = Get-ValidatedHandle $edit
            $nativeResult = [IntPtr]::Zero
            $sent = [ModelShelfDialogNative]::SendMessageTimeout($handle, 0x000C, [IntPtr]::Zero,
                $fixturePath, 0x0002, 2000, [ref]$nativeResult)
            if ($sent -eq [IntPtr]::Zero -or $nativeResult -eq [IntPtr]::Zero) {
                throw 'Failed to set text on the validated dialog edit.'
            }
        }
        Invoke-DialogButton '1'
    }
    'yes' { Invoke-DialogButton '6' }
    'no' { Invoke-DialogButton '7' }
    'cancel' { Invoke-DialogButton '2' }
}
