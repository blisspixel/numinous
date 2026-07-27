# Numinous installer for Windows. One line to play, in PowerShell:
#
#   irm https://raw.githubusercontent.com/blisspixel/numinous/main/scripts/install.ps1 | iex
#
# What it does, in order: downloads the latest published release for this
# machine, verifies both archive checksums and closed payload manifests, puts
# numinous, numinous-app, and numinous-mcp in ~\.numinous\bin, installs the
# built-in radio once, and adds that directory to the user PATH.
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
# Options: -Uninstall, -NoModifyPath, -AdoptLegacy, -Source, -SelfTest.
# Set NUMINOUS_HOME to install somewhere other than ~\.numinous.
[CmdletBinding()]
param(
    [switch]$Uninstall,
    [switch]$NoModifyPath,
    [switch]$AdoptLegacy,
    [switch]$Source,
    [switch]$SelfTest,
    [string]$ReleaseArchive = '',
    [string]$ReleaseChecksum = '',
    [string]$SoundtrackArchive = '',
    [string]$SoundtrackChecksum = '',
    [string]$ReleaseTag = '',
    [int]$WaitForProcessId = 0,
    [string]$DeleteInstaller = ''
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = `
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = 'blisspixel/numinous'
$RepoUrl = "https://github.com/$Repo"
$RepoApiUrl = "https://api.github.com/repos/$Repo"
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
$InstallMarkerText = 'Numinous install root v2'
$LegacyInstallMarkerText = 'Numinous install root'

function Say([string]$Message) { Write-Host $Message }
function Fail([string]$Message) { throw $Message }
function Have([string]$Name) {
    return [bool](Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue)
}

function Assert-NoReparseAncestor([string]$Path) {
    $current = $Path
    while (-not [string]::IsNullOrEmpty($current)) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                Fail "NUMINOUS_HOME must not pass through a reparse point: $current"
            }
        }
        $parent = Split-Path -Parent $current
        if ([string]::IsNullOrEmpty($parent) -or $parent -eq $current) { break }
        $current = $parent
    }
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
    Assert-NoReparseAncestor $full
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
$SoundtrackDir = Join-Path $NuminousHome 'soundtrack'

# Run a native command and stop with a clear message if it fails.
function Invoke-Checked([string]$What, [scriptblock]$Action) {
    & $Action
    if ($LASTEXITCODE -ne 0) { Fail "$What failed; the output above says why." }
}

function Get-ReleaseTarget {
    $architecture = if ($env:PROCESSOR_ARCHITEW6432) {
        $env:PROCESSOR_ARCHITEW6432
    } else {
        $env:PROCESSOR_ARCHITECTURE
    }
    if ($architecture -ne 'AMD64') {
        Fail ("published Windows releases currently require x86-64, but this process is " +
            "'$architecture'. Re-run with -Source to build locally.")
    }
    return 'x86_64-pc-windows-msvc'
}

function Get-LatestReleaseTag {
    Say "Checking the latest published release at $RepoUrl"
    $headers = @{
        Accept = 'application/vnd.github+json'
        'User-Agent' = 'numinous-installer'
        'X-GitHub-Api-Version' = '2022-11-28'
    }
    $releases = @(Invoke-RestMethod -UseBasicParsing -Headers $headers `
        -Uri "$RepoApiUrl/releases?per_page=20")
    $release = @($releases | Where-Object { -not $_.draft } | Select-Object -First 1)
    if ($release.Count -ne 1) {
        Fail "no published Numinous release is available yet; use -Source to build main."
    }
    $tag = [string]$release[0].tag_name
    if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
        Fail "the latest release returned an unsafe tag name."
    }
    return $tag
}

function Read-ArchiveChecksum([string]$Path, [string]$ArchiveName) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction Stop
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        $item.Length -gt 1024) {
        Fail 'the release checksum sidecar is not a small ordinary file.'
    }
    $text = [IO.File]::ReadAllText($item.FullName).TrimEnd("`r", "`n")
    $match = [regex]::Match($text, '^([0-9a-f]{64})  ([A-Za-z0-9._-]+)$')
    if (-not $match.Success -or $match.Groups[2].Value -cne $ArchiveName) {
        Fail 'the release checksum sidecar is malformed or names another archive.'
    }
    return $match.Groups[1].Value
}

function Assert-ArchiveChecksum(
    [string]$ArchivePath,
    [string]$ChecksumPath,
    [string]$ArchiveName
) {
    $archive = Get-Item -LiteralPath $ArchivePath -Force -ErrorAction Stop
    if ($archive.PSIsContainer -or
        ($archive.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        Fail 'the release download is not an ordinary archive file.'
    }
    $expected = Read-ArchiveChecksum $ChecksumPath $ArchiveName
    $actual = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -cne $expected) {
        Fail "release archive checksum mismatch for $ArchiveName."
    }
    return $expected
}

