# install.ps1 - installs the latest release of Handpic for the current user
# with its per-user setup (%LOCALAPPDATA%\Programs\Handpic, no administrator
# prompt), where its updater expects it. The setup is checked against the
# release's SHA256SUMS before it runs. Written to be piped to iex in Windows
# PowerShell 5.1 and PowerShell 7: the body runs in a script block so its
# preferences stay out of the session, and a failure throws instead of
# exiting, which would close the shell.
#
# Usage: irm https://raw.githubusercontent.com/minodisk/handpic/main/install.ps1 | iex

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    if (-not [Environment]::Is64BitOperatingSystem) {
        throw 'Handpic needs 64-bit Windows (x64, or Arm on Windows 11); this is 32-bit Windows.'
    }

    $repo = 'minodisk/handpic'
    $release = Invoke-RestMethod -UseBasicParsing -Uri "https://api.github.com/repos/$repo/releases/latest"
    $tag = $release.tag_name
    if ($tag -notmatch '^v[0-9]') {
        throw "Could not resolve the latest release (got '$tag')."
    }
    $version = $tag.Substring(1)
    $setup = "Handpic-$version-windows-x64-setup.exe"
    $base = "https://github.com/$repo/releases/download/$tag"

    $dir = Join-Path ([IO.Path]::GetTempPath()) ('handpic-install-' + [Guid]::NewGuid())
    New-Item -ItemType Directory -Path $dir | Out-Null
    try {
        $sums = Join-Path $dir 'SHA256SUMS'
        $path = Join-Path $dir $setup
        Write-Host 'Downloading SHA256SUMS'
        Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile $sums
        Write-Host "Downloading $setup"
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$setup" -OutFile $path

        $expected = $null
        foreach ($line in Get-Content -Path $sums) {
            $fields = $line -split '\s+', 2
            if ($fields.Count -eq 2 -and $fields[1] -eq $setup) {
                $expected = $fields[0]
            }
        }
        if (-not $expected) {
            throw "$setup is not listed in SHA256SUMS."
        }
        $actual = (Get-FileHash -Algorithm SHA256 -Path $path).Hash
        if ($actual -ne $expected) {
            throw "Checksum mismatch for ${setup}; nothing installed."
        }

        Write-Host "Installing Handpic $version"
        $process = Start-Process -FilePath $path -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            throw "The setup exited with code $($process.ExitCode)."
        }
        Write-Host "Installed Handpic ${version}: $env:LOCALAPPDATA\Programs\Handpic\Handpic.exe"
    }
    finally {
        Remove-Item -Recurse -Force -Path $dir -ErrorAction SilentlyContinue
    }
}
