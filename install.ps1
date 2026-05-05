param(
    [string]$Version = $env:CCC_VERSION,
    [string]$InstallRoot = $env:CCC_INSTALL_ROOT,
    [string]$BinDir = $env:CCC_BIN_DIR,
    [string]$Platform = $env:CCC_PLATFORM,
    [string]$DownloadUrl = $env:CCC_DOWNLOAD_URL,
    [string]$PrintAsset = $env:CCC_PRINT_ASSET
)

$ErrorActionPreference = "Stop"

$Repo = "HoRi0506/Codex-Cli-Captain-Release"
$SupportedPlatforms = @(
    "darwin-arm64",
    "darwin-x86_64",
    "linux-arm64",
    "linux-x86_64",
    "windows-x86_64"
)

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = "v0.0.12-pre"
}
if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
    $InstallRoot = Join-Path $env:LOCALAPPDATA "ccc"
}
if ([string]::IsNullOrWhiteSpace($BinDir)) {
    $BinDir = Join-Path $env:USERPROFILE ".local\bin"
}

function Resolve-CccPlatform {
    param([string]$Override)

    if (-not [string]::IsNullOrWhiteSpace($Override)) {
        if ($SupportedPlatforms -notcontains $Override) {
            throw "Unsupported platform override: $Override`nSupported platforms: $($SupportedPlatforms -join ' ')"
        }
        return $Override
    }

    if ([System.Environment]::OSVersion.Platform -ne "Win32NT") {
        throw "install.ps1 performs native Windows installs only. Use install.sh on macOS or Linux."
    }

    switch ($env:PROCESSOR_ARCHITECTURE) {
        "ARM64" { throw "Windows on ARM is not published for this pre-release yet." }
        "AMD64" { return "windows-x86_64" }
        default { throw "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
}

$ResolvedPlatform = Resolve-CccPlatform -Override $Platform
if (-not $ResolvedPlatform.StartsWith("windows-")) {
    throw "install.ps1 performs native Windows installs only. Resolved platform: $ResolvedPlatform"
}

$AssetName = "ccc-$($Version.TrimStart('v'))-$ResolvedPlatform.tar.gz"
if ($PrintAsset -eq "1") {
    Write-Output $AssetName
    exit 0
}

if ([string]::IsNullOrWhiteSpace($DownloadUrl)) {
    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$AssetName"
}

if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    throw "Missing required command: tar"
}
if (-not (Get-Command codex -ErrorAction SilentlyContinue)) {
    throw "Missing required command: codex"
}

$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("ccc-install-" + [Guid]::NewGuid().ToString("N"))
$ArchivePath = Join-Path $TempRoot $AssetName
$ExtractDir = Join-Path $TempRoot "extract"
$ReleaseDir = Join-Path $InstallRoot "releases\$($Version.TrimStart('v'))-$ResolvedPlatform"
$CurrentDir = Join-Path $InstallRoot "current"
$TargetCmd = Join-Path $BinDir "ccc.cmd"

try {
    New-Item -ItemType Directory -Force -Path $TempRoot, $ExtractDir, $InstallRoot, $BinDir | Out-Null

    Write-Output "Downloading $AssetName..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ArchivePath

    Write-Output "Unpacking $AssetName..."
    tar -xzf $ArchivePath -C $ExtractDir

    $ExtractedExe = Join-Path $ExtractDir "bin\ccc.exe"
    $ExtractedUnixName = Join-Path $ExtractDir "bin\ccc"
    if (Test-Path $ExtractedExe) {
        $CccBinary = $ExtractedExe
    } elseif (Test-Path $ExtractedUnixName) {
        $CccBinary = $ExtractedUnixName
    } else {
        throw "The downloaded bundle does not contain bin\ccc.exe or bin\ccc."
    }

    if ((Get-Item $CccBinary).Length -le 0) {
        throw "The downloaded bundle contains an empty ccc binary."
    }

    if (Test-Path $ReleaseDir) {
        Remove-Item -Recurse -Force $ReleaseDir
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $ReleaseDir) | Out-Null
    New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null
    Copy-Item -Recurse -Force (Join-Path $ExtractDir "*") $ReleaseDir

    if (Test-Path $CurrentDir) {
        Remove-Item -Recurse -Force $CurrentDir
    }
    New-Item -ItemType Directory -Force -Path $CurrentDir | Out-Null
    Copy-Item -Recurse -Force (Join-Path $ReleaseDir "*") $CurrentDir

    $CurrentExe = Join-Path $CurrentDir "bin\ccc.exe"
    if (-not (Test-Path $CurrentExe)) {
        $CurrentExe = Join-Path $CurrentDir "bin\ccc"
    }

    "@echo off`r`n`"$CurrentExe`" %*`r`n" | Set-Content -Encoding ASCII -Path $TargetCmd

    Write-Output "Refreshing Codex MCP registration..."
    try {
        codex mcp remove ccc *> $null
    } catch {
    }
    codex mcp add ccc -- $CurrentExe mcp

    Write-Output "Running setup..."
    & $CurrentExe setup

    Write-Output ""
    Write-Output "Running check-install..."
    & $CurrentExe check-install

    Write-Output ""
    Write-Output "Install complete."
    Write-Output "Installed root: $CurrentDir"
    Write-Output "Command shim: $TargetCmd"
    Write-Output "Restart Codex CLI before using `$cap or relying on the new MCP registration."
    Write-Output "After restarting Codex CLI, run: ccc check-install"
} finally {
    if (Test-Path $TempRoot) {
        Remove-Item -Recurse -Force $TempRoot
    }
}
