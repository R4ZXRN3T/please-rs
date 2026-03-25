# please-rs

A tiny Rust CLI that runs commands with `sudo`.

It supports two modes:

- `please <command> [args...]` -> runs `sudo <command> [args...]`
- `please` (no arguments) -> finds your last shell command (excluding `please` itself of course) and runs it with `sudo`

## Why this exists

Typing `sudo !!` or copy-pasting the previous command can be shell-specific and easy to forget.
`please` gives you one consistent command across supported shells.

## Build Requirements

- [Git](https://git-scm.com/) for cloning the repo
- [Rust toolchain](https://rust-lang.org/learn/get-started/) (for building from source)
- [Make](https://www.gnu.org/software/make/) for executing the build scripts.
- [jq](https://jqlang.org/) for determining the version during build
- A C/C++ compiler. ideally, [gcc](https://gcc.gnu.org/) for Linux/MacOs
  and [MSVC](https://visualstudio.microsoft.com/downloads/#title-build-tools-for-visual-studio-2026) for Windows (scroll
  down to 'Tools for Visual Studio', download and install 'Build tools for Visual Studio')
- [UPX](https://upx.github.io/) For compressing the binary (Only for Linux)

### Platform-Specific Requirements

#### Linux & macOS

- A `sudo` command available in `PATH` (standard on most systems)

#### Windows

> **⚠️ Windows 11 (Build 24H2 or later) is required**
>
> `sudo` is only available on Windows 11 with update `24H2` or later.
> To enable it:
>
> **System Settings → System → Advanced → Activate sudo → Choose an operating mode (inline recommended)**
>
> [read more about sudo for windows](https://learn.microsoft.com/windows/advanced-settings/sudo/)
>
> If you're on an earlier Windows version, you will have to use [gsudo](https://github.com/gerardog/gsudo), which works
> on everything newer than Windows 7 SP 1. Without any `sudo` implementation, `please` will not work.
> Also, make sure that `sudo` is available on `PATH`.

### Supported Shells

The following shells are supported for `please` (no args) mode, which re-runs your last command with `sudo`:

| Shell                 | Linux | macOS | Windows* |
|-----------------------|:-----:|:-----:|:--------:|
| `bash`                |   ✅   |   ✅   |   ⚠️**   |
| `sh`                  |   ✅   |   ✅   |    ❌     |
| `zsh`                 |   ✅   |   ✅   |    ❌     |
| `fish`                |   ✅   |   ✅   |    ❌     |
| `nu` (nushell)        |   ✅   |   ✅   |    ✅     |
| `PowerShell` / `pwsh` |   ✅   |   ✅   |    ✅     |
| `cmd`                 |   ❌   |   ❌   |    ✅     |

*Windows can run all shells except cmd through [WSL](https://learn.microsoft.com/windows/wsl/) (Windows Subsystem for
Linux)\
**bash on Windows requires [Git Bash](https://gitforwindows.org/#bash)

## Installation

### 1) Install on Arch Linux (AUR)

On Arch Linux, you can install `please-rs` with your preferred AUR helper:

```bash
yay -S please-rs
```

or

```bash
paru -S please-rs
```

Manual AUR install:

```bash
git clone https://aur.archlinux.org/please-rs.git
cd please-rs
makepkg -si
```

Release note: the AUR package version (`pkgver`) should match the GitHub release tag.

### 2) Build and install with Makefile

1. Clone the repository (requires Git)

```bash
git clone https://github.com/R4ZXRN3T/please-rs.git
```

2. Open a terminal inside the cloned directory
3. Run:

On Linux/macOS:

```bash
sudo make install
```

On Windows (PowerShell, elevated/Administrator):

```powershell
make install
```

What this does:

- builds a release binary into `final/...`
- installs the binary globally
	- Linux: `/usr/bin/please`
	- macOS: `/usr/local/bin/please`
	- Windows: `%ProgramFiles%\please\please.exe` and adds that folder to system `PATH`

### 3) Use prebuilt artifacts

If you do not want to build from source, download a prebuilt binary from GitHub Releases:

- https://github.com/R4ZXRN3T/please-rs/releases

Look for assets named like:

- `please-<version>-Linux-<arch>.zip`
- `please-<version>-macOS-<arch>.zip`
- `please-<version>-Windows-<arch>.zip`

Typical architectures are `x86_64` and `aarch64`.

Unzip these files and place the executables into any location of your choice that is available in path

Recommended locations:

| OS      | Path                     |
|---------|--------------------------|
| Linux   | /usr/bin/                |
| MacOS   | /usr/local/bin/          |
| Windows | C:\Program Files\please\ |

(On windows you will have to add `C:\Program Files\please` to either user or system `PATH`.)

### 4) Build with Cargo only

```bash
cargo build --release
```

Binary location:

- `target/release/please` (Linux/macOS)
- `target/release/please.exe` (Windows)

## Usage

Show help:

```bash
please --help
```

Show detailed runtime info:

```bash
please --info
```

Print only the version:

```bash
please --version
```

Print the detected shell:

```bash
please --print-shell
```

Run a command with `sudo`:

```bash
please apt update
please systemctl restart nginx
```

Re-run your last non-`please` command with `sudo`:

```bash
please
```

Built-in options are handled as standalone arguments (`please --help`, `please --info`, etc.).

## How no-argument mode works

When you run `please` without arguments:

1. it detects your shell
2. reads history from shell-specific sources
3. skips entries that are `please` itself
4. wraps the selected command for your shell and executes it via `sudo`

History source highlights:

- `bash`/`sh`: `$HISTFILE` or `~/.bash_history` / `~/.sh_history`
- `zsh`: `$HISTFILE`, then `$ZDOTDIR/.zsh_history`, then `~/.zsh_history`
- `fish`: `$XDG_DATA_HOME/fish/fish_history` or `~/.local/share/fish/fish_history`
- `PowerShell`: common PSReadLine locations on Windows, Linux, and macOS
- `nu`: `nu` CLI history query, then `~/.config/nushell/history.txt`

Shell wrapping examples:

- PowerShell: `sudo powershell -NoProfile -Command "..."`
- cmd: `sudo cmd /C "..."`
- bash: `sudo bash -c "..."`

## Configuration

You can force shell detection by setting `PLEASE_SHELL`.

Examples:

```bash
export PLEASE_SHELL=bash
```

```powershell
$env:PLEASE_SHELL = "pwsh"
```

Accepted values include names like `cmd`, `powershell`, `pwsh`, `bash`, `sh`, `zsh`, `fish`, `nu`.

To inspect what `please` currently resolves, run:

```bash
please --info
```

## Uninstall

(For using the uninstalling per make, you need to be in the same directory as the [Makefile](Makefile))

Using Makefile:

```bash
make remove
```

Platform behavior:

- Linux: removes `/usr/bin/please`
- macOS: removes `/usr/local/bin/please`
- Windows: runs `scripts/uninstall-windows.ps1` (requires elevated PowerShell)

## Development

### Build

```bash
make build
```

Per-platform scripts used by `make`:

- `scripts/build-linux.sh`
- `scripts/build-macos.sh`
- `scripts/build-windows.ps1`

Notes:

- build scripts use nightly Rust and `-Z build-std`
- Linux build script expects `upx`
- scripts use `jq` to read package version from Cargo metadata

### Clean

```bash
make clean
```

## Troubleshooting

- `failed to start command 'sudo'`:
	- `sudo` is not available in your `PATH`. Install/provide a compatible `sudo` command for your platform.
- `unable to detect active shell`:
	- set `PLEASE_SHELL`, or run `please <command>` explicitly.
- `unable to discover a previous non-'please' command`:
	- run another command first so it appears in history, then try `please` again.
- Windows install/remove scripts fail:
	- run from an elevated (Administrator) PowerShell session.

## Security note

`please` executes your selected command with elevated privileges through `sudo`.
Always verify what command will run before confirming elevation prompts.

## License

This project is licensed under the terms in [LICENSE](LICENSE).
