# Numinous installer for Windows. One line to play, in PowerShell:
#
#   irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
#
# What it does, in order: checks the tools this machine needs (and says exactly
# how to get any that are missing), installs Rust through rustup if cargo is
# absent, fetches the source into ~\.numinous\src (git when available, a
# snapshot download otherwise), builds the release binaries, puts numinous,
# numinous-app, and numinous-mcp in ~\.numinous\bin, links the built-in radio
# next to them, and adds that directory to the user PATH.
#
# Re-run it any time to update. Remove everything it installed with:
#
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1))) -Uninstall
#
# or, from a checkout: powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -Uninstall
#
# Uninstalling never touches play history: ~\.numinous-journey,
# ~\.numinous-scores, and ~\.numinous-cairn stay yours.
#
# Options: -Uninstall, -NoModifyPath, -SelfTest.
# Set NUMINOUS_HOME to install somewhere other than ~\.numinous.
[CmdletBinding()]
param(
    [switch]$Uninstall,
    [switch]$NoModifyPath,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = `
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = 'blisspixel/numinous'
$RepoUrl = "https://github.com/$Repo"
$SnapshotUrl = "https://codeload.github.com/$Repo/tar.gz/refs/heads/main"
$NuminousHome = if ($env:NUMINOUS_HOME) { $env:NUMINOUS_HOME } else { Join-Path $HOME '.numinous' }
$SrcDir = Join-Path $NuminousHome 'src'
$BinDir = Join-Path $NuminousHome 'bin'
$Binaries = @('numinous.exe', 'numinous-app.exe', 'numinous-mcp.exe')

function Say([string]$Message) { Write-Host $Message }
function Fail([string]$Message) { throw $Message }
function Have([string]$Name) {
    return [bool](Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue)
}

# Run a native command and stop with a clear message if it fails.
function Invoke-Checked([string]$What, [scriptblock]$Action) {
    & $Action
    if ($LASTEXITCODE -ne 0) { Fail "$What failed; the output above says why." }
}

# Remove a directory that may be a junction: unlink a junction without ever
# recursing into its target, and only delete real directories recursively.
function Remove-DirectoryOrJunction([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        [IO.Directory]::Delete($Path, $false)
    } else {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

# Read the user Path exactly as stored (no expansion), so editing it never
# hardcodes the expanded value of someone else's %VAR% entries.
function Get-UserPathRaw([Microsoft.Win32.RegistryKey]$Key) {
    return [string]$Key.GetValue(
        'Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
}

# Put one directory first while preserving every unrelated raw entry exactly as
# written. Expanding only for comparison avoids baking environment-variable
# values into the stored user Path.
function Promote-PathEntry([string]$Current, [string]$Dir) {
    $target = $Dir.TrimEnd('\')
    $kept = @()
    foreach ($part in ($Current -split ';')) {
        if ($part -eq '') { continue }
        $expanded = [Environment]::ExpandEnvironmentVariables($part).TrimEnd('\')
        if ($expanded -ine $target) { $kept += $part }
    }
    return (@($Dir) + $kept) -join ';'
}

# PowerShell can return every matching executable even without -All. Select the
# first PATH match explicitly so a valid promoted install is not mistaken for
# the stale fallback that follows it.
function Select-FirstCommandSource([object[]]$Commands) {
    if ($null -eq $Commands -or $Commands.Count -eq 0) {
        Fail 'PATH verification could not resolve the numinous command.'
    }
    return [string]$Commands[0].Source
}

function Add-UserPath([string]$Dir) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $current = Get-UserPathRaw $key
        $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        if ($key.GetValueNames() -contains 'Path') { $kind = $key.GetValueKind('Path') }
        $promoted = Promote-PathEntry $current $Dir
        if ($promoted -ceq $current) { return $false }
        $key.SetValue('Path', $promoted, $kind)
        return $true
    } finally {
        $key.Close()
    }
}

function Test-PathPromotion {
    $target = 'C:\Users\Player\.numinous\bin'
    $stale = 'C:\Users\Player\.cargo\bin'
    $other = '%LOCALAPPDATA%\Programs\Tools'
    $actual = Promote-PathEntry "$stale;$target\;$other;$TARGET" $target
    $parts = @($actual -split ';')
    if ($parts[0] -cne $target) { Fail 'PATH self-test: install directory was not promoted.' }
    if (@($parts | Where-Object { $_.TrimEnd('\') -ieq $target }).Count -ne 1) {
        Fail 'PATH self-test: duplicate install entries remain.'
    }
    if ($parts[1] -cne $stale -or $parts[2] -cne $other) {
        Fail 'PATH self-test: unrelated entries changed order or spelling.'
    }
    $commands = @(
        [pscustomobject]@{ Source = "$target\numinous.exe" },
        [pscustomobject]@{ Source = "$stale\numinous.exe" }
    )
    $resolved = Select-FirstCommandSource $commands
    if ($resolved -cne "$target\numinous.exe") {
        Fail 'PATH self-test: resolver did not select the first executable.'
    }
    Say 'Windows installer PATH promotion: pass.'
}

function Remove-UserPath([string]$Dir) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    try {
        $current = Get-UserPathRaw $key
        if (-not $current) { return }
        $kept = @()
        foreach ($part in ($current -split ';')) {
            $expanded = [Environment]::ExpandEnvironmentVariables($part).TrimEnd('\')
            if ($part -ne '' -and $expanded -ine $Dir.TrimEnd('\')) { $kept += $part }
        }
        $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        if ($key.GetValueNames() -contains 'Path') { $kind = $key.GetValueKind('Path') }
        $key.SetValue('Path', ($kept -join ';'), $kind)
    } finally {
        $key.Close()
    }
}

# Tell running shells the environment changed, so the next terminal a user
# opens from Explorer or the Start menu sees the new PATH without a sign-out.
function Send-EnvironmentChange {
    try {
        Add-Type -Namespace NuminousInstall -Name NativeMethods -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
public static extern System.IntPtr SendMessageTimeout(
    System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
'@
        $result = [UIntPtr]::Zero
        [void][NuminousInstall.NativeMethods]::SendMessageTimeout(
            [IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
    } catch {
        # Best effort only; a fresh sign-in always picks the change up.
    }
}

function Uninstall-Numinous {
    Remove-DirectoryOrJunction (Join-Path $BinDir 'radio')
    if (Test-Path -LiteralPath $NuminousHome) {
        Remove-Item -LiteralPath $NuminousHome -Recurse -Force
    }
    Remove-UserPath $BinDir
    Send-EnvironmentChange
    Say "Numinous is uninstalled: $NuminousHome is gone and the PATH entry is removed."
    Say 'Your play history stays: ~\.numinous-journey, ~\.numinous-scores, ~\.numinous-cairn.'
}

function Install-Rust {
    $cargoBin = Join-Path $HOME '.cargo\bin'
    if (Test-Path (Join-Path $cargoBin 'cargo.exe')) { $env:Path = "$cargoBin;$env:Path" }
    if (Have 'cargo') { return }
    Say 'Rust is not installed yet. Installing it with rustup (https://rustup.rs).'
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64' }
        'ARM64' { 'aarch64' }
        default { Fail "unsupported processor architecture '$($env:PROCESSOR_ARCHITECTURE)'." }
    }
    $rustupInit = Join-Path $env:TEMP 'numinous-rustup-init.exe'
    Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/$arch" -OutFile $rustupInit
    $rustupArgs = @('-y', '--default-toolchain', 'none')
    if ($NoModifyPath) { $rustupArgs += '--no-modify-path' }
    Invoke-Checked 'rustup' { & $rustupInit @rustupArgs }
    Remove-Item $rustupInit -Force -ErrorAction SilentlyContinue
    $env:Path = "$cargoBin;$env:Path"
    if (-not (Have 'cargo')) {
        Fail 'rustup finished but cargo is still missing; open a new terminal and re-run.'
    }
}

function Test-BuildTools {
    if ($env:NUMINOUS_SKIP_MSVC_CHECK) { return }
    $vsRoot = ${env:ProgramFiles(x86)}
    if ($vsRoot) {
        $vswhere = Join-Path $vsRoot 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path $vswhere) {
            $found = & $vswhere -products * -latest -property installationPath `
                -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64
            if (-not [string]::IsNullOrWhiteSpace(($found | Out-String))) { return }
        }
    }
    Fail ('Rust on Windows links with the Microsoft C++ Build Tools, which are not installed. ' +
        'Get them from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and check ' +
        '"Desktop development with C++" during setup, then re-run this installer. ' +
        '(If you know your linker is fine, set NUMINOUS_SKIP_MSVC_CHECK=1 to skip this check.)')
}

