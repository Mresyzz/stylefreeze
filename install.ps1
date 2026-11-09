# StyleFreeze Windows installer
# Usage: irm https://raw.githubusercontent.com/Mresyzz/stylefreeze/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "Mresyzz/stylefreeze"
$Asset = "stylefreeze-x86_64-pc-windows-msvc.zip"
$Url = "https://github.com/$Repo/releases/latest/download/$Asset"

$TargetDir = Join-Path $HOME ".cargo\bin"
if (-not (Test-Path $TargetDir)) {
    $TargetDir = Join-Path $env:LOCALAPPDATA "Programs\stylefreeze"
    New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
}

$TempZip = Join-Path $env:TEMP "stylefreeze.zip"
$TempExtract = Join-Path $env:TEMP "stylefreeze_extracted"

Write-Host "[INFO] Downloading StyleFreeze from $Url..."
Invoke-WebRequest -Uri $Url -OutFile $TempZip

if (Test-Path $TempExtract) {
    Remove-Item -Recurse -Force $TempExtract
}
Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

$SourceBin = Join-Path $TempExtract "stylefreeze.exe"
$DestBin = Join-Path $TargetDir "stylefreeze.exe"

Copy-Item -Path $SourceBin -Destination $DestBin -Force
Remove-Item -Force $TempZip
Remove-Item -Recurse -Force $TempExtract

Write-Host "[OK] StyleFreeze installed successfully to $DestBin"

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$TargetDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$TargetDir", "User")
    Write-Host "[INFO] Added $TargetDir to User PATH. Please restart your terminal."
}
