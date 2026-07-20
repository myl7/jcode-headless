<#
.SYNOPSIS
    Install jcode on Windows.
.DESCRIPTION
    Downloads the latest jcode release and installs it to %LOCALAPPDATA%\jcode\bin.

    One-liner install:
      irm https://jcode.sh/install.ps1 | iex

    Or download and run (allows parameters):
      & ([scriptblock]::Create((irm https://jcode.sh/install.ps1)))
.PARAMETER InstallDir
    Override the installation directory (default: $env:LOCALAPPDATA\jcode\bin)
.PARAMETER Version
    Override the version tag to install. Required when using a local artifact path.
.PARAMETER ArtifactExePath
    Use a local jcode.exe artifact instead of downloading from GitHub.
.PARAMETER ArtifactTgzPath
    Use a local jcode .tar.gz artifact instead of downloading from GitHub.
.PARAMETER BuildFromSource
    If no prebuilt release asset is available, explicitly allow a source build.
    Source builds require Git, Rust, and the Visual Studio C++ Build Tools.
#>
param(
    [string]$InstallDir,
    [string]$Version,
    [string]$ArtifactExePath,
    [string]$ArtifactTgzPath,
    [switch]$BuildFromSource
)

$ErrorActionPreference = 'Stop'

if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Host "error: PowerShell 5.1 or later is required" -ForegroundColor Red
    exit 1
}

$Repo = "1jehuang/jcode"
$ReleaseMetadataBase = if ($env:JCODE_RELEASE_METADATA_BASE) {
    $env:JCODE_RELEASE_METADATA_BASE.TrimEnd('/')
} else {
    "https://jcode.sh/releases"
}

if (-not $InstallDir) {
    $localAppData = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData) }
    if (-not $localAppData -and $env:USERPROFILE) { $localAppData = Join-Path $env:USERPROFILE "AppData\Local" }
    $InstallDir = Join-Path $localAppData "jcode\bin"
}

function Write-Info($msg) { Write-Host $msg -ForegroundColor Blue }
function Write-Err($msg) { throw "error: $msg" }
function Write-Warn($msg) { Write-Host "warning: $msg" -ForegroundColor Yellow }

function ConvertFrom-JcodeWebContent($Content) {
    if ($null -eq $Content) { return "" }

    # Windows PowerShell 5.1 returns Byte[] for some text responses when the
    # server uses application/octet-stream (including jcode.sh metadata and
    # GitHub release checksum manifests). Casting Byte[] directly to [string]
    # produces a space-separated list of decimal bytes instead of the text.
    if ($Content -is [byte[]]) {
        return [System.Text.Encoding]::UTF8.GetString($Content)
    }

    return [string]$Content
}

function Test-JcodeReleaseTag([string]$Tag) {
    return [bool]($Tag -match '^v[0-9]+\.[0-9]+\.[0-9]+(?:[+.-][A-Za-z0-9.-]+)?$')
}

function Resolve-JcodeReleaseTagFromUri([string]$Uri) {
    if (-not $Uri) { return $null }
    if ($Uri -match '/releases/tag/([^/?#]+)') {
        return [Uri]::UnescapeDataString($Matches[1])
    }
    return $null
}

function Get-LatestJcodeReleaseTag {
    # Avoid api.github.com here. Its unauthenticated limit is only 60 requests
    # per public IP per hour, so installs are unreliable behind shared NAT/VPNs.
    $metadataTag = $null
    try {
        $metadataResponse = Invoke-WebRequest -UseBasicParsing -Uri "$ReleaseMetadataBase/latest/version"
        $candidate = (ConvertFrom-JcodeWebContent -Content $metadataResponse.Content).Trim()
        if (Test-JcodeReleaseTag $candidate) { $metadataTag = $candidate }
    } catch {}

    try {
        $response = Invoke-WebRequest -UseBasicParsing -Method Head -Uri "https://github.com/$Repo/releases/latest"
        $baseResponse = $response.BaseResponse
        $resolvedUri = $null

        if ($baseResponse) {
            $responseUriProperty = $baseResponse.PSObject.Properties['ResponseUri']
            if ($responseUriProperty -and $responseUriProperty.Value) {
                $resolvedUri = [string]$responseUriProperty.Value
            }

            if (-not $resolvedUri) {
                $requestMessageProperty = $baseResponse.PSObject.Properties['RequestMessage']
                if ($requestMessageProperty -and $requestMessageProperty.Value) {
                    $resolvedUri = [string]$requestMessageProperty.Value.RequestUri
                }
            }
        }

        $tag = Resolve-JcodeReleaseTagFromUri $resolvedUri
        if ($tag) { return $tag }
    } catch {
        if (-not $metadataTag) {
            Write-Err "Failed to determine latest version: $_"
        }
    }

    if ($metadataTag) {
        Write-Warn "GitHub release lookup unavailable; using cached jcode.sh metadata ($metadataTag)."
        return $metadataTag
    }
    Write-Err "Failed to determine latest version"
}

