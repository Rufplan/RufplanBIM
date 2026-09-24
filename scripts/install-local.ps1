# Builds the release installer and installs Rufplan Studio for the current user.
# Re-run after pulling or changing code to upgrade the installed app in place.
# Usage (from the repo root):  powershell -ExecutionPolicy Bypass -File scripts\install-local.ps1

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Get-Process -Name "rufplan-studio" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Closing running Rufplan Studio (pid $($_.Id))..."
    $_ | Stop-Process -Force
}

Push-Location (Join-Path $repo "app")
try {
    if (-not (Test-Path "node_modules")) { npm ci }
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }
} finally {
    Pop-Location
}

$installer = Get-ChildItem (Join-Path $repo "target\release\bundle\nsis\*-setup.exe") |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $installer) { throw "no installer found in target\release\bundle\nsis" }

Write-Host "Installing $($installer.Name)..."
# /S = silent. Tauri's NSIS installer defaults to a per-user install (no admin prompt).
$p = Start-Process -FilePath $installer.FullName -ArgumentList "/S" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "installer exited with code $($p.ExitCode)" }

$exe = Join-Path $env:LOCALAPPDATA "Rufplan Studio\rufplan-studio.exe"
if (Test-Path $exe) {
    Write-Host "Installed: $exe"
    Write-Host "Launch it from the Start menu (Rufplan Studio)."
} else {
    Write-Host "Installer finished; check the Start menu for Rufplan Studio."
}
