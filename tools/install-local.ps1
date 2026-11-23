# Builds craft.exe and puts it where the plugin looks for it (bin/craft.exe).
# Use it after installing the plugin from a clone or from source: the repository does not
# track plugin/bin, only release archives carry the binary. Run again after a plugin update.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
} finally { Pop-Location }

$exe = Join-Path $repo 'target\release\craft.exe'
$targets = @(Join-Path $repo 'plugin\bin')
$cache = Join-Path $env:USERPROFILE '.claude\plugins\cache\craft-dev-toolkit\craft-dev-toolkit'
if (Test-Path $cache) {
    $targets += Get-ChildItem $cache -Directory | ForEach-Object { Join-Path $_.FullName 'bin' }
}
foreach ($dir in $targets) {
    New-Item -ItemType Directory -Force $dir | Out-Null
    Copy-Item $exe $dir -Force
    Write-Host "craft.exe -> $dir"
}
# dev-to-main smoke test