function Get-JcodeReleaseDownloadBases([string]$ReleaseTag) {
    $bases = New-Object System.Collections.Generic.List[string]
    try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri "$ReleaseMetadataBase/$ReleaseTag/download-bases"
        foreach ($line in ((ConvertFrom-JcodeWebContent -Content $response.Content) -split "`r?`n")) {
            $candidate = $line.Trim().TrimEnd('/')
            if ($candidate -match '^https://\S+$' -and -not $bases.Contains($candidate)) {
                $bases.Add($candidate)
            }
        }
    } catch {}

    $githubBase = "https://github.com/$Repo/releases/download/$ReleaseTag"
    if (-not $bases.Contains($githubBase)) { $bases.Add($githubBase) }
    return $bases.ToArray()
}

function Get-JcodeSha256FromManifest {
    param(
        [Parameter(Mandatory = $true)][string]$ManifestText,
        [Parameter(Mandatory = $true)][string]$AssetName
    )

    foreach ($line in ($ManifestText -split "`r?`n")) {
        if ($line -match '^\s*([0-9a-fA-F]{64})\s+\*?(.+?)\s*$') {
            $candidateName = [System.IO.Path]::GetFileName($Matches[2])
            if ($candidateName -eq $AssetName) {
                return $Matches[1].ToLowerInvariant()
            }
        }
    }

    return $null
}

function Get-ReleaseChecksum([string]$ReleaseTag, [string]$AssetName) {
    $lastError = $null
    foreach ($checksumUrl in @(
        "$ReleaseMetadataBase/$ReleaseTag/SHA256SUMS",
        "https://github.com/$Repo/releases/download/$ReleaseTag/SHA256SUMS"
    )) {
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $checksumUrl
            $expected = Get-JcodeSha256FromManifest -ManifestText (ConvertFrom-JcodeWebContent -Content $response.Content) -AssetName $AssetName
            if ($expected) { return $expected }
        } catch {
            $lastError = $_
        }
    }

    if ($lastError) {
        Write-Err "Could not download SHA256SUMS for $ReleaseTag. Refusing to install an unverified download: $lastError"
    }
    Write-Err "SHA256SUMS for $ReleaseTag does not list $AssetName"
}

function Assert-JcodeFileChecksum([string]$FilePath, [string]$ExpectedSha256, [string]$AssetName) {
    try {
        $actual = (Get-FileHash -LiteralPath $FilePath -Algorithm SHA256).Hash.ToLowerInvariant()
    } catch {
        Write-Err "Could not calculate SHA256 for ${AssetName}: $_"
    }

    if ($actual -ne $ExpectedSha256) {
        Remove-Item -LiteralPath $FilePath -Force -ErrorAction SilentlyContinue
        Write-Err "SHA256 verification failed for $AssetName (expected $ExpectedSha256, got $actual)"
    }

    Write-Info "Verified SHA256: $AssetName"
    return $actual
}

function Get-JcodeLocalAppDataDir {
    if ($env:LOCALAPPDATA) {
        return $env:LOCALAPPDATA
    }

    $localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
    if ($localAppData) {
        return $localAppData
    }

    if ($env:USERPROFILE) {
        return (Join-Path $env:USERPROFILE "AppData\Local")
    }

    return (Join-Path ([Environment]::GetFolderPath("UserProfile")) "AppData\Local")
}

function Get-DefaultJcodeInstallDir {
    return (Join-Path (Get-JcodeLocalAppDataDir) "jcode\bin")
}

function ConvertTo-JcodePathKey([string]$PathValue) {
    if (-not $PathValue) {
        return ""
    }

    $clean = [Environment]::ExpandEnvironmentVariables($PathValue.Trim().Trim('"'))
    if (-not $clean) {
        return ""
    }

    try {
        $clean = [System.IO.Path]::GetFullPath($clean)
    } catch {
    }


    $clean = $clean.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    return $clean.ToUpperInvariant()
}

