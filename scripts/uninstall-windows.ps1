#Requires -Version 7

$ErrorActionPreference = 'Stop'

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
{
	throw 'Please run `make remove` from an elevated PowerShell (Administrator).'
}

$installDir = Join-Path $env:ProgramFiles 'please'
$exePath = Join-Path $installDir 'please.exe'

if (Test-Path -Path $exePath -PathType Leaf)
{
	Remove-Item -Path $exePath -Force
	Write-Host ('Removed ' + $exePath)
}
else
{
	Write-Host ('Binary not found, skipping: ' + $exePath)
}

if (Test-Path -Path $installDir -PathType Container)
{
	$remainingEntries = Get-ChildItem -Path $installDir -Force
	if (-not $remainingEntries)
	{
		Remove-Item -Path $installDir -Force
		Write-Host ('Removed empty directory ' + $installDir)
	}
	else
	{
		Write-Host ('Keeping directory with remaining files: ' + $installDir)
	}
}

function Normalize-PathEntry
{
	param(
		[string]$PathEntry
	)

	if ( [string]::IsNullOrWhiteSpace($PathEntry))
	{
		return $null
	}

	try
	{
		return [System.IO.Path]::GetFullPath($PathEntry.Trim()).TrimEnd('\\')
	}
	catch
	{
		return $PathEntry.Trim().TrimEnd('\\')
	}
}

$targetPath = Normalize-PathEntry -PathEntry $installDir
$machinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
$pathEntries = @()

if (-not [string]::IsNullOrWhiteSpace($machinePath))
{
	$pathEntries = @($machinePath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

$keptEntries = New-Object System.Collections.Generic.List[string]
$removedCount = 0

foreach ($entry in $pathEntries)
{
	$normalizedEntry = Normalize-PathEntry -PathEntry $entry
	if ($normalizedEntry -and ($normalizedEntry -ieq $targetPath))
	{
		$removedCount++
		continue
	}

	[void]$keptEntries.Add($entry.Trim())
}

if ($removedCount -gt 0)
{
	$updatedPath = $keptEntries -join ';'
	[Environment]::SetEnvironmentVariable('Path', $updatedPath, 'Machine')
	Write-Host ('Removed from system PATH: ' + $installDir)
}
else
{
	Write-Host ('Not present in system PATH: ' + $installDir)
}

Write-Host 'Uninstall completed.'