function Get-Source {
    New-Item -ItemType Directory -Force -Path $NuminousHome | Out-Null
    if ((Test-Path (Join-Path $SrcDir '.git')) -and (Have 'git')) {
        Say "Updating the source in $SrcDir"
        Invoke-Checked 'git fetch' { git -C $SrcDir fetch --depth 1 origin main }
        Invoke-Checked 'git reset' { git -C $SrcDir reset --hard --quiet origin/main }
        return
    }
    $stage = Join-Path $NuminousHome ".staging-$PID"
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    try {
        if (Have 'git') {
            Say "Cloning $RepoUrl into $SrcDir"
            $newTree = Join-Path $stage 'src'
            Invoke-Checked 'git clone' { git clone --depth 1 $RepoUrl $newTree }
        } elseif (Have 'tar') {
            Say 'git is not installed; downloading a source snapshot instead.'
            $archive = Join-Path $stage 'numinous.tar.gz'
            Invoke-WebRequest -UseBasicParsing -Uri $SnapshotUrl -OutFile $archive
            Invoke-Checked 'extracting the snapshot' { tar -xzf $archive -C $stage }
            $newTree = Join-Path $stage 'numinous-main'
            if (-not (Test-Path $newTree)) { Fail 'unexpected source snapshot layout.' }
        } else {
            Fail 'neither git nor tar.exe is available. Install git (winget install Git.Git) and re-run.'
        }
        # Keep the previous build cache so updates do not rebuild from scratch,
        # and unlink the radio junction before its target moves.
        Remove-DirectoryOrJunction (Join-Path $BinDir 'radio')
        $oldTarget = Join-Path $SrcDir 'target'
        if (Test-Path $oldTarget) { Move-Item $oldTarget (Join-Path $newTree 'target') }
        if (Test-Path $SrcDir) { Remove-Item $SrcDir -Recurse -Force }
        Move-Item $newTree $SrcDir
    } finally {
        if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    }
}

