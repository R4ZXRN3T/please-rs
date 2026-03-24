#Requires -Version 7

rustup override set nightly

$PKGNAME = "please"
$PKGVERSION = (cargo metadata --format-version 1 | jq -r ".packages[] | select(.name==`"$PKGNAME`-rs") | .version")

# Use CARGO_BUILD_TARGET if set (for cross-compilation), otherwise detect host architecture
if ($env:CARGO_BUILD_TARGET)
{
	$ARCH = $env:CARGO_BUILD_TARGET.Split('-')[0]
}
else
{
	$ARCH = switch ((cmd /c echo %PROCESSOR_ARCHITECTURE%))
	{
		"AMD64" {
			"x86_64"
		}
		"ARM64" {
			"aarch64"
		}
		"X86" {
			"i686"
		}
		default {
			"unknown"
		}
	}
}

Remove-Item -Path ".\final\$PKGNAME-$PKGVERSION-Windows-$ARCH" -Recurse -Force -ErrorAction SilentlyContinue

rustup component add rust-src --toolchain nightly

$env:RUSTFLAGS = "-Zlocation-detail=none -Zfmt-debug=none"
$buildCmd = "cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=`"optimize_for_size`" --release"
if ($env:CARGO_BUILD_TARGET)
{
	$buildCmd += " --target $( $env:CARGO_BUILD_TARGET )"
}
Invoke-Expression $buildCmd

New-Item -ItemType Directory -Path ".\final\$PKGNAME-$PKGVERSION-Windows-$ARCH" -Force | Out-Null

# Binary location depends on whether we cross-compiled
if ($env:CARGO_BUILD_TARGET)
{
	$binaryPath = ".\target\$( $env:CARGO_BUILD_TARGET )\release\$PKGNAME.exe"
}
else
{
	$binaryPath = ".\target\release\$PKGNAME.exe"
}

Move-Item -Path $binaryPath -Destination ".\final\$PKGNAME-$PKGVERSION-Windows-$ARCH\$PKGNAME.exe"
Set-Location ".\final\$PKGNAME-$PKGVERSION-Windows-$ARCH"

Set-Location "..\.."