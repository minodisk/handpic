# In-app `page` lines with prefetch on: each build pages each folder with
# `--auto $Presses` (keyed pages, one every 1003 ms), reusing the build's index
# and settings file of the ruler runs in $Out. Idle-gated like run-ab-gpui.ps1;
# no external ruler.
param(
    [Parameter(Mandatory)] [string]$Gpui,
    [Parameter(Mandatory)] [string]$GpuiB,
    [Parameter(Mandatory)] [string]$Out,
    [string]$Root = 'D:\Photos\tests\20261108-gpui-compare',
    [string[]]$Folders = @('ARW', 'RAF'),
    [int]$Presses = 59
)
$ErrorActionPreference = 'Stop'
$busyNames = @('cargo', 'rustc', 'link', 'msbuild', 'cl')
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

foreach ($folder in $Folders) {
    foreach ($app in @('A', 'B')) {
        $name = "$folder-prefetch-$app"
        $before = Wait-Idle
        $exe = if ($app -eq 'A') { $Gpui } else { $GpuiB }
        $argList = @("`"$(Join-Path $Root $folder)`"", '--auto', $Presses,
            '--index', "`"$(Join-Path $Out "$app-index.sqlite")`"",
            '--settings', "`"$(Join-Path $Out "$app-settings.sqlite")`"",
            '--log', "`"$(Join-Path $Out "$name.app.log")`"")
        $proc = Start-Process -FilePath $exe -ArgumentList $argList -PassThru `
            -RedirectStandardOutput (Join-Path $Out "$name.stdout.txt") `
            -RedirectStandardError (Join-Path $Out "$name.stderr.txt")
        $null = $proc.Handle
        $proc.WaitForExit()
        $line = "$name before=$before exit=$($proc.ExitCode)"
        Add-Content (Join-Path $Out 'runs-prefetch.txt') $line
        Write-Host $line
    }
}
