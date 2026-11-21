# Same-session A/B of page-turn latency: A = GPUI (crates/native, main),
# B = Tauri (this branch). Per folder: one throwaway warm run of each, then
# A B A B A B. Each run launches the app on the folder, lets latprobe press
# Right $Presses times at 1003 ms and measure keypress to DWM present, then
# closes it. A run with a load sample over 15 % after the first, or a
# compiler process, is discarded and redone in its slot.
param(
    [Parameter(Mandatory)] [string]$Gpui,
    [Parameter(Mandatory)] [string]$Tauri,
    [Parameter(Mandatory)] [string]$Probe,
    [Parameter(Mandatory)] [string]$Out,
    [string]$Root = 'D:\Photos\tests\20261108-gpui-compare',
    [string[]]$Folders = @('ARW', 'RAF'),
    [int]$Runs = 3,
    [int]$Presses = 60,
    [int]$SettleMs = 8000
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
$tauriLogDir = Join-Path $env:LOCALAPPDATA 'dev.minodisk.handpic.latencyab\logs'
$busyNames = @('cargo', 'rustc', 'link', 'msbuild', 'cl')
# A 1 px mouse move and back before each run keeps the window from losing the
# foreground (as in the fit-preview-downscale run).
Add-Type -Namespace W -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, System.UIntPtr e);'
function Wake { [W.M]::mouse_event(1, 1, 0, 0, [UIntPtr]::Zero); [W.M]::mouse_event(1, -1, 0, 0, [UIntPtr]::Zero) }
# A fresh Tauri profile asks for the sidecar format before any folder opens.
$tauriSettings = Join-Path $env:APPDATA 'dev.minodisk.handpic.latencyab\settings.json'
if (-not (Test-Path $tauriSettings)) {
    New-Item -ItemType Directory -Force (Split-Path $tauriSettings) | Out-Null
    [IO.File]::WriteAllText($tauriSettings, '{"sidecarFormat":"both"}')
}

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
    if ($app -eq 'A') {
        $argList = @("`"$dir`"", '--no-prefetch',
            '--index', "`"$(Join-Path $Out 'gpui-index.sqlite')`"",
            '--settings', "`"$(Join-Path $Out 'gpui-settings.sqlite')`"",
            '--log', "`"$(Join-Path $Out "$name.app.log")`"")
        $proc = Start-Process -FilePath $Gpui -ArgumentList $argList -PassThru `
            -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    } else {
        if (Test-Path $tauriLogDir) { Get-ChildItem $tauriLogDir -Filter *.log | Remove-Item }
        $env:HANDPIC_MEASURE_FOLDER = $dir
        $proc = Start-Process -FilePath $Tauri -PassThru `
            -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        Remove-Item Env:HANDPIC_MEASURE_FOLDER
    }
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
    if ($app -eq 'B' -and (Test-Path $tauriLogDir)) {
        Get-ChildItem $tauriLogDir -Filter *.log | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $Out "$name.app.log")
        }
    }
    if ($probe.ExitCode -ne 0) { $bad += "probe exit $($probe.ExitCode)" }
    $summary = (Get-Content $probeOut -ErrorAction SilentlyContinue | Select-String '^summary') -join ''
    $line = "$name before=$before samples=[$($samples -join '; ')] bad=[$($bad -join '; ')] $summary"
    Add-Content (Join-Path $Out 'runs.txt') $line
    Write-Host $line
    return $bad.Count -eq 0
}

foreach ($folder in $Folders) {
    Invoke-Run 'A' $folder "$folder-warm-A" | Out-Null
    Invoke-Run 'B' $folder "$folder-warm-B" | Out-Null
    for ($r = 1; $r -le $Runs; $r++) {
        foreach ($app in @('A', 'B')) {
            for ($attempt = 1; $attempt -le 4; $attempt++) {
                $name = "$folder-$app$r-try$attempt"
                if (Invoke-Run $app $folder $name) { break }
            }
        }
    }
}
Write-Host "done: $Out"
