# Numinous installer for Windows. One line to play, in PowerShell:
#
#   irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
#
# What it does, in order: checks the tools this machine needs (and says exactly
# how to get any that are missing), installs Rust through rustup if cargo is
# absent, replaces ~\.numinous\src from a fixed source snapshot, builds the
# release binaries, puts numinous,
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
$RequestedNuminousHome = if ($SelfTest) {
    Join-Path $HOME '.numinous'
} elseif ($env:NUMINOUS_HOME) {
    $env:NUMINOUS_HOME
} else {
    Join-Path $HOME '.numinous'
}
$Binaries = @('numinous.exe', 'numinous-app.exe', 'numinous-mcp.exe')
$InstallMarkerName = '.numinous-install-root'
$InstallMarkerText = 'Numinous install root'

function Say([string]$Message) { Write-Host $Message }
function Fail([string]$Message) { throw $Message }
function Have([string]$Name) {
    return [bool](Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue)
}

function Resolve-InstallRoot([string]$Path, [string]$HomePath) {
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path -match '[\x00-\x1f\x7f]') {
        Fail 'NUMINOUS_HOME must name a dedicated absolute directory.'
    }
    if (-not [IO.Path]::IsPathRooted($Path) -or $Path -match '^[A-Za-z]:[^\\/]') {
        Fail 'NUMINOUS_HOME must be an absolute path.'
    }
    if (@($Path -split '[\\/]' | Where-Object { $_ -eq '.' -or $_ -eq '..' }).Count -ne 0) {
        Fail 'NUMINOUS_HOME must not contain . or .. path components.'
    }

    $full = [IO.Path]::GetFullPath($Path)
    $volumeRoot = [IO.Path]::GetPathRoot($full)
    $trimChars = [char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    if ($full.Length -gt $volumeRoot.Length) { $full = $full.TrimEnd($trimChars) }
    $homeFull = [IO.Path]::GetFullPath($HomePath)
    $homeRoot = [IO.Path]::GetPathRoot($homeFull)
    if ($homeFull.Length -gt $homeRoot.Length) { $homeFull = $homeFull.TrimEnd($trimChars) }
    if ($full.Equals($volumeRoot, [StringComparison]::OrdinalIgnoreCase) -or
        $full.Equals($homeFull, [StringComparison]::OrdinalIgnoreCase)) {
        Fail 'NUMINOUS_HOME must name a dedicated directory, not HOME or a volume root.'
    }

    $parent = Split-Path -Parent $full
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        Fail 'the parent directory of NUMINOUS_HOME must already exist.'
    }
    $parentItem = Get-Item -LiteralPath $parent -Force
    if ($parentItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        Fail 'the parent directory of NUMINOUS_HOME must not be a reparse point.'
    }
    if (Test-Path -LiteralPath $full) {
        $item = Get-Item -LiteralPath $full -Force
        if (-not $item.PSIsContainer) {
            Fail 'NUMINOUS_HOME exists but is not a directory.'
        }
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            Fail 'NUMINOUS_HOME must not be a reparse point.'
        }
    }
    return $full
}

$NuminousHome = Resolve-InstallRoot $RequestedNuminousHome $HOME
$SrcDir = Join-Path $NuminousHome 'src'
$BinDir = Join-Path $NuminousHome 'bin'

# Run a native command and stop with a clear message if it fails.
function Invoke-Checked([string]$What, [scriptblock]$Action) {
    & $Action
    if ($LASTEXITCODE -ne 0) { Fail "$What failed; the output above says why." }
}

# Remove a tree without following any reparse point found inside it.
function Remove-DirectoryOrJunction([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        if ($item.PSIsContainer) {
            [IO.Directory]::Delete($Path, $false)
        } else {
            [IO.File]::Delete($Path)
        }
        return
    }
    if (-not $item.PSIsContainer) {
        Remove-Item -LiteralPath $Path -Force
        return
    }
    foreach ($child in Get-ChildItem -LiteralPath $Path -Force) {
        Remove-DirectoryOrJunction $child.FullName
    }
    Remove-Item -LiteralPath $Path -Force
}

function Test-DirectoryEmpty([string]$Path) {
    return @(Get-ChildItem -LiteralPath $Path -Force).Count -eq 0
}

