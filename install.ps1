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
    $Version = "v0.0.7"
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

function Remove-StalePluginCacheVersions {
    param(
        [string]$CacheRoot,
        [string]$ActiveVersion
    )

    if (-not (Test-Path $CacheRoot)) {
        return
    }

    Get-ChildItem -Force -Path $CacheRoot |
        Where-Object { $_.PSIsContainer -and $_.Name -ne $ActiveVersion } |
        Remove-Item -Recurse -Force
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
$MarketplaceName = "ccc-local"
$MarketplaceDir = Join-Path $InstallRoot "plugin-marketplace"
$PluginDir = Join-Path $MarketplaceDir "plugins\ccc"
$CodexHome = if ([string]::IsNullOrWhiteSpace($env:CODEX_HOME)) { Join-Path $env:USERPROFILE ".codex" } else { $env:CODEX_HOME }
$CodexConfig = Join-Path $CodexHome "config.toml"
$PluginCacheDir = Join-Path $CodexHome "plugins\cache\$MarketplaceName\ccc\$($Version.TrimStart('v'))"
$LegacyCapSkillDir = Join-Path $CodexHome "skills\cap"

function Set-CccPluginMarketplace {
    param(
        [string]$CurrentRoot
    )

    if (Test-Path $PluginDir) {
        Remove-Item -Recurse -Force $PluginDir
    }
    New-Item -ItemType Directory -Force -Path $PluginDir | Out-Null
    Get-ChildItem -Force -Path $CurrentRoot | Copy-Item -Destination $PluginDir -Recurse -Force
    $PackagedCapSkill = Join-Path $PluginDir "share\skills\cap\SKILL.md"
    if (Test-Path $PackagedCapSkill) {
        $PluginCapSkillDir = Join-Path $PluginDir "skills\cap"
        if (Test-Path $PluginCapSkillDir) {
            Remove-Item -Recurse -Force $PluginCapSkillDir
        }
        New-Item -ItemType Directory -Force -Path $PluginCapSkillDir | Out-Null
        Copy-Item -Force $PackagedCapSkill (Join-Path $PluginCapSkillDir "SKILL.md")
    }

    if (Test-Path (Join-Path $PluginDir "bin\ccc.exe")) {
        $PluginCacheExe = Join-Path $PluginCacheDir "bin\ccc.exe"
    } else {
        $PluginCacheExe = Join-Path $PluginCacheDir "bin\ccc"
    }

    $mcpJson = @{
        mcpServers = @{
            ccc = @{
                command = $PluginCacheExe
                args = @("mcp")
            }
        }
    } | ConvertTo-Json -Depth 8
    Set-Content -Encoding UTF8 -Path (Join-Path $PluginDir ".mcp.json") -Value $mcpJson

    if (Test-Path $PluginCacheDir) {
        Remove-Item -Recurse -Force $PluginCacheDir
    }
    New-Item -ItemType Directory -Force -Path $PluginCacheDir | Out-Null
    Get-ChildItem -Force -Path $PluginDir | Copy-Item -Destination $PluginCacheDir -Recurse -Force
    Remove-StalePluginCacheVersions -CacheRoot (Split-Path $PluginCacheDir -Parent) -ActiveVersion (Split-Path $PluginCacheDir -Leaf)
    if (Test-Path $LegacyCapSkillDir) {
        Remove-Item -Recurse -Force $LegacyCapSkillDir
    }

    $marketplaceJson = @{
        name = $MarketplaceName
        interface = @{
            displayName = "CCC local"
        }
        plugins = @(
            @{
                name = "ccc"
                source = @{
                    source = "local"
                    path = "./plugins/ccc"
                }
                policy = @{
                    installation = "AVAILABLE"
                    authentication = "ON_INSTALL"
                }
                category = "Engineering"
            }
        )
    } | ConvertTo-Json -Depth 8
    $MarketplaceMetadataDir = Join-Path $MarketplaceDir ".agents\plugins"
    New-Item -ItemType Directory -Force -Path $MarketplaceMetadataDir | Out-Null
    Set-Content -Encoding UTF8 -Path (Join-Path $MarketplaceMetadataDir "marketplace.json") -Value $marketplaceJson

    New-Item -ItemType Directory -Force -Path (Split-Path $CodexConfig) | Out-Null
    if (-not (Test-Path $CodexConfig)) {
        New-Item -ItemType File -Force -Path $CodexConfig | Out-Null
    }

    $lines = Get-Content -Path $CodexConfig
    $filtered = New-Object System.Collections.Generic.List[string]
    $skip = $false
    foreach ($line in $lines) {
        if ($line -match '^\[mcp_servers\.ccc(\.|])' -or
            $line -match '^\[marketplaces\.ccc-local]' -or
            $line -match '^\[plugins\."ccc@ccc-local"]') {
            $skip = $true
            continue
        }
        if ($line -match '^\[') {
            $skip = $false
        }
        if (-not $skip) {
            $filtered.Add($line)
        }
    }

    $filtered.Add("")
    $filtered.Add("[marketplaces.$MarketplaceName]")
    $filtered.Add('source_type = "local"')
    $filtered.Add("source = '$MarketplaceDir'")
    $filtered.Add("")
    $filtered.Add("[plugins.`"ccc@$MarketplaceName`"]")
    $filtered.Add("enabled = true")
    Set-Content -Encoding UTF8 -Path $CodexConfig -Value $filtered
}

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

    $ReleaseExe = $CccBinary

    if (Test-Path $ReleaseDir) {
        Remove-Item -Recurse -Force $ReleaseDir
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $ReleaseDir) | Out-Null
    New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null
    Copy-Item -Recurse -Force (Join-Path $ExtractDir "*") $ReleaseDir

    Write-Output "Running setup..."
    & $ReleaseExe setup

    Write-Output ""
    Write-Output "Running check-install..."
    & $ReleaseExe check-install

    Write-Output ""
    Write-Output "Refreshing Codex CCC plugin marketplace..."
    Set-CccPluginMarketplace -CurrentRoot $ReleaseDir

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

    Write-Output ""
    Write-Output "Install complete."
    Write-Output "Installed root: $CurrentDir"
    Write-Output "Command shim: $TargetCmd"
    Write-Output "Plugin marketplace: $MarketplaceDir"
    Write-Output "Restart Codex CLI before using `$cap or relying on the new CCC plugin registration."
    Write-Output "After restarting Codex CLI, run: codex mcp list"
} finally {
    if (Test-Path $TempRoot) {
        Remove-Item -Recurse -Force $TempRoot
    }
}