function Assert-PayloadManifest([string]$Root) {
    $rootItem = Get-Item -LiteralPath $Root -Force -ErrorAction Stop
    if (-not $rootItem.PSIsContainer -or
        ($rootItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        Fail 'the release payload root is not an ordinary directory.'
    }
    $manifestPath = Join-Path $Root 'MANIFEST.sha256'
    $manifestItem = Get-Item -LiteralPath $manifestPath -Force -ErrorAction Stop
    if ($manifestItem.PSIsContainer -or
        ($manifestItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        $manifestItem.Length -gt 1024 * 1024) {
        Fail 'the release payload manifest is not a bounded ordinary file.'
    }
    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $rootPrefix = "$rootFull\"
    $listed = @{}
    foreach ($line in [IO.File]::ReadAllLines($manifestPath)) {
        $match = [regex]::Match($line, '^([0-9a-f]{64})  ([A-Za-z0-9._/-]+)$')
        if (-not $match.Success) { Fail 'the release payload manifest is malformed.' }
        $relative = $match.Groups[2].Value
        if (@($relative -split '/' | Where-Object { $_ -in @('', '.', '..') }).Count -ne 0 -or
            $relative.Contains('\')) {
            Fail 'the release payload manifest contains an unsafe path.'
        }
        $candidate = [IO.Path]::GetFullPath((Join-Path $Root ($relative -replace '/', '\')))
        if (-not $candidate.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
            Fail 'the release payload manifest escapes its root.'
        }
        if ($listed.ContainsKey($relative)) {
            Fail 'the release payload manifest contains a duplicate path.'
        }
        $item = Get-Item -LiteralPath $candidate -Force -ErrorAction Stop
        if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            Fail "release payload entry is not an ordinary file: $relative"
        }
        $actual = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -cne $match.Groups[1].Value) {
            Fail "release payload checksum mismatch: $relative"
        }
        $listed[$relative] = $true
    }
    if ($listed.Count -eq 0) { Fail 'the release payload manifest is empty.' }
    $allItems = @(Get-ChildItem -LiteralPath $Root -Recurse -Force)
    if (@($allItems | Where-Object {
            $_.Attributes -band [IO.FileAttributes]::ReparsePoint
        }).Count -ne 0) {
        Fail 'the release payload contains a reparse point.'
    }
    $receiptPath = Join-Path $rootFull '.archive.sha256'
    $actualFiles = @($allItems | Where-Object {
        -not $_.PSIsContainer -and $_.FullName -cne $manifestItem.FullName -and
            $_.FullName -cne $receiptPath
    })
    foreach ($item in $actualFiles) {
        $relative = $item.FullName.Substring($rootPrefix.Length).Replace('\', '/')
        if (-not $listed.ContainsKey($relative)) {
            Fail "release payload contains an unlisted file: $relative"
        }
    }
    if ($actualFiles.Count -ne $listed.Count) {
        Fail 'the release payload inventory differs from its manifest.'
    }
}

function Assert-SafeArchiveMembers([string]$ArchivePath, [string]$ExpectedRoot) {
    $members = @(& tar -tzf $ArchivePath)
    if ($LASTEXITCODE -ne 0 -or $members.Count -eq 0) {
        Fail 'the release tar archive could not be listed.'
    }
    foreach ($member in $members) {
        if ($member.Contains('\') -or $member.Contains('//') -or
            @($member -split '/' | Where-Object { $_ -in @('.', '..') }).Count -ne 0 -or
            -not ($member -ceq $ExpectedRoot -or $member.StartsWith("$ExpectedRoot/"))) {
            Fail 'the release tar archive contains an unsafe member path.'
        }
    }
}

function Assert-SafeZipMembers([string]$ArchivePath, [string]$ExpectedRoot) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        if ($archive.Entries.Count -eq 0) { Fail 'the release ZIP archive is empty.' }
        $seen = New-Object 'Collections.Generic.HashSet[string]' `
            ([StringComparer]::Ordinal)
        [long]$totalLength = 0
        foreach ($entry in $archive.Entries) {
            $name = $entry.FullName
            if ([string]::IsNullOrEmpty($name) -or $name.Contains('\') -or
                $name.Contains('//') -or $name -notmatch '^[A-Za-z0-9._/-]+$' -or
                @($name -split '/' | Where-Object { $_ -in @('.', '..') }).Count -ne 0 -or
                -not ($name -ceq $ExpectedRoot -or $name.StartsWith("$ExpectedRoot/"))) {
                Fail 'the release ZIP archive contains an unsafe member path.'
            }
            if (-not $seen.Add($name)) {
                Fail 'the release ZIP archive contains a duplicate member path.'
            }
            $unixKind = ($entry.ExternalAttributes -shr 16) -band 0xF000
            $isDirectory = $name.EndsWith('/')
            if (($isDirectory -and $unixKind -notin @(0, 0x4000)) -or
                (-not $isDirectory -and $unixKind -notin @(0, 0x8000))) {
                Fail 'the release ZIP archive contains a non-file member.'
            }
            $totalLength += $entry.Length
            if ($entry.Length -gt 512MB -or $totalLength -gt 1GB) {
                Fail 'the release ZIP archive expands beyond its size budget.'
            }
        }
    } finally {
        $archive.Dispose()
    }
}

function Install-ReleasePayload(
    [string]$ArchivePath,
    [string]$Destination,
    [string]$ExpectedRoot,
    [string]$ArchiveHash,
    [string]$ExpectedTag,
    [string]$ExpectedKind,
    [string]$ExpectedTarget
) {
    if (-not (Test-InstallMarker $NuminousHome)) {
        Fail 'release installation requires a marked install root.'
    }
    $stage = Join-Path $NuminousHome ('.release-stage-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $stage | Out-Null
    try {
        if ($ArchivePath.EndsWith('.zip', [StringComparison]::OrdinalIgnoreCase)) {
            Assert-SafeZipMembers $ArchivePath $ExpectedRoot
            Expand-Archive -LiteralPath $ArchivePath -DestinationPath $stage
        } elseif ($ArchivePath.EndsWith('.tar.gz', [StringComparison]::OrdinalIgnoreCase)) {
            Assert-SafeArchiveMembers $ArchivePath $ExpectedRoot
            Invoke-Checked 'extracting the release tar archive' {
                tar -xzf $ArchivePath -C $stage
            }
        } else {
            Fail 'the release archive has an unsupported extension.'
        }
        $newTree = Join-Path $stage $ExpectedRoot
        Assert-PayloadManifest $newTree
        $metadataPath = Join-Path $newTree 'RELEASE.json'
        $metadata = Get-Content -Raw -LiteralPath $metadataPath | ConvertFrom-Json
        if ($metadata.schema -cne 'numinous.release' -or $metadata.schemaVersion -ne 1 -or
            $metadata.tag -cne $ExpectedTag -or $metadata.kind -cne $ExpectedKind -or
            $metadata.target -cne $ExpectedTarget) {
            Fail 'the release metadata does not match the requested payload.'
        }
        [IO.File]::WriteAllText(
            (Join-Path $newTree '.archive.sha256'),
            "$ArchiveHash`r`n",
            (New-Object Text.UTF8Encoding($false)))
        Remove-DirectoryOrJunction $Destination
        Move-Item -LiteralPath $newTree -Destination $Destination
    } finally {
        Remove-DirectoryOrJunction $stage
    }
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

function Protect-InstallDirectory([string]$Path) {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $system = New-Object Security.Principal.SecurityIdentifier('S-1-5-18')
    $existing = Get-Acl -LiteralPath $Path
    if ($existing.AreAccessRulesProtected -and
        $existing.GetOwner([Security.Principal.SecurityIdentifier]).Value -eq
            $currentUser.Value) {
        $explicitRules = @($existing.GetAccessRules(
            $true,
            $false,
            [Security.Principal.SecurityIdentifier]))
        $expectedIdentities = @($currentUser.Value, $system.Value)
        $safeRules = @($explicitRules | Where-Object {
            $_.AccessControlType -eq [Security.AccessControl.AccessControlType]::Allow -and
            ($_.FileSystemRights -band [Security.AccessControl.FileSystemRights]::FullControl) -eq
                [Security.AccessControl.FileSystemRights]::FullControl -and
            $_.IdentityReference.Value -in $expectedIdentities
        })
        if ($explicitRules.Count -eq 2 -and $safeRules.Count -eq 2 -and
            @($safeRules.IdentityReference.Value | Sort-Object -Unique).Count -eq 2) {
            return
        }
    }
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $acl.SetAccessRuleProtection($true, $false)
    $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor `
        [Security.AccessControl.InheritanceFlags]::ObjectInherit
    $propagation = [Security.AccessControl.PropagationFlags]::None
    $allow = [Security.AccessControl.AccessControlType]::Allow
    foreach ($identity in @($currentUser, $system)) {
        $rule = New-Object Security.AccessControl.FileSystemAccessRule(
            $identity,
            [Security.AccessControl.FileSystemRights]::FullControl,
            $inheritance,
            $propagation,
            $allow)
        [void]$acl.AddAccessRule($rule)
    }
    $acl.SetOwner($currentUser)
    Set-Acl -LiteralPath $Path -AclObject $acl
}

function New-RustupStage([string]$Parent) {
    $parentItem = Get-Item -LiteralPath $Parent -Force -ErrorAction Stop
    if (-not $parentItem.PSIsContainer -or
        ($parentItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        Fail 'rustup staging requires a private ordinary install directory.'
    }
    $parentAcl = Get-Acl -LiteralPath $Parent
    if (-not $parentAcl.AreAccessRulesProtected) {
        Fail 'rustup staging requires a protected install directory.'
    }

    for ($attempt = 0; $attempt -lt 8; $attempt++) {
        $stage = Join-Path $Parent ('.rustup-stage-' + [Guid]::NewGuid().ToString('N'))
        if (Test-Path -LiteralPath $stage) { continue }
        New-Item -ItemType Directory -Path $stage -ErrorAction Stop | Out-Null
        Protect-InstallDirectory $stage
        $item = Get-Item -LiteralPath $stage -Force
        $acl = Get-Acl -LiteralPath $stage
        if ($item.PSIsContainer -and
            -not ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -and
            $acl.AreAccessRulesProtected) {
            return $stage
        }
        Remove-DirectoryOrJunction $stage
        Fail 'rustup staging could not establish a private ordinary directory.'
    }
    Fail 'rustup staging could not allocate a unique directory.'
}

function Complete-StagedFile([string]$Stage, [string]$Destination) {
    $backup = Join-Path (Split-Path -Parent $Destination) `
        ('.numinous-' + [Guid]::NewGuid().ToString('N') + '.old')
    try {
        $item = Get-Item -LiteralPath $Destination -Force -ErrorAction SilentlyContinue
        if ($null -eq $item) {
            [IO.File]::Move($Stage, $Destination)
            return
        }
        if ($item.PSIsContainer -or
            ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            Fail "refusing an unsafe file destination: $Destination"
        }
        [IO.File]::Replace($Stage, $Destination, $backup, $true)
        Remove-Item -LiteralPath $backup -Force
    } finally {
        Remove-Item -LiteralPath $Stage -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue
    }
}

function Publish-Bytes([byte[]]$Bytes, [string]$Destination) {
    $directory = Split-Path -Parent $Destination
    $stage = Join-Path $directory ('.numinous-' + [Guid]::NewGuid().ToString('N') + '.tmp')
    try {
        $stream = [IO.File]::Open(
            $stage,
            [IO.FileMode]::CreateNew,
            [IO.FileAccess]::Write,
            [IO.FileShare]::None)
        try {
            $stream.Write($Bytes, 0, $Bytes.Length)
            $stream.Flush($true)
        } finally {
            $stream.Dispose()
        }
        Complete-StagedFile $stage $Destination
    } finally {
        Remove-Item -LiteralPath $stage -Force -ErrorAction SilentlyContinue
    }
}

function Publish-File([string]$Source, [string]$Destination) {
    $directory = Split-Path -Parent $Destination
    $stage = Join-Path $directory ('.numinous-' + [Guid]::NewGuid().ToString('N') + '.tmp')
    try {
        $sourceStream = [IO.File]::Open(
            $Source,
            [IO.FileMode]::Open,
            [IO.FileAccess]::Read,
            [IO.FileShare]::Read)
        try {
            $destinationStream = [IO.File]::Open(
                $stage,
                [IO.FileMode]::CreateNew,
                [IO.FileAccess]::Write,
                [IO.FileShare]::None)
            try {
                $sourceStream.CopyTo($destinationStream)
                $destinationStream.Flush($true)
            } finally {
                $destinationStream.Dispose()
            }
        } finally {
            $sourceStream.Dispose()
        }
        Complete-StagedFile $stage $Destination
    } finally {
        Remove-Item -LiteralPath $stage -Force -ErrorAction SilentlyContinue
    }
}

function Test-FailedPublicationCleanup([string]$Directory) {
    $source = Join-Path $Directory 'cleanup-source.bin'
    $destination = Join-Path $Directory 'failed-publication.bin'
    [IO.File]::WriteAllText($source, 'source')
    New-Item -ItemType Directory -Path $destination | Out-Null
    $rejected = $false
    try { Publish-File $source $destination } catch { $rejected = $true }
    if (-not $rejected) {
        Fail 'publication self-test: an unsafe destination was unexpectedly published.'
    }
    if (-not (Test-Path -LiteralPath $destination -PathType Container) -or
        @(Get-ChildItem -LiteralPath $Directory -Filter '.numinous-*.tmp' -Force).Count -ne 0) {
        Fail 'publication self-test: failed publication left staged state.'
    }
}

function Protect-InstallMarkerPayload([string]$Root) {
    Add-Type -AssemblyName System.Security
    $encoding = New-Object Text.UTF8Encoding($false)
    $rootBytes = $encoding.GetBytes([IO.Path]::GetFullPath($Root))
    $entropy = $encoding.GetBytes($InstallMarkerText)
    $protected = [Security.Cryptography.ProtectedData]::Protect(
        $rootBytes,
        $entropy,
        [Security.Cryptography.DataProtectionScope]::CurrentUser)
    return [Convert]::ToBase64String($protected)
}

function Test-LegacyInstallMarker([string]$Root) {
    $marker = Join-Path $Root $InstallMarkerName
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { return $false }
    $item = Get-Item -LiteralPath $marker -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -or $item.Length -gt 64) {
        return $false
    }
    $content = [IO.File]::ReadAllText($marker)
    return $content -ceq $LegacyInstallMarkerText -or
        $content -ceq "$LegacyInstallMarkerText`n" -or
        $content -ceq "$LegacyInstallMarkerText`r`n"
}

function Test-InstallMarker([string]$Root) {
    $marker = Join-Path $Root $InstallMarkerName
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { return $false }
    $item = Get-Item -LiteralPath $marker -Force
    if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { return $false }
    if ($item.Length -gt 4096) { return $false }
    $lines = @([IO.File]::ReadAllLines($marker))
    if ($lines.Count -ne 2 -or $lines[0] -cne $InstallMarkerText) { return $false }
    try {
        Add-Type -AssemblyName System.Security
        $encoding = New-Object Text.UTF8Encoding($false)
        $protected = [Convert]::FromBase64String($lines[1])
        $entropy = $encoding.GetBytes($InstallMarkerText)
        $rootBytes = [Security.Cryptography.ProtectedData]::Unprotect(
            $protected,
            $entropy,
            [Security.Cryptography.DataProtectionScope]::CurrentUser)
        $claimedRoot = $encoding.GetString($rootBytes)
        return $claimedRoot -ieq [IO.Path]::GetFullPath($Root)
    } catch {
        return $false
    }
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
    $payload = Protect-InstallMarkerPayload $Root
    Publish-Bytes ($encoding.GetBytes("$InstallMarkerText`r`n$payload`r`n")) $marker
}

function Test-LegacyInstallRoot([string]$Root) {
    if (-not (Test-Path -LiteralPath $Root -PathType Container)) {
        return $false
    }
    $children = @(Get-ChildItem -LiteralPath $Root -Force)
    if ($children.Count -lt 2 -or $children.Count -gt 3 -or
        @($children | Where-Object { $_.Name -notin @('src', 'bin', $InstallMarkerName) }).Count -ne 0 -or
        @($children | Where-Object {
            $_.Name -in @('src', 'bin') -and
                (-not $_.PSIsContainer -or ($_.Attributes -band [IO.FileAttributes]::ReparsePoint))
        }).Count -ne 0) {
        return $false
    }
    if ((Test-Path -LiteralPath (Join-Path $Root $InstallMarkerName)) -and
        -not (Test-LegacyInstallMarker $Root)) {
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

function Test-InstallRootClaimable(
    [string]$Root,
    [string]$DefaultRoot = (Join-Path $HOME '.numinous'),
    [bool]$AllowLegacy = $false
) {
    return (Test-InstallMarker $Root) -or
        (Test-DirectoryEmpty $Root) -or
        ($AllowLegacy -and
            [IO.Path]::GetFullPath($Root) -ieq [IO.Path]::GetFullPath($DefaultRoot) -and
            (Test-LegacyInstallRoot $Root))
}

function Initialize-InstallRoot(
    [string]$Root = $NuminousHome,
    [string]$DefaultRoot = (Join-Path $HOME '.numinous'),
    [bool]$AllowLegacy = [bool]$AdoptLegacy
) {
    $Root = Resolve-InstallRoot $Root $HOME
    if (Test-Path -LiteralPath $Root) {
        $isLegacy = [IO.Path]::GetFullPath($Root) -ieq [IO.Path]::GetFullPath($DefaultRoot) -and
            (Test-LegacyInstallRoot $Root)
        if ($isLegacy -and -not $AllowLegacy) {
            Fail 'a legacy default install needs explicit -AdoptLegacy consent before migration.'
        }
        if (-not (Test-InstallRootClaimable $Root $DefaultRoot $AllowLegacy)) {
            Fail 'NUMINOUS_HOME exists but is not a marked Numinous install root.'
        }
    } else {
        New-Item -ItemType Directory -Path $Root | Out-Null
    }
    $rechecked = Resolve-InstallRoot $Root $HOME
    if ($rechecked -ine $Root) {
        Fail 'NUMINOUS_HOME changed while the installer was starting.'
    }
    if (-not (Test-InstallRootClaimable $Root $DefaultRoot $AllowLegacy)) {
        Fail 'NUMINOUS_HOME contents changed while the installer was starting.'
    }
    Protect-InstallDirectory $Root
    Write-InstallMarker $Root
}

function Remove-ValidatedInstallRoot(
    [string]$Root,
    [string]$DefaultRoot = (Join-Path $HOME '.numinous'),
    [bool]$AllowLegacy = [bool]$AdoptLegacy
) {
    $resolved = Resolve-InstallRoot $Root $HOME
    if (-not (Test-Path -LiteralPath $resolved)) { return }
    $marked = Test-InstallMarker $resolved
    $isDefault = $resolved -ieq [IO.Path]::GetFullPath($DefaultRoot)
    $legacy = $isDefault -and (Test-LegacyInstallRoot $resolved)
    if ($legacy -and -not $AllowLegacy) {
        Fail 'a legacy default install needs explicit -AdoptLegacy consent before removal.'
    }
    if (-not $marked -and -not ($legacy -and $AllowLegacy)) {
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
    Remove-Item -LiteralPath (Join-Path $resolved $InstallMarkerName) `
        -Force -ErrorAction SilentlyContinue
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
    Protect-InstallDirectory $testBase
    Protect-InstallDirectory $testBase
    try {
        $firstRustupStage = New-RustupStage $testBase
        $secondRustupStage = New-RustupStage $testBase
        if ($firstRustupStage -eq $secondRustupStage -or
            -not (Test-Path -LiteralPath $firstRustupStage -PathType Container) -or
            -not (Test-Path -LiteralPath $secondRustupStage -PathType Container)) {
            Fail 'rustup staging self-test: unique private directories were not created.'
        }
        Remove-DirectoryOrJunction $firstRustupStage
        Remove-DirectoryOrJunction $secondRustupStage

        $hadFailureProbe = Test-Path Env:NUMINOUS_INSTALLER_TEST_FAILURE
        $previousFailureProbe = $env:NUMINOUS_INSTALLER_TEST_FAILURE
        $previousErrorActionPreference = $ErrorActionPreference
        $env:NUMINOUS_INSTALLER_TEST_FAILURE = '1'
        $ErrorActionPreference = 'Continue'
        try {
            Get-Content -Raw -LiteralPath $PSCommandPath |
                powershell -NoProfile -ExecutionPolicy Bypass -Command - 2>$null | Out-Null
            $failureStatus = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
            if ($hadFailureProbe) {
                $env:NUMINOUS_INSTALLER_TEST_FAILURE = $previousFailureProbe
            } else {
                Remove-Item Env:NUMINOUS_INSTALLER_TEST_FAILURE
            }
        }
        if ($failureStatus -eq 0) {
            Fail 'failure-status self-test: in-memory invocation swallowed a terminating error.'
        }

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
        [IO.File]::WriteAllText(
            (Join-Path $legacy $InstallMarkerName),
            "$LegacyInstallMarkerText`r`n")
        if (Test-InstallRootClaimable $legacy) {
            Fail 'root self-test: a custom legacy install was accepted without user-bound identity.'
        }
        if (Test-InstallRootClaimable $legacy $legacy) {
            Fail 'root self-test: a legacy default install migrated without explicit consent.'
        }
        if (-not (Test-InstallRootClaimable $legacy $legacy $true)) {
            Fail 'root self-test: the exact default legacy install shape could not migrate.'
        }
        Set-Content -LiteralPath (Join-Path $legacy 'unexpected.txt') -Value 'keep'
        if (Test-InstallRootClaimable $legacy $legacy $true) {
            Fail 'root self-test: a legacy root with unexpected contents was accepted.'
        }
        Remove-Item -LiteralPath (Join-Path $legacy 'unexpected.txt') -Force
        $rejectedArbitrary = $false
        try { Initialize-InstallRoot $unmarked } catch { $rejectedArbitrary = $true }
        if (-not $rejectedArbitrary) {
            Fail 'root self-test: arbitrary nonempty contents were accepted.'
        }
        Initialize-InstallRoot $legacy $legacy $true
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
        [IO.File]::WriteAllText(
            (Join-Path $legacyUninstall $InstallMarkerName),
            "$LegacyInstallMarkerText`r`n")
        $rejectedLegacyWithoutConsent = $false
        try {
            Remove-ValidatedInstallRoot $legacyUninstall $legacyUninstall
        } catch {
            $rejectedLegacyWithoutConsent = $true
        }
        if (-not $rejectedLegacyWithoutConsent -or -not (Test-Path -LiteralPath $legacyUninstall)) {
            Fail 'uninstall self-test: a legacy default install was removed without explicit consent.'
        }
        Remove-ValidatedInstallRoot $legacyUninstall $legacyUninstall $true
        if (Test-Path -LiteralPath $legacyUninstall) {
            Fail 'uninstall self-test: the exact legacy install was retained.'
        }

        $forged = Join-Path $testBase 'forged-marker'
        New-Item -ItemType Directory -Path $forged | Out-Null
        [IO.File]::WriteAllText(
            (Join-Path $forged $InstallMarkerName),
            "$LegacyInstallMarkerText`r`n")
        Set-Content -LiteralPath (Join-Path $forged 'keep.txt') -Value 'keep'
        $rejectedForged = $false
        try { Remove-ValidatedInstallRoot $forged $forged } catch { $rejectedForged = $true }
        if (-not $rejectedForged -or -not (Test-Path -LiteralPath $forged)) {
            Fail 'uninstall self-test: a forged public marker authorized default-root removal.'
        }

        $ancestorTarget = Join-Path $testBase 'ancestor-target'
        New-Item -ItemType Directory -Path (Join-Path $ancestorTarget 'parent') -Force | Out-Null
        $ancestorLink = Join-Path $testBase 'ancestor-link'
        New-Item -ItemType Junction -Path $ancestorLink -Target $ancestorTarget | Out-Null
        $rejectedAncestor = $false
        try {
            [void](Resolve-InstallRoot (Join-Path $ancestorLink 'parent\root') $HOME)
        } catch {
            $rejectedAncestor = $true
        }
        if (-not $rejectedAncestor) {
            Fail 'root self-test: an older ancestor junction was accepted.'
        }

        $publication = Join-Path $testBase 'publication'
        New-Item -ItemType Directory -Path $publication | Out-Null
        $victim = Join-Path $publication 'victim.bin'
        $destination = Join-Path $publication 'numinous.exe'
        $replacement = Join-Path $publication 'replacement.bin'
        [IO.File]::WriteAllText($victim, 'victim-before')
        New-Item -ItemType HardLink -Path $destination -Target $victim | Out-Null
        [IO.File]::WriteAllText($replacement, 'replacement-binary')
        Publish-File $replacement $destination
        if ([IO.File]::ReadAllText($victim) -cne 'victim-before' -or
            [IO.File]::ReadAllText($destination) -cne 'replacement-binary') {
            Fail 'publication self-test: binary replacement wrote through an existing hardlink.'
        }
        Test-FailedPublicationCleanup $publication

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
    $stage = New-RustupStage $NuminousHome
    try {
        $rustupInit = Join-Path $stage 'rustup-init.exe'
        Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/$arch" -OutFile $rustupInit
        $download = Get-Item -LiteralPath $rustupInit -Force
        if ($download.PSIsContainer -or
            ($download.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            Fail 'rustup download did not produce an ordinary file.'
        }
        $rustupArgs = @('-y', '--default-toolchain', 'none')
        if ($NoModifyPath) { $rustupArgs += '--no-modify-path' }
        Invoke-Checked 'rustup' { & $rustupInit @rustupArgs }
    } finally {
        Remove-DirectoryOrJunction $stage
    }
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
        if (-not [version]::TryParse($version, [ref]$parsed) -or $parsed -lt [version]'1.88') {
            Fail ('this cargo is older than the minimum supported Rust (1.88) and rustup is ' +
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

function Copy-ReleaseFile(
    [string]$ProvidedPath,
    [string]$Url,
    [string]$Destination,
    [string]$Description
) {
    if ($ProvidedPath) {
        $provided = Get-Item -LiteralPath $ProvidedPath -Force -ErrorAction Stop
        if ($provided.PSIsContainer -or
            ($provided.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            Fail "$Description fixture is not an ordinary file."
        }
        Copy-Item -LiteralPath $provided.FullName -Destination $Destination
    } else {
        Say "Downloading $Description"
        Invoke-WebRequest -UseBasicParsing -Headers @{ 'User-Agent' = 'numinous-installer' } `
            -Uri $Url -OutFile $Destination
    }
}

function Test-InstalledSoundtrack([string]$ExpectedHash) {
    if (-not (Test-Path -LiteralPath $SoundtrackDir -PathType Container)) { return $false }
    $receipt = Join-Path $SoundtrackDir '.archive.sha256'
    $receiptItem = Get-Item -LiteralPath $receipt -Force -ErrorAction SilentlyContinue
    if ($null -eq $receiptItem -or $receiptItem.PSIsContainer -or
        ($receiptItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        $receiptItem.Length -gt 128) {
        return $false
    }
    if ([IO.File]::ReadAllText($receipt).Trim() -cne $ExpectedHash) { return $false }
    try {
        Assert-PayloadManifest $SoundtrackDir
        return $true
    } catch {
        return $false
    }
}

function Install-LatestRelease {
    $tag = if ($ReleaseTag) { $ReleaseTag } else { Get-LatestReleaseTag }
    if ($tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
        Fail 'the requested release tag is unsafe.'
    }
    if ([bool]$ReleaseArchive -ne [bool]$ReleaseChecksum -or
        [bool]$SoundtrackArchive -ne [bool]$SoundtrackChecksum) {
        Fail 'local release fixtures require matching archive and checksum paths.'
    }
    $target = Get-ReleaseTarget
    $payloadRoot = "numinous-$tag-$target"
    $payloadName = "$payloadRoot.zip"
    $soundtrackRoot = "numinous-$tag-soundtrack"
    $soundtrackName = "$soundtrackRoot.tar.gz"
    $downloadStage = Join-Path $NuminousHome ('.download-' + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $downloadStage | Out-Null
    try {
        $payloadPath = Join-Path $downloadStage $payloadName
        $payloadChecksumPath = Join-Path $downloadStage "$payloadName.sha256"
        $releaseBase = "$RepoUrl/releases/download/$tag"
        Copy-ReleaseFile $ReleaseArchive "$releaseBase/$payloadName" `
            $payloadPath 'the Windows release payload'
        Copy-ReleaseFile $ReleaseChecksum "$releaseBase/$payloadName.sha256" `
            $payloadChecksumPath 'the Windows payload checksum'
        $payloadHash = Assert-ArchiveChecksum `
            $payloadPath $payloadChecksumPath $payloadName

        $soundtrackChecksumPath = Join-Path $downloadStage "$soundtrackName.sha256"
        Copy-ReleaseFile $SoundtrackChecksum "$releaseBase/$soundtrackName.sha256" `
            $soundtrackChecksumPath 'the soundtrack checksum'
        $soundtrackHash = Read-ArchiveChecksum $soundtrackChecksumPath $soundtrackName

        Remove-DirectoryOrJunction (Join-Path $BinDir 'radio')
        Install-ReleasePayload $payloadPath $SrcDir $payloadRoot $payloadHash `
            $tag 'binaries' $target

        if (Test-InstalledSoundtrack $soundtrackHash) {
            Say 'The verified built-in soundtrack is already current.'
        } else {
            $soundtrackPath = Join-Path $downloadStage $soundtrackName
            Copy-ReleaseFile $SoundtrackArchive "$releaseBase/$soundtrackName" `
                $soundtrackPath 'the built-in soundtrack'
            [void](Assert-ArchiveChecksum `
                $soundtrackPath $soundtrackChecksumPath $soundtrackName)
            Install-ReleasePayload $soundtrackPath $SoundtrackDir $soundtrackRoot `
                $soundtrackHash $tag 'soundtrack' 'all'
        }
    } finally {
        Remove-DirectoryOrJunction $downloadStage
    }
    $script:InstalledReleaseTag = $tag
}

function Assert-BinaryDestinationsReplaceable {
    foreach ($binary in $Binaries) {
        $destination = Join-Path $BinDir $binary
        if (-not (Test-Path -LiteralPath $destination)) { continue }
        try {
            $stream = [IO.File]::Open(
                $destination,
                [IO.FileMode]::Open,
                [IO.FileAccess]::ReadWrite,
                [IO.FileShare]::None)
            $stream.Dispose()
        } catch {
            Fail "cannot update $binary while it is running; close Numinous and try again."
        }
    }
}

function Install-Binaries(
    [string]$BinarySourceDir = (Join-Path $SrcDir 'target\release'),
    [string]$RadioSource = (Join-Path $SrcDir 'assets\radio')
) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    Protect-InstallDirectory $BinDir
    Assert-BinaryDestinationsReplaceable
    foreach ($binary in $Binaries) {
        $from = Join-Path $BinarySourceDir $binary
        try {
            Publish-File $from (Join-Path $BinDir $binary)
        } catch {
            Fail "could not replace $binary; close any running Numinous windows and re-run."
        }
    }
    # The app finds the built-in radio next to its executable. A junction
    # avoids duplicating the tracks; fall back to a copy if it is refused.
    $radioLink = Join-Path $BinDir 'radio'
    Remove-DirectoryOrJunction $radioLink
    try {
        New-Item -ItemType Junction -Path $radioLink -Target $RadioSource | Out-Null
    } catch {
        Copy-Item $RadioSource $radioLink -Recurse
    }
}

function Install-Numinous {
    if ($PSVersionTable.PSVersion.Major -lt 5) {
        Fail 'this installer needs Windows PowerShell 5.1 or newer.'
    }
    Initialize-InstallRoot
    if ($Source) {
        Test-BuildTools
        Install-Rust
        Get-Source
        Build-Numinous
        Install-Binaries
    } else {
        Install-LatestRelease
        Install-Binaries (Join-Path $SrcDir 'bin') (Join-Path $SoundtrackDir 'radio')
    }
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
    if ($Source) {
        Say 'This source build follows main. Re-run with -Source to update it.'
    } else {
        Say "Installed release: $script:InstalledReleaseTag"
        Say 'Update any time with: numinous update'
    }
    Say 'Uninstall with -Uninstall.'
}

try {
    if ($env:NUMINOUS_INSTALLER_TEST_FAILURE) {
        Fail 'intentional installer failure-status probe.'
    } elseif ($SelfTest) {
        Test-PathPromotion
        Test-InstallerSafety
    } elseif ($Uninstall) {
        Uninstall-Numinous
    } else {
        if ($Source -and ($ReleaseArchive -or $ReleaseChecksum -or
                $SoundtrackArchive -or $SoundtrackChecksum -or $ReleaseTag)) {
            Fail '-Source cannot be combined with release fixture options.'
        }
        if ($WaitForProcessId -lt 0 -or $WaitForProcessId -eq $PID) {
            Fail 'the update helper received an invalid parent process id.'
        }
        if ($WaitForProcessId -gt 0) {
            Say 'Waiting for the running Numinous command to close before updating.'
            Wait-Process -Id $WaitForProcessId -ErrorAction SilentlyContinue
        }
        Install-Numinous
    }
} catch {
    Write-Host "numinous install: $($_.Exception.Message)" -ForegroundColor Red
    if ($PSCommandPath) { exit 1 }
    throw
} finally {
    if ($DeleteInstaller) {
        try {
            $candidate = [IO.Path]::GetFullPath($DeleteInstaller)
            $temporaryRoot = [IO.Path]::GetFullPath($env:TEMP).TrimEnd('\') + '\'
            $name = [IO.Path]::GetFileName($candidate)
            if ($candidate.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase) -and
                $name -match '^numinous-update-[0-9a-f]{32}\.ps1$') {
                Remove-Item -LiteralPath $candidate -Force -ErrorAction SilentlyContinue
            }
        } catch {}
    }
}
