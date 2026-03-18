#Requires -Version 7

$ErrorActionPreference = 'Stop'

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
{
	throw 'Please run `make install` from an elevated PowerShell (Administrator).'
}

$installDir = Join-Path $env:ProgramFiles 'please'
[void](New-Item -ItemType Directory -Path $installDir -Force)

$sourceItems = @(Get-ChildItem -Path '.\final' -Filter 'please.exe' -Recurse)
$source = if ($sourceItems.Count -gt 0)
{
	$sourceItems[0].FullName
}
else
{
	$null
}

if (-not $source)
{
	throw 'please.exe not found under .\final. Build failed or artifact layout changed.'
}

Copy-Item -Path $source -Destination (Join-Path $installDir 'please.exe') -Force

$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
if (-not (($machinePath -split ';') -contains $installDir))
{
	$updatedPath = ($machinePath.TrimEnd(';') + ';' + $installDir)
	[Environment]::SetEnvironmentVariable('Path', $updatedPath, 'Machine')
	Write-Host ('Added to system PATH: ' + $installDir)
}
else
{
	Write-Host ('Already in system PATH: ' + $installDir)
}

Write-Host ('Installed please.exe to ' + $installDir)