function Split-JcodePathList([string]$PathValue) {
    if (-not $PathValue) {
        return @()
    }

    $entries = @()
    foreach ($entry in ($PathValue -split ';')) {
        $clean = $entry.Trim().Trim('"')
        if ($clean) {
            $entries += $clean
        }
    }
    return $entries
}

function Join-JcodePathList([string[]]$Entries) {
    if (-not $Entries -or $Entries.Count -eq 0) {
        return ""
    }

    return ($Entries -join ';')
}

function Get-JcodeManagedPathKeys([string]$InstallDir) {
    $keys = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($candidate in @($InstallDir, (Get-DefaultJcodeInstallDir))) {
        $key = ConvertTo-JcodePathKey $candidate
        if ($key) {
            [void]$keys.Add($key)
        }
    }
    return $keys
}

function Resolve-JcodePathUpdate {
    param(
        [Parameter(Mandatory = $true)][string]$InstallDir,
        [AllowNull()][string]$CurrentPath,
        [switch]$RemoveOnly
    )

    $managedKeys = Get-JcodeManagedPathKeys -InstallDir $InstallDir
    $nextEntries = @()
    $removedManaged = 0

    foreach ($entry in (Split-JcodePathList $CurrentPath)) {
        $key = ConvertTo-JcodePathKey $entry
        if (-not $key) {
            continue
        }

        if ($managedKeys.Contains($key)) {
            $removedManaged += 1
            continue
        }

        $nextEntries += $entry
    }

    if (-not $RemoveOnly) {
        $nextEntries = @($InstallDir) + $nextEntries
    }

    $nextPath = Join-JcodePathList $nextEntries
    $changed = ($nextPath -ne ([string]$CurrentPath))

    return [pscustomobject]@{
        Path = $nextPath
        Changed = $changed
        RemovedManagedEntries = $removedManaged
        RemovedDuplicateEntries = 0
        AddedLauncherEntry = (-not $RemoveOnly)
        InstallDir = $InstallDir
    }
}