function Test-InstallMarker([string]$Root) {
    $marker = Join-Path $Root $InstallMarkerName
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { return $false }
    $item = Get-Item -LiteralPath $marker -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { return $false }
    if ($item.Length -gt 64) { return $false }
    $content = [IO.File]::ReadAllText($marker)
    return $content -ceq $InstallMarkerText -or
        $content -ceq "$InstallMarkerText`n" -or
        $content -ceq "$InstallMarkerText`r`n"
}

function Write-InstallMarker([string]$Root) {
    $marker = Join-Path $Root $InstallMarkerName
    if (Test-Path -LiteralPath $marker) {
        $item = Get-Item -LiteralPath $marker -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            Fail 'the install-root marker must not be a reparse point.'
        }
    }
    $encoding = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($marker, "$InstallMarkerText`r`n", $encoding)
}

function Test-LegacyInstallRoot([string]$Root) {
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        return $false
    }
    $children = @(Get-ChildItem -LiteralPath $Root -Force)
    if ($children.Count -ne 2 -or
        @($children | Where-Object { $_.Name -notin @('src', 'bin') }).Count -ne 0 -or
        @($children | Where-Object {
            -not $_.PSIsContainer -or ($_.Attributes -band [IO.FileAttributes]::ReparsePoint)
        }).Count -ne 0) {
        return $false
    }
    $manifest = Get-Item -LiteralPath (Join-Path $Root 'src\Cargo.toml') -Force -ErrorAction SilentlyContinue
    if ($null -eq $manifest -or $manifest.PSIsContainer -or
        ($manifest.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        return $false
    }
    foreach ($binary in $Binaries) {
        $item = Get-Item -LiteralPath (Join-Path $Root "bin\$binary") -Force -ErrorAction SilentlyContinue
        if ($null -eq $item -or $item.PSIsContainer -or
            ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            return $false
        }
    }
    return $true
}

function Test-InstallRootClaimable([string]$Root) {
    return (Test-InstallMarker $Root) -or
        (Test-DirectoryEmpty $Root) -or
        (Test-LegacyInstallRoot $Root)
}

function Initialize-InstallRoot([string]$Root = $NuminousHome) {
    $Root = Resolve-InstallRoot $Root $HOME
    if (Test-Path -LiteralPath $Root) {
        if (-not (Test-InstallRootClaimable $Root)) {
            Fail 'NUMINOUS_HOME exists but is not a marked Numinous install root.'
        }
    } else {
        New-Item -ItemType Directory -Path $Root | Out-Null
    }
    $rechecked = Resolve-InstallRoot $Root $HOME
    if ($rechecked -ine $Root) {
        Fail 'NUMINOUS_HOME changed while the installer was starting.'
    }
    if (-not (Test-InstallRootClaimable $Root)) {
        Fail 'NUMINOUS_HOME contents changed while the installer was starting.'
    }
    Write-InstallMarker $Root
}