function Build-Numinous {
    if (Have 'rustup') {
        # Install the pinned toolchain up front so the build step is only a
        # build. Older rustup releases need the toolchain named; current ones
        # install it on demand anyway, so a failure here is not fatal.
        Push-Location $SrcDir
        try { rustup toolchain install } catch {} finally { Pop-Location }
    } else {
        # A standalone cargo cannot honor the pinned toolchain file, so accept
        # it only if it meets the workspace MSRV in Cargo.toml.
        $version = (cargo --version) -replace '^cargo (\d+\.\d+).*', '$1'
        $parsed = [version]'0.0'
        if (-not [version]::TryParse($version, [ref]$parsed) -or $parsed -lt [version]'1.85') {
            Fail ('this cargo is older than the minimum supported Rust (1.85) and rustup is ' +
                'absent. Install rustup from https://rustup.rs and re-run this installer.')
        }
        Say 'note: using cargo without rustup; the pinned toolchain file is ignored.'
    }
    Say 'Building the release binaries (the first build takes several minutes).'
    Push-Location $SrcDir
    try {
        Invoke-Checked 'the build' {
            cargo build --release --locked --bin numinous --bin numinous-app --bin numinous-mcp
        }
    } finally {
        Pop-Location
    }
}

function Install-Binaries {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    foreach ($binary in $Binaries) {
        $from = Join-Path $SrcDir "target\release\$binary"
        try {
            Copy-Item $from (Join-Path $BinDir $binary) -Force
        } catch {
            Fail "could not replace $binary; close any running Numinous windows and re-run."
        }
    }
    # The app finds the built-in radio next to its executable. A junction
    # avoids duplicating the tracks; fall back to a copy if it is refused.
    $radioLink = Join-Path $BinDir 'radio'
    $radioSource = Join-Path $SrcDir 'assets\radio'
    Remove-DirectoryOrJunction $radioLink
    try {
        New-Item -ItemType Junction -Path $radioLink -Target $radioSource | Out-Null
    } catch {
        Copy-Item $radioSource $radioLink -Recurse
    }
}

function Install-Numinous {
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Fail 'this installer needs Windows PowerShell 5.1 or newer.'
    }
    Test-BuildTools
    Install-Rust
    Get-Source
    Build-Numinous
    Install-Binaries
    $pathChanged = $false
    if (-not $NoModifyPath) {
        $pathChanged = Add-UserPath $BinDir
        if ($pathChanged) { Send-EnvironmentChange }
        $env:Path = Promote-PathEntry $env:Path $BinDir
    }
    Say ''
    Say 'Numinous is installed.'
    Say ''
    Say '  numinous-app     the window: rooms, sound, games, the radio'
    Say '  numinous         the same world, live in the terminal'
    Say ''
    Say 'Digital minds connect over MCP:'
    Say "  claude mcp add numinous -- $BinDir\numinous-mcp.exe"
    Say ''
    if ($NoModifyPath) {
        Say "PATH was not modified. Run the binaries by full path from $BinDir,"
        Say 'or add that directory to PATH yourself.'
    } elseif ($pathChanged) {
        Say "Open a new terminal so PATH picks up $BinDir, then type: numinous-app"
    } else {
        Say 'Type numinous-app to begin.'
    }
    Say ''
    Say 'Installed commands:'
    foreach ($binary in $Binaries) {
        Say "  $(Join-Path $BinDir $binary)"
    }
    $installedCli = Join-Path $BinDir 'numinous.exe'
    $resolvedCli = if ($NoModifyPath) {
        $installedCli
    } else {
        Select-FirstCommandSource @(
            Get-Command numinous -CommandType Application -ErrorAction Stop
        )
    }
    if (-not $NoModifyPath -and $resolvedCli -ine $installedCli) {
        Fail "PATH still resolves numinous to $resolvedCli instead of the new install."
    }
    Invoke-Checked 'installed CLI version check' { & $resolvedCli --version }
    Say ''
    Say "Read PLAY.md first if you read anything: $SrcDir\PLAY.md"
    Say 'Update any time by re-running this installer. Uninstall with -Uninstall.'
}

try {
    if ($SelfTest) {
        Test-PathPromotion
    } elseif ($Uninstall) {
        Uninstall-Numinous
    } else {
        Install-Numinous
    }
} catch {
    Write-Host "numinous install: $($_.Exception.Message)" -ForegroundColor Red
    if ($PSCommandPath) { exit 1 }
}