function Send-JcodeEnvironmentChangedBroadcast {
    if ($env:JCODE_DISABLE_ENV_BROADCAST -eq "1") {
        return $false
    }

    if (-not ("Jcode.EnvironmentBroadcast" -as [type])) {
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
namespace Jcode {
    public static class EnvironmentBroadcast {
        [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
        public static extern IntPtr SendMessageTimeout(
            IntPtr hWnd,
            UInt32 Msg,
            UIntPtr wParam,
            string lParam,
            UInt32 fuFlags,
            UInt32 uTimeout,
            out UIntPtr lpdwResult);
    }
}
"@
    }

    $result = [UIntPtr]::Zero
    [Jcode.EnvironmentBroadcast]::SendMessageTimeout([IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, "Environment", 0x0002, 5000, [ref]$result) | Out-Null
    return $true
}

function Set-JcodeUserPath {
    param(
        [Parameter(Mandatory = $true)][string]$InstallDir,
        [AllowNull()][string]$CurrentPath,
        [scriptblock]$SetUserPathAction,
        [scriptblock]$BroadcastAction,
        [bool]$Broadcast = $true
    )

    if (-not $PSBoundParameters.ContainsKey('CurrentPath')) {
        $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    }

    $update = Resolve-JcodePathUpdate -InstallDir $InstallDir -CurrentPath $CurrentPath
    $broadcasted = $false

    if ($update.Changed) {
        if ($SetUserPathAction) {
            & $SetUserPathAction $update.Path
        } else {
            [Environment]::SetEnvironmentVariable("Path", $update.Path, "User")
        }

        if ($Broadcast) {
            if ($BroadcastAction) {
                & $BroadcastAction | Out-Null
            } else {
                Send-JcodeEnvironmentChangedBroadcast | Out-Null
            }
            $broadcasted = $true
        }
    }

    $update | Add-Member -NotePropertyName Broadcasted -NotePropertyValue $broadcasted
    return $update
}

function Remove-JcodeUserPath {
    param(
        [Parameter(Mandatory = $true)][string]$InstallDir,
        [AllowNull()][string]$CurrentPath,
        [scriptblock]$SetUserPathAction,
        [scriptblock]$BroadcastAction,
        [bool]$Broadcast = $true
    )

    if (-not $PSBoundParameters.ContainsKey('CurrentPath')) {
        $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    }

    $update = Resolve-JcodePathUpdate -InstallDir $InstallDir -CurrentPath $CurrentPath -RemoveOnly
    $broadcasted = $false

    if ($update.Changed) {
        if ($SetUserPathAction) {
            & $SetUserPathAction $update.Path
        } else {
            [Environment]::SetEnvironmentVariable("Path", $update.Path, "User")
        }

        if ($Broadcast) {
            if ($BroadcastAction) {
                & $BroadcastAction | Out-Null
            } else {
                Send-JcodeEnvironmentChangedBroadcast | Out-Null
            }
            $broadcasted = $true
        }
    }

    $update | Add-Member -NotePropertyName Broadcasted -NotePropertyValue $broadcasted
    return $update
}

function Set-JcodeProcessPath([string]$InstallDir) {
    $update = Resolve-JcodePathUpdate -InstallDir $InstallDir -CurrentPath $env:Path
    $env:Path = $update.Path
    return $update
}

function Remove-JcodeStaleLauncherBackups {
    param(
        [Parameter(Mandatory = $true)][string]$LauncherDir
    )

    Get-ChildItem -LiteralPath $LauncherDir -Filter '.jcode-launcher-old-*.exe' -File -Force -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

function Install-JcodeLauncher {
    param(
        [Parameter(Mandatory = $true)][string]$SourcePath,
        [Parameter(Mandatory = $true)][string]$LauncherPath
    )

    $launcherDir = Split-Path -Parent $LauncherPath
    New-Item -ItemType Directory -Path $launcherDir -Force | Out-Null

    $operationId = [guid]::NewGuid().ToString('N')
    $tempLauncher = Join-Path $launcherDir (".jcode-launcher-{0}.tmp.exe" -f $operationId)
    $oldLauncher = Join-Path $launcherDir (".jcode-launcher-old-{0}.exe" -f $operationId)
    $movedExistingLauncher = $false
    try {
        Copy-Item -Path $SourcePath -Destination $tempLauncher -Force
        if (Test-Path -LiteralPath $LauncherPath) {
            # Windows will not overwrite a loaded executable, but it does allow
            # the directory entry to be renamed while the process keeps running
            # from its existing file handle. Move the old launcher aside first,
            # then atomically put the new binary at the stable PATH location.
            Move-Item -LiteralPath $LauncherPath -Destination $oldLauncher
            $movedExistingLauncher = $true
        }

        try {
            Move-Item -LiteralPath $tempLauncher -Destination $LauncherPath
        } catch {
            if ($movedExistingLauncher -and -not (Test-Path -LiteralPath $LauncherPath)) {
                Move-Item -LiteralPath $oldLauncher -Destination $LauncherPath
                $movedExistingLauncher = $false
            }
            throw
        }

        if ($movedExistingLauncher) {
            # Removal succeeds immediately for an idle launcher. If an older
            # jcode process still has the renamed executable loaded, Windows
            # keeps it until that process exits and the next install cleans it.
            Remove-Item -LiteralPath $oldLauncher -Force -ErrorAction SilentlyContinue
        }

        # Only prune backups after the stable path contains the new launcher.
        # Doing this before replacement could delete another concurrent
        # installer's rollback file during its short rename window.
        Remove-JcodeStaleLauncherBackups -LauncherDir $launcherDir
    } finally {
        Remove-Item -LiteralPath $tempLauncher -Force -ErrorAction SilentlyContinue
    }

    return $LauncherPath
}

function Resolve-OptionalPath([string]$PathValue) {
    if (-not $PathValue) {
        return $null
    }

    try {
        return (Resolve-Path -LiteralPath $PathValue -ErrorAction Stop).Path
    } catch {
        Write-Err "Provided path does not exist: $PathValue"
    }
}

function Stop-ProcessTree([int]$ProcessId) {
    try {
        Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
            Where-Object { $_.ParentProcessId -eq $ProcessId } |
            ForEach-Object { Stop-ProcessTree -ProcessId $_.ProcessId }
    } catch {}

    try {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    } catch {}
}

function Invoke-ProcessWithTimeout {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$ArgumentList = @(),
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [Parameter(Mandatory = $true)][string]$FriendlyName,
        [switch]$CaptureOutput
    )

    $startParams = @{
        FilePath = $FilePath
        ArgumentList = $ArgumentList
        PassThru = $true
        NoNewWindow = $true
    }

    $stdoutPath = $null
    $stderrPath = $null
    if ($CaptureOutput) {
        $stdoutPath = Join-Path $env:TEMP ("jcode-{0}-{1}-stdout.log" -f $FriendlyName, [guid]::NewGuid().ToString('N'))
        $stderrPath = Join-Path $env:TEMP ("jcode-{0}-{1}-stderr.log" -f $FriendlyName, [guid]::NewGuid().ToString('N'))
        $startParams.RedirectStandardOutput = $stdoutPath
        $startParams.RedirectStandardError = $stderrPath
    }

    $process = Start-Process @startParams
    # Wait-Process did not gain -Timeout until newer PowerShell releases. Use
    # the underlying .NET Process API so the documented PowerShell 5.1 minimum
    # is real rather than only passing on PowerShell 7.
    $timedOut = -not $process.WaitForExit($TimeoutSeconds * 1000)
    if ($timedOut) {
        Stop-ProcessTree -ProcessId $process.Id
        return [pscustomobject]@{
            TimedOut = $true
            ExitCode = $null
            StdoutPath = $stdoutPath
            StderrPath = $stderrPath
        }
    }

    # Ensure redirected streams have finished flushing before callers inspect
    # their files, then refresh ExitCode from the completed process.
    $process.WaitForExit()
    $process.Refresh()
    return [pscustomobject]@{
        TimedOut = $false
        ExitCode = $process.ExitCode
        StdoutPath = $stdoutPath
        StderrPath = $stderrPath
    }
}

function Write-LogTail([string]$Path, [string]$Label) {
    if (-not $Path -or -not (Test-Path $Path)) {
        return
    }

    $lines = Get-Content -Path $Path -Tail 40 -ErrorAction SilentlyContinue
    if ($lines -and $lines.Count -gt 0) {
        Write-Warn "$Label (last 40 lines):"
        $lines | ForEach-Object { Write-Host $_ }
    }
}

function Resolve-JcodeWindowsArtifact([string[]]$ArchitectureCandidates) {
    $sawX64 = $false

    foreach ($arch in @($ArchitectureCandidates)) {
        if (-not $arch) { continue }
        switch -Regex ($arch.Trim()) {
            '^(Arm64|ARM64|AARCH64|aarch64)$' { return "jcode-windows-aarch64" }
            '^(X64|AMD64|x86_64)$' { $sawX64 = $true }
        }
    }

    if ($sawX64) { return "jcode-windows-x86_64" }
    return $null
}

function Get-JcodeWindowsArtifact {
    $candidates = @()

    try {
        $runtimeArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
        if ($runtimeArch) { $candidates += [string]$runtimeArch }
    } catch {}

    foreach ($envArch in @($env:PROCESSOR_ARCHITECTURE, $env:PROCESSOR_ARCHITEW6432)) {
        if ($envArch) { $candidates += [string]$envArch }
    }

    $artifact = Resolve-JcodeWindowsArtifact $candidates
    if ($artifact) { return $artifact }

    $displayArch = if ($candidates.Count -gt 0) { $candidates -join ", " } else { "<unknown>" }
    Write-Err "Unsupported architecture: $displayArch (supported: x86_64, ARM64)"
}

function Invoke-JcodeInstall {
$Artifact = Get-JcodeWindowsArtifact

$ResolvedArtifactExePath = Resolve-OptionalPath $ArtifactExePath
$ResolvedArtifactTgzPath = Resolve-OptionalPath $ArtifactTgzPath

if ($ResolvedArtifactExePath -and $ResolvedArtifactTgzPath) {
    Write-Err "Provide only one of -ArtifactExePath or -ArtifactTgzPath"
}

if (-not $Version) {
    if ($ResolvedArtifactExePath) {
        $Version = Get-JcodeVersionFromBinary $ResolvedArtifactExePath
        if (-not $Version) {
            Write-Err "Could not detect a jcode version from '$ResolvedArtifactExePath'. Pass -Version explicitly if this is a trusted local build."
        }
        Write-Info "Detected local artifact version: $Version"
    } elseif ($ResolvedArtifactTgzPath) {
        Write-Err "-Version is required when using -ArtifactTgzPath"
    } else {
        Write-Info "Fetching latest release..."
        $Version = Get-LatestJcodeReleaseTag
    }
}

if (-not $Version) { Write-Err "Failed to determine latest version" }

$VersionNum = $Version.TrimStart('v')
$DownloadBases = Get-JcodeReleaseDownloadBases $Version

$BuildsDir = Join-Path (Get-JcodeLocalAppDataDir) "jcode\builds"
$StableDir = Join-Path $BuildsDir "stable"
$VersionDir = Join-Path $BuildsDir "versions\$VersionNum"
$LauncherPath = Join-Path $InstallDir "jcode.exe"

$Existing = ""
if (Test-Path $LauncherPath) {
    try { $Existing = & $LauncherPath --version 2>$null | Select-Object -First 1 } catch {}
}

if ($Existing) {
    if ($Existing -match [regex]::Escape($VersionNum)) {
        Write-Info "jcode $Version is already installed - reinstalling"
    } else {
        Write-Info "Updating jcode $Existing -> $Version"
    }
} else {
    Write-Info "Installing jcode $Version"
}
Write-Info "  launcher: $LauncherPath"

foreach ($d in @($InstallDir, $StableDir, $VersionDir)) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}

$TempDir = Join-Path $env:TEMP "jcode-install-$(Get-Random)"
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
$DownloadMode = ""
$DownloadPath = Join-Path $TempDir "jcode.download"
$DownloadedAssetName = $null

if ($ResolvedArtifactExePath) {
    Write-Info "Using local artifact exe: $ResolvedArtifactExePath"
    Copy-Item -Path $ResolvedArtifactExePath -Destination $DownloadPath -Force
    $DownloadMode = "bin"
} elseif ($ResolvedArtifactTgzPath) {
    Write-Info "Using local artifact archive: $ResolvedArtifactTgzPath"
    Copy-Item -Path $ResolvedArtifactTgzPath -Destination $DownloadPath -Force
    $DownloadMode = "tar"
} else {
    foreach ($candidate in @(
        @{ Name = "$Artifact.exe"; Mode = "bin" },
        @{ Name = "$Artifact.tar.gz"; Mode = "tar" }
    )) {
        foreach ($base in $DownloadBases) {
            try {
                Write-Info "Downloading $($candidate.Name) from $base..."
                Invoke-WebRequest -UseBasicParsing -Uri "$base/$($candidate.Name)" -OutFile $DownloadPath
                $DownloadMode = $candidate.Mode
                $DownloadedAssetName = $candidate.Name
                break
            } catch {}
        }
        if ($DownloadMode) { break }
    }
}

if (-not $ResolvedArtifactExePath -and -not $ResolvedArtifactTgzPath -and $DownloadMode) {
    $downloadedAssetName = if ($DownloadMode -eq "bin") { "$Artifact.exe" } else { "$Artifact.tar.gz" }
    $expectedSha256 = Get-ReleaseChecksum -ReleaseTag $Version -AssetName $downloadedAssetName
    Assert-JcodeFileChecksum -FilePath $DownloadPath -ExpectedSha256 $expectedSha256 -AssetName $downloadedAssetName | Out-Null
}

$DestBin = Join-Path $VersionDir "jcode.exe"

if ($DownloadMode -eq "tar") {
    Write-Info "Extracting..."
    tar xzf $DownloadPath -C $TempDir 2>$null
    $SrcBin = Join-Path $TempDir "$Artifact.exe"
    if (-not (Test-Path $SrcBin)) {
        Write-Err "Downloaded archive did not contain expected binary: $Artifact.exe"
    }
    Move-Item -Path $SrcBin -Destination $DestBin -Force
} elseif ($DownloadMode -eq "bin") {
    Move-Item -Path $DownloadPath -Destination $DestBin -Force
} else {
    if (-not $BuildFromSource) {
        $releaseUrl = "https://github.com/$Repo/releases/tag/$Version"
        Write-Err "No prebuilt $Artifact asset was found in $Version. Check $releaseUrl or rerun the downloaded script with -BuildFromSource. The installer will not start a long source build automatically."
    }

    Write-Info "No prebuilt asset found for $Artifact in $Version; -BuildFromSource was requested"
    Assert-JcodeSourceBuildPrerequisites

    $SrcDir = Join-Path $TempDir "jcode-src"
    Write-Info "Cloning $Repo at $Version..."
    $gitCloneResult = Invoke-ProcessWithTimeout -FilePath "git" -ArgumentList @(
        "clone",
        "--depth", "1",
        "--branch", $Version,
        "https://github.com/$Repo.git",
        $SrcDir
    ) -TimeoutSeconds 600 -FriendlyName "git-clone" -CaptureOutput
    if ($gitCloneResult.TimedOut) {
        Write-LogTail -Path $gitCloneResult.StdoutPath -Label "git stdout"
        Write-LogTail -Path $gitCloneResult.StderrPath -Label "git stderr"
        Write-Err "git clone timed out after 600 seconds"
    }
    if ($gitCloneResult.ExitCode -ne 0) {
        Write-LogTail -Path $gitCloneResult.StdoutPath -Label "git stdout"
        Write-LogTail -Path $gitCloneResult.StderrPath -Label "git stderr"
        Write-Err "Failed to clone $Repo at $Version (exit code: $($gitCloneResult.ExitCode))"
    }

    Write-Info "Building jcode from source (this can take several minutes)..."
    $cargoResult = Invoke-ProcessWithTimeout -FilePath "cargo" -ArgumentList @(
        "build", "--release", "--locked", "-p", "jcode", "--bin", "jcode",
        "--manifest-path", (Join-Path $SrcDir "Cargo.toml")
    ) -TimeoutSeconds 1800 -FriendlyName "cargo-build" -CaptureOutput
    if ($cargoResult.TimedOut) {
        Write-LogTail -Path $cargoResult.StdoutPath -Label "cargo stdout"
        Write-LogTail -Path $cargoResult.StderrPath -Label "cargo stderr"
        Write-Err "cargo build timed out after 1800 seconds"
    }
    if ($cargoResult.ExitCode -ne 0) {
        Write-LogTail -Path $cargoResult.StdoutPath -Label "cargo stdout"
        Write-LogTail -Path $cargoResult.StderrPath -Label "cargo stderr"
        Write-Err "cargo build failed (exit code: $($cargoResult.ExitCode))"
    }

    $BuiltBin = Join-Path $SrcDir "target\release\jcode.exe"
    if (-not (Test-Path $BuiltBin)) { Write-Err "Built binary not found at $BuiltBin" }
    Copy-Item -Path $BuiltBin -Destination $DestBin -Force
}

Assert-JcodeBinaryCandidate -BinaryPath $DestBin -ExpectedVersion $Version | Out-Null

$StableBin = Join-Path $StableDir "jcode.exe"
Copy-Item -Path $DestBin -Destination $StableBin -Force
Set-Content -Path (Join-Path $BuildsDir "stable-version") -Value $VersionNum
Install-JcodeLauncher -SourcePath $StableBin -LauncherPath $LauncherPath | Out-Null
} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

# Gracefully reload any running background server onto the freshly installed
# binary (issue #291). `server reload` only reloads a genuinely-older daemon,
# hands its live sessions to the new process, and is a no-op when nothing is
# running, so it is safe to call unconditionally. Best-effort: never fail the
# install over it.
if ($env:JCODE_SKIP_SERVER_RELOAD -ne "1") {
    try {
        & $LauncherPath server reload 2>$null | Out-Null
    } catch {
    }
}

$userPathUpdate = Set-JcodeUserPath -InstallDir $InstallDir
if ($userPathUpdate.Changed) {
    Write-Info "Updated user PATH with $InstallDir"
    if ($userPathUpdate.RemovedManagedEntries -gt 0 -or $userPathUpdate.RemovedDuplicateEntries -gt 0) {
        Write-Info "  removed $($userPathUpdate.RemovedManagedEntries) stale jcode PATH entr$(if ($userPathUpdate.RemovedManagedEntries -eq 1) { 'y' } else { 'ies' }) and $($userPathUpdate.RemovedDuplicateEntries) duplicate entr$(if ($userPathUpdate.RemovedDuplicateEntries -eq 1) { 'y' } else { 'ies' })"
    }
} else {
    Write-Info "User PATH already contains $InstallDir"
}

Set-JcodeProcessPath -InstallDir $InstallDir | Out-Null

Write-Host ""
Write-Info "jcode $Version installed successfully!"
Write-Host ""


if (Get-Command jcode -ErrorAction SilentlyContinue) {
    Write-Info "Run 'jcode --help' to get started."
} else {
    Write-Host "  Open a new terminal window, then run:"
    Write-Host ""
    Write-Host "    jcode --help" -ForegroundColor Green
}
}

if ($env:JCODE_INSTALL_PS1_IMPORT_ONLY -ne "1") {
    Invoke-JcodeInstall
}