function Remove-ValidatedInstallRoot([string]$Root) {
    $resolved = Resolve-InstallRoot $Root $HOME
    if (-not (Test-Path -LiteralPath $resolved)) { return }
    $marked = Test-InstallMarker $resolved
    $legacy = Test-LegacyInstallRoot $resolved
    if (-not $marked -and -not $legacy) {
        Fail "refusing to remove an unmarked install root: $resolved"
    }
    $rechecked = Resolve-InstallRoot $resolved $HOME
    if ($rechecked -ine $resolved -or
        ($marked -and -not (Test-InstallMarker $resolved)) -or
        ($legacy -and -not (Test-LegacyInstallRoot $resolved))) {
        Fail 'the install root changed during uninstall.'
    }
    if ($marked) {
        Remove-DirectoryOrJunction $resolved
        return
    }
    Remove-DirectoryOrJunction (Join-Path $resolved 'src')
    Remove-DirectoryOrJunction (Join-Path $resolved 'bin')
    if (-not (Test-DirectoryEmpty $resolved)) {
        Fail 'the legacy install root gained unexpected contents during uninstall.'
    }
    Remove-Item -LiteralPath $resolved -Force
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

function Test-InstallerSafety {
    if (-not (Have 'tar')) { Fail 'installer safety self-test requires tar.exe.' }
    $testBase = Join-Path $env:TEMP ('numinous-installer-test-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testBase | Out-Null
    try {
        $rejectedHome = $false
        try { [void](Resolve-InstallRoot $HOME $HOME) } catch { $rejectedHome = $true }
        if (-not $rejectedHome) { Fail 'root self-test: HOME was accepted as an install root.' }

        $unmarked = Join-Path $testBase 'unmarked'
        New-Item -ItemType Directory -Path $unmarked | Out-Null
        Set-Content -LiteralPath (Join-Path $unmarked 'keep.txt') -Value 'keep'
        Set-Content -LiteralPath (Join-Path $unmarked $InstallMarkerName) -Value 'not a marker'
        $rejectedUnmarked = $false
        try { Remove-ValidatedInstallRoot $unmarked } catch { $rejectedUnmarked = $true }
        if (-not $rejectedUnmarked -or -not (Test-Path -LiteralPath $unmarked)) {
            Fail 'uninstall self-test: an unmarked root was removed.'
        }

        $legacy = Join-Path $testBase 'legacy-default'
        New-Item -ItemType Directory -Path (Join-Path $legacy 'src') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $legacy 'bin') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $legacy 'src\Cargo.toml') -Value '[workspace]'
        foreach ($binary in $Binaries) {
            Set-Content -LiteralPath (Join-Path $legacy "bin\$binary") -Value 'binary'
        }
        if (-not (Test-InstallRootClaimable $legacy)) {
            Fail 'root self-test: the exact legacy install shape could not migrate.'
        }
        Set-Content -LiteralPath (Join-Path $legacy 'unexpected.txt') -Value 'keep'
        if (Test-InstallRootClaimable $legacy) {
            Fail 'root self-test: a legacy root with unexpected contents was accepted.'
        }
        Remove-Item -LiteralPath (Join-Path $legacy 'unexpected.txt') -Force
        $rejectedArbitrary = $false
        try { Initialize-InstallRoot $unmarked } catch { $rejectedArbitrary = $true }
        if (-not $rejectedArbitrary) {
            Fail 'root self-test: arbitrary nonempty contents were accepted.'
        }
        Initialize-InstallRoot $legacy
        if (-not (Test-InstallMarker $legacy)) {
            Fail 'root self-test: the legacy install was not marked during migration.'
        }

        $legacyUninstall = Join-Path $testBase 'legacy-uninstall'
        New-Item -ItemType Directory -Path (Join-Path $legacyUninstall 'src') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $legacyUninstall 'bin') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $legacyUninstall 'src\Cargo.toml') -Value '[workspace]'
        foreach ($binary in $Binaries) {
            Set-Content -LiteralPath (Join-Path $legacyUninstall "bin\$binary") -Value 'binary'
        }
        Remove-ValidatedInstallRoot $legacyUninstall
        if (Test-Path -LiteralPath $legacyUninstall) {
            Fail 'uninstall self-test: the exact legacy install was retained.'
        }

        $marked = Join-Path $testBase 'marked'
        New-Item -ItemType Directory -Path $marked | Out-Null
        Write-InstallMarker $marked
        $adjacent = Join-Path $testBase 'adjacent.txt'
        Set-Content -LiteralPath $adjacent -Value 'keep'
        $outside = Join-Path $testBase 'outside'
        New-Item -ItemType Directory -Path (Join-Path $outside 'radio') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $outside 'radio\keep.txt') -Value 'keep'
        New-Item -ItemType Junction -Path (Join-Path $marked 'bin') -Target $outside | Out-Null
        Remove-ValidatedInstallRoot $marked
        if ((Test-Path -LiteralPath $marked) -or
            -not (Test-Path -LiteralPath $adjacent) -or
            -not (Test-Path -LiteralPath (Join-Path $outside 'radio\keep.txt'))) {
            Fail 'uninstall self-test: marked-root removal crossed its boundary.'
        }

        $sourceRoot = Join-Path $testBase 'source-root'
        New-Item -ItemType Directory -Path $sourceRoot | Out-Null
        Write-InstallMarker $sourceRoot
        $sourceDir = Join-Path $sourceRoot 'src'
        $binaryDir = Join-Path $sourceRoot 'bin'
        New-Item -ItemType Directory -Path (Join-Path $sourceDir '.git') -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $sourceDir 'target') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $sourceDir '.git\config') -Value 'alternate origin'
        Set-Content -LiteralPath (Join-Path $sourceDir 'untrusted.txt') -Value 'untrusted'
        Set-Content -LiteralPath (Join-Path $sourceDir 'target\cached.txt') -Value 'untrusted cache'
        $sourceOutside = Join-Path $testBase 'source-outside'
        New-Item -ItemType Directory -Path (Join-Path $sourceOutside 'radio') -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $sourceOutside 'radio\keep.txt') -Value 'keep'
        New-Item -ItemType Junction -Path $binaryDir -Target $sourceOutside | Out-Null

        $package = Join-Path $testBase 'package'
        $trustedTree = Join-Path $package 'numinous-main'
        New-Item -ItemType Directory -Path $trustedTree -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $trustedTree 'trusted.txt') -Value 'trusted'
        $archive = Join-Path $testBase 'trusted.tar.gz'
        Push-Location $package
        try {
            Invoke-Checked 'creating the installer self-test archive' {
                tar -czf $archive numinous-main
            }
        } finally {
            Pop-Location
        }
        Get-Source -ArchivePath $archive -InstallRoot $sourceRoot `
            -SourceDir $sourceDir -BinaryDir $binaryDir
        if (-not (Test-Path -LiteralPath (Join-Path $sourceDir 'trusted.txt')) -or
            (Test-Path -LiteralPath (Join-Path $sourceDir 'untrusted.txt')) -or
            (Test-Path -LiteralPath (Join-Path $sourceDir 'target\cached.txt')) -or
            -not (Test-Path -LiteralPath (Join-Path $sourceOutside 'radio\keep.txt'))) {
            Fail 'provenance self-test: pre-existing source or build cache influenced the update.'
        }
        Say 'Windows installer root, uninstall, and provenance checks: pass.'
    } finally {
        Remove-DirectoryOrJunction $testBase
    }
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
    Remove-ValidatedInstallRoot $NuminousHome
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

function Get-Source(
    [string]$ArchivePath = '',
    [string]$InstallRoot = $NuminousHome,
    [string]$SourceDir = $SrcDir,
    [string]$BinaryDir = $BinDir
) {
    if (-not (Test-InstallMarker $InstallRoot)) {
        Fail 'source installation requires a marked install root.'
    }
    if (-not (Have 'tar')) {
        Fail 'tar.exe is required to extract the trusted source snapshot.'
    }
    $stage = Join-Path $InstallRoot ('.staging-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage | Out-Null
    try {
        $archive = Join-Path $stage 'numinous.tar.gz'
        if ($ArchivePath) {
            Copy-Item -LiteralPath $ArchivePath -Destination $archive
        } else {
            Say "Downloading the trusted source snapshot from $RepoUrl"
            Invoke-WebRequest -UseBasicParsing -Uri $SnapshotUrl -OutFile $archive
        }
        Invoke-Checked 'extracting the trusted source snapshot' {
            tar -xzf $archive -C $stage
        }
        $newTree = Join-Path $stage 'numinous-main'
        if (-not (Test-Path -LiteralPath $newTree -PathType Container)) {
            Fail 'unexpected source snapshot layout.'
        }
        if (Test-Path -LiteralPath $BinaryDir) {
            $binaryItem = Get-Item -LiteralPath $BinaryDir -Force
            if ($binaryItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                Remove-DirectoryOrJunction $BinaryDir
            } else {
                Remove-DirectoryOrJunction (Join-Path $BinaryDir 'radio')
            }
        }
        Remove-DirectoryOrJunction $SourceDir
        Move-Item -LiteralPath $newTree -Destination $SourceDir
    } finally {
        Remove-DirectoryOrJunction $stage
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
    Initialize-InstallRoot
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
        Test-InstallerSafety
    } elseif ($Uninstall) {
        Uninstall-Numinous
    } else {
        Install-Numinous
    }
} catch {
    Write-Host "numinous install: $($_.Exception.Message)" -ForegroundColor Red
    if ($PSCommandPath) { exit 1 }
}
