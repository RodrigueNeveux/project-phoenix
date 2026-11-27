param(
  [string]$Exe = (Join-Path $PSScriptRoot "discord-phoenix.exe"),
  [switch]$Unregister
)
$ErrorActionPreference = "Stop"
$key = "HKCU:\Software\Classes\discord"
if ($Unregister) {
  if (Test-Path $key) { Remove-Item -Recurse -Force $key }
  Write-Host "discord:// links are no longer handled by Discord Phoenix."
  exit 0
}
$Exe = (Resolve-Path $Exe).Path
New-Item -Force $key | Out-Null
Set-ItemProperty $key "(default)" "URL:Discord Protocol"
Set-ItemProperty $key "URL Protocol" ""
New-Item -Force "$key\DefaultIcon" | Out-Null
Set-ItemProperty "$key\DefaultIcon" "(default)" "`"$Exe`",0"
New-Item -Force "$key\shell\open\command" | Out-Null
Set-ItemProperty "$key\shell\open\command" "(default)" "`"$Exe`" --url -- `"%1`""
Write-Host "discord:// links now open $Exe"
