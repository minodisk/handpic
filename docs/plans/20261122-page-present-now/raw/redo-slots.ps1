# Redo of the slots run-ab-gpui.ps1 left empty after 4 tries, in the given order,
# from try 5 on, with the same functions and gates. -Slots like 'RAF-A2','RAF-B1'.
# The rest of run-ab-gpui.ps1's header:
# Same-session A/B of page-turn latency of two GPUI builds: A = before, B = after.
# Per folder: one throwaway warm run of each, then
# A B A B A B. Each run launches the app on the folder, lets latprobe press
# Right $Presses times at 1003 ms and measure keypress to DWM present, then
# closes it. A run with a load sample over 15 % after the first, or a
# compiler process, is discarded and redone in its slot.
param(
    [Parameter(Mandatory)] [string]$Gpui,
    [Parameter(Mandatory)] [string]$GpuiB,
    [Parameter(Mandatory)] [string]$Probe,
    [Parameter(Mandatory)] [string]$Out,
    [string]$Root = 'D:\Photos\tests\20261108-gpui-compare',
    [string[]]$Folders = @('ARW', 'RAF'),
    [int]$Runs = 3,
    [int]$Presses = 60,
    [int]$SettleMs = 8000,
    [Parameter(Mandatory)] [string[]]$Slots,
    [int]$FirstTry = 5
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
$busyNames = @('cargo', 'rustc', 'link', 'msbuild', 'cl')

Add-Type -Namespace W -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, System.UIntPtr e);'
function Wake { [W.M]::mouse_event(1, 1, 0, 0, [UIntPtr]::Zero); [W.M]::mouse_event(1, -1, 0, 0, [UIntPtr]::Zero) }
# Keep the display and the system awake for the whole session (a slept display
# delivers no Desktop Duplication frame): ES_CONTINUOUS | ES_DISPLAY_REQUIRED |
# ES_SYSTEM_REQUIRED. This changes no power setting.
Add-Type -Namespace W -Name P -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint f);'
[W.P]::SetThreadExecutionState([uint32]2147483651) | Out-Null

function Get-Load { (Get-CimInstance Win32_Processor | Measure-Object LoadPercentage -Average).Average }
function Get-Busy { @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $busyNames -contains $_.Name } | ForEach-Object Name) }

function Wait-Idle {
    for ($i = 0; $i -lt 200; $i++) {
        $load = Get-Load
        if ($load -le 15 -and (Get-Busy).Count -eq 0) { return $load }
        Start-Sleep -Seconds 3
    }
    throw "the machine did not go idle (load $load %)"
}

function Invoke-Run([string]$app, [string]$folder, [string]$name) {
    $dir = Join-Path $Root $folder
    $before = Wait-Idle
    Wake
    $stdout = Join-Path $Out "$name.stdout.txt"
    $stderr = Join-Path $Out "$name.stderr.txt"
    $exe = if ($app -eq 'A') { $Gpui } else { $GpuiB }
    $argList = @("`"$dir`"", '--no-prefetch',
        '--index', "`"$(Join-Path $Out "$app-index.sqlite")`"",
        '--settings', "`"$(Join-Path $Out "$app-settings.sqlite")`"",
        '--log', "`"$(Join-Path $Out "$name.app.log")`"")
    $proc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $samples = New-Object System.Collections.Generic.List[string]
    $probeOut = Join-Path $Out "$name.probe.txt"
    $probe = Start-Process -FilePath $Probe -PassThru -NoNewWindow `
        -ArgumentList @('--pid', $proc.Id, '--presses', $Presses, '--settle-ms', $SettleMs,
            '--out', "`"$(Join-Path $Out "$name.csv")`"", '--close') `
        -RedirectStandardOutput $probeOut -RedirectStandardError (Join-Path $Out "$name.probe.err.txt")
    # Caching the handle keeps ExitCode readable after the exit.
    $null = $probe.Handle
    $bad = @()
    $first = $true
    while (-not $probe.HasExited) {
        $load = Get-Load
        $busy = Get-Busy
        $samples.Add("$load $($busy -join ',')")
        if (-not $first -and $load -gt 15) { $bad += "load $load" }
        if ($busy.Count -gt 0) { $bad += "busy $($busy -join ',')" }
        $first = $false
        Start-Sleep -Seconds 3
    }
    $probe.WaitForExit()
    if (-not $proc.WaitForExit(20000)) { Stop-Process -Id $proc.Id -Force }
    if ($probe.ExitCode -ne 0) { $bad += "probe exit $($probe.ExitCode)" }
    $summary = (Get-Content $probeOut -ErrorAction SilentlyContinue | Select-String '^summary') -join ''
    $line = "$name before=$before samples=[$($samples -join '; ')] bad=[$($bad -join '; ')] $summary"
    Add-Content (Join-Path $Out 'runs.txt') $line
    Write-Host $line
    return $bad.Count -eq 0
}

foreach ($slot in $Slots) {
    $folder, $rest = $slot -split '-', 2
    $app = $rest.Substring(0, 1)
    for ($attempt = $FirstTry; $attempt -le $FirstTry + 3; $attempt++) {
        if (Invoke-Run $app $folder "$slot-try$attempt") { break }
    }
}
Write-Host "done: $Out"
