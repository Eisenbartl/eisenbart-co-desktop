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
$storageSource = Get-Content -LiteralPath (Join-Path $repository 'src-tauri/src/storage.rs') -Raw
$schemaMatch = [regex]::Match($storageSource, 'const SCHEMA_VERSION:\s*i32\s*=\s*(\d+);')
if (-not $schemaMatch.Success) { throw 'The workspace schema version could not be determined.' }
$schemaVersion = $schemaMatch.Groups[1].Value
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

WHAT YOU CAN CHECK
Client profiles and notes; exact budgets; debt payoff comparisons; emergency and
sinking-fund savings; dated cash flow; encrypted backup/recovery; CSV and selectable-
text PDF transaction review; recurring-payment and running-balance review; approved-
data dashboards; saved scenarios; counselor and client-plan PDF reports.

NEW IN THIS VERSION
The dashboard lists saved tasks and appointments across your scoped clients.
Overdue, due-today, upcoming and completed/cancelled views; kind/name/title filters;
optional archived clients; and 25-item pages are available. Open a row to review
the exact current client follow-up. Dates and times use America/Chicago; reminders
appear inside the app. Intake notes remain separate from financial facts/reports.

TRY THIS
1. Create a fictional client; open Intake and follow-ups to try the new controls.
   Save optional intake and a dated task/appointment. Return to Dashboard, apply
   follow-up filters and open the item; complete or cancel it in the client record.
   Save normal income and necessary expenses in the separate budget.
2. Save debt, savings and cash-flow models if you want those forecasts.
3. Open Compare combined financial scenarios and reload the saved baseline.
4. Model a temporary income reduction and review its monthly funding/recovery.
5. Save the reviewed scenario, select it in an internal counselor assessment and
   export a PDF to a private folder. Restart and check the saved records.
6. Create an encrypted backup in Settings using your workspace password.

This is an evaluation build. Use fictional records only. There is no forgotten-
password reset; reinstalling cannot recover a lost password. Scanned PDF OCR,
advanced financial terms and other original product requirements remain open.

The Windows workflow runs frontend/backend tests, silent installation, an installed
x64 responsive native window, and uninstall. These checks do not establish the
full Windows client workflow, a security review or clean Windows 11 offline install.
The installer includes the WebView2 offline redistributable when packaging succeeds.

UPDATING AN EARLIER VERSION
Close the application before installing. Preserve your existing workspace and its
password; never delete the data folder to fix setup. If an older preview has no
backup controls, first copy the entire closed-app data folder to a safe location.
This version uses database schema $schemaVersion. Older applications may not open
it. Recovery generations and workspace-active.json select restored records; do
not move or edit those files individually. Use supported encrypted backup/restore.

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
