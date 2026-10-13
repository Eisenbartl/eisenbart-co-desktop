#Requires -Version 7.0
<# Create instructions and SHA-256 hashes beside a successfully tested installer. #>
[CmdletBinding()]
param(
    [string]$BundleDirectory = (Join-Path $PSScriptRoot '../src-tauri/target/release/bundle/nsis'),
    [string]$SourceCommit = $env:GITHUB_SHA
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) {
    throw 'Download metadata must be created on Windows beside the tested Windows installer.'
}
$repository = Split-Path -Parent $PSScriptRoot
$configuration = Get-Content -LiteralPath (Join-Path $repository 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$bundle = (Get-Item -LiteralPath $BundleDirectory -ErrorAction Stop).FullName
$installers = @(Get-ChildItem -LiteralPath $bundle -Filter '*.exe' -File)
if ($installers.Count -ne 1) {
    throw "Expected one NSIS installer; found $($installers.Count)."
}
if ([string]::IsNullOrWhiteSpace($SourceCommit)) {
    $SourceCommit = git -C $repository rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'The source commit could not be determined.' }
}
if ($SourceCommit -notmatch '^([0-9a-fA-F]{40}|[0-9a-fA-F]{64})$') {
    throw 'The source commit must be a complete hexadecimal Git commit identifier.'
}
$installer = $installers[0]
$signature = Get-AuthenticodeSignature -LiteralPath $installer.FullName
if ($signature.Status -notin @('NotSigned', 'Valid')) {
    throw "Installer signature verification returned $($signature.Status)."
}
$signatureDescription = if ($signature.Status -eq 'NotSigned') {
    'UNSIGNED DEVELOPMENT INSTALLER. Windows may show an unknown-publisher or SmartScreen warning.'
} else {
    'Windows reports a valid Authenticode signature. This remains a development build.'
}
$checksum = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$runUrl = if ($env:GITHUB_REPOSITORY -and $env:GITHUB_RUN_ID) {
    "https://github.com/$($env:GITHUB_REPOSITORY)/actions/runs/$($env:GITHUB_RUN_ID)"
} else {
    'Local Windows build; no GitHub Actions run identifier supplied.'
}
$readme = @"
Eisenbart & Co. Financial Counseling Suite - Windows development build

Version: $($configuration.version)
Source commit: $SourceCommit
Installer: $($installer.Name)
Build/check log: $runUrl

$signatureDescription

INSTALL ON YOUR WINDOWS DESKTOP
1. Extract the whole downloaded ZIP file into a folder.
2. Open $($installer.Name) and follow the installation instructions.
3. Open Eisenbart Counseling Suite from the Start menu.
4. Use fictional records only and retain the password you choose securely.

This is the first working foundation: local password sign-in, encrypted local
storage, client profiles, notes, statuses, and locking. The full financial
application is still being developed. There is no supported password recovery
or tested backup/restore yet. Do not store real client financial information.

The Windows workflow runs frontend/backend tests and checks silent installation,
the installed x64 program displaying a responsive native window, and uninstall.
This is not a complete Windows client-workflow or security test. A clean Windows
11 offline installation without preinstalled WebView2 still requires testing.
The installer is configured to include the WebView2 offline redistributable.

VERIFY THE DOWNLOAD (optional)
In PowerShell in this extracted folder:
  Get-FileHash -LiteralPath '$($installer.Name)' -Algorithm SHA256
Compare the displayed hash to SHA256SUMS.txt. This checks for download changes;
it does not replace publisher signing or establish production readiness.

Documentation: https://github.com/Eisenbartl/eisenbart-co-desktop/blob/main/docs/windows.md
"@
Set-Content -LiteralPath (Join-Path $bundle 'README.txt') -Value $readme -Encoding utf8NoBOM
Set-Content -LiteralPath (Join-Path $bundle 'SHA256SUMS.txt') -Value "$checksum  $($installer.Name)" -Encoding utf8NoBOM
Write-Host "Prepared download instructions and SHA-256 checksum for $($installer.Name)."
