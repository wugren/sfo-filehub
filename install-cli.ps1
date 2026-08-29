[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Version,

    [string]$InstallDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Repository = 'wugren/sfo-filehub'
$ApiUrl = "https://api.github.com/repos/$Repository/releases/latest"
$ReleasesUrl = "https://github.com/$Repository/releases/download"
$VersionWasSpecified = $PSBoundParameters.ContainsKey('Version')
$InstallDirWasSpecified = $PSBoundParameters.ContainsKey('InstallDir')

function Normalize-Version {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw 'Version must not be empty when specified.'
    }

    if ($Value -notmatch '^v?([0-9]+\.[0-9]+\.[0-9]+)$') {
        throw "Invalid version '$Value'; expected MAJOR.MINOR.PATCH or vMAJOR.MINOR.PATCH."
    }

    return $Matches[1]
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Normalize-PathEntry {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return ''
    }

    return [IO.Path]::GetFullPath($Value.Trim()).TrimEnd('\', '/')
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'install-cli.ps1 supports Windows only; use install-cli.sh on Linux or macOS.'
}

$DetectedArchitecture = if ($env:PROCESSOR_ARCHITEW6432) {
    $env:PROCESSOR_ARCHITEW6432
} else {
    $env:PROCESSOR_ARCHITECTURE
}

if ($DetectedArchitecture -ne 'AMD64') {
    throw "Unsupported Windows architecture '$DetectedArchitecture'; releases support Windows x86_64 only."
}

if ($InstallDirWasSpecified) {
    if ([string]::IsNullOrWhiteSpace($InstallDir)) {
        throw 'InstallDir must not be empty when specified.'
    }
    $InstallDir = [IO.Path]::GetFullPath($InstallDir)
} else {
    if ([string]::IsNullOrWhiteSpace($env:ProgramFiles)) {
        throw 'ProgramFiles is unavailable; specify -InstallDir explicitly.'
    }
    $InstallDir = Join-Path $env:ProgramFiles 'filehub\bin'
    if (-not (Test-Administrator)) {
        throw 'The default system installation requires an elevated PowerShell session. Run as Administrator or specify -InstallDir.'
    }
}

if ($VersionWasSpecified) {
    $ResolvedVersion = Normalize-Version $Version
} else {
    try {
        $Release = Invoke-RestMethod -Uri $ApiUrl -Headers @{
            Accept = 'application/vnd.github+json'
        }
    } catch {
        throw "Could not resolve the latest GitHub release; specify -Version to retry. $($_.Exception.Message)"
    }

    if ($null -eq $Release -or [string]::IsNullOrWhiteSpace([string]$Release.tag_name)) {
        throw 'Latest GitHub release response has no tag_name; specify -Version to retry.'
    }
    $ResolvedVersion = Normalize-Version ([string]$Release.tag_name)
}

$Tag = "v$ResolvedVersion"
$ArchiveName = "filehub-cli_${ResolvedVersion}_windows-x86_64.tar.gz"
$DownloadUrl = "$ReleasesUrl/$Tag/$ArchiveName"
$TempDir = Join-Path ([IO.Path]::GetTempPath()) ("filehub-install-" + [Guid]::NewGuid().ToString('N'))
$ArchivePath = Join-Path $TempDir $ArchiveName
$ExtractedBinary = Join-Path $TempDir 'filehub.exe'
$TargetPath = Join-Path $InstallDir 'filehub.exe'
$StagedPath = Join-Path $InstallDir ('.filehub.install.' + [Guid]::NewGuid().ToString('N') + '.exe')
$BackupPath = Join-Path $InstallDir ('.filehub.backup.' + [Guid]::NewGuid().ToString('N') + '.exe')
$HadExistingTarget = $false
$TargetWasReplaced = $false
$PreserveBackup = $false

try {
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    Write-Host "Downloading filehub CLI $ResolvedVersion for windows-x86_64..."
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $DownloadUrl -OutFile $ArchivePath
    } catch {
        throw "Download failed: $DownloadUrl. $($_.Exception.Message)"
    }

    if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf) -or (Get-Item -LiteralPath $ArchivePath).Length -eq 0) {
        throw 'Downloaded archive is empty.'
    }

    $TarCommand = Get-Command tar.exe -ErrorAction Stop
    $ArchiveEntries = @(& $TarCommand.Source -tzf $ArchivePath)
    if ($LASTEXITCODE -ne 0) {
        throw 'Downloaded archive is not a readable tar.gz.'
    }
    $ArchiveEntries = @($ArchiveEntries | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($ArchiveEntries.Count -ne 1 -or $ArchiveEntries[0] -cne 'filehub.exe') {
        throw 'Archive must contain exactly one root file named filehub.exe.'
    }
    $ArchiveListing = @(& $TarCommand.Source -tvzf $ArchivePath)
    if ($LASTEXITCODE -ne 0 -or $ArchiveListing.Count -ne 1 -or -not $ArchiveListing[0].StartsWith('-')) {
        throw 'Archive entry filehub.exe must be a regular file.'
    }

    & $TarCommand.Source -xzf $ArchivePath -C $TempDir filehub.exe
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $ExtractedBinary -PathType Leaf)) {
        throw 'Could not extract a non-empty filehub.exe from the archive.'
    }
    $ExtractedItem = Get-Item -LiteralPath $ExtractedBinary -Force
    if (($ExtractedItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $ExtractedItem.Length -eq 0) {
        throw 'Extracted filehub.exe must be a non-empty regular file, not a reparse point.'
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    if (Test-Path -LiteralPath $TargetPath) {
        if (-not (Test-Path -LiteralPath $TargetPath -PathType Leaf)) {
            throw "$TargetPath exists but is not a file."
        }
        $HadExistingTarget = $true
    }

    Copy-Item -LiteralPath $ExtractedBinary -Destination $StagedPath
    if ($HadExistingTarget) {
        [IO.File]::Replace($StagedPath, $TargetPath, $BackupPath, $true)
    } else {
        Move-Item -LiteralPath $StagedPath -Destination $TargetPath
    }
    $TargetWasReplaced = $true

    if (-not $InstallDirWasSpecified) {
        $MachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
        $PathEntries = @($MachinePath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        $NormalizedInstallDir = Normalize-PathEntry $InstallDir
        $AlreadyPresent = $false
        foreach ($Entry in $PathEntries) {
            if ([string]::Equals((Normalize-PathEntry $Entry), $NormalizedInstallDir, [StringComparison]::OrdinalIgnoreCase)) {
                $AlreadyPresent = $true
                break
            }
        }

        if (-not $AlreadyPresent) {
            $NewMachinePath = if ([string]::IsNullOrWhiteSpace($MachinePath)) {
                $InstallDir
            } else {
                $MachinePath.TrimEnd(';') + ';' + $InstallDir
            }
            [Environment]::SetEnvironmentVariable('Path', $NewMachinePath, 'Machine')
        }
    }

    if (Test-Path -LiteralPath $BackupPath -PathType Leaf) {
        try {
            Remove-Item -LiteralPath $BackupPath -Force
        } catch {
            Write-Warning "Installed successfully but could not remove the previous-version backup at $BackupPath."
        }
    }

    Write-Host "Installed filehub CLI $ResolvedVersion to $TargetPath"
    if (-not $InstallDirWasSpecified) {
        Write-Host 'Open a new terminal before running filehub.'
    }
} catch {
    if ($TargetWasReplaced) {
        try {
            if ($HadExistingTarget -and (Test-Path -LiteralPath $BackupPath -PathType Leaf)) {
                [IO.File]::Replace($BackupPath, $TargetPath, $null, $true)
            } elseif (-not $HadExistingTarget -and (Test-Path -LiteralPath $TargetPath -PathType Leaf)) {
                Remove-Item -LiteralPath $TargetPath -Force
            }
        } catch {
            $PreserveBackup = $true
            Write-Warning "Installation failed and the previous binary could not be restored; backup retained at $BackupPath. $($_.Exception.Message)"
        }
    }
    throw
} finally {
    if (Test-Path -LiteralPath $StagedPath -PathType Leaf) {
        Remove-Item -LiteralPath $StagedPath -Force -ErrorAction SilentlyContinue
    }
    if (-not $PreserveBackup -and (Test-Path -LiteralPath $BackupPath -PathType Leaf)) {
        Remove-Item -LiteralPath $BackupPath -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $TempDir) {
        Remove-Item -LiteralPath $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
