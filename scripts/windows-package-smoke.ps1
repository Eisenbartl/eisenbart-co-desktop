#Requires -Version 7.0
<#
Test the real NSIS package on a disposable GitHub-hosted Windows runner.
This checks installation, a responsive native window, and uninstallation. It
does not enter client data or prove the complete workflow or offline install.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not $IsWindows -or $env:GITHUB_ACTIONS -ne 'true' -or
    $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or [string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
    throw 'This installation test is restricted to disposable GitHub-hosted Windows runners.'
}

$repository = Split-Path -Parent $PSScriptRoot
$configuration = Get-Content -LiteralPath (Join-Path $repository 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
if ($configuration.identifier -ne 'com.eisenbart.counseling') {
    throw 'The expected application data identifier changed; review the storage safety guards.'
}
$localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
if ([string]::IsNullOrWhiteSpace($localAppData)) {
    throw 'Windows did not provide a per-user local application-data directory.'
}
$applicationData = [IO.Path]::GetFullPath((Join-Path $localAppData $configuration.identifier))
# Windows uses known folders, not Linux XDG overrides. Never use this script on
# an existing counselor workspace, even if a previous CI test left one behind.
if (Test-Path -LiteralPath $applicationData) {
    throw 'Existing application data found. The installation test will not touch it.'
}
if (Get-Process -Name 'eisenbart-counseling-suite' -ErrorAction SilentlyContinue) {
    throw 'An application process already exists. The installation test will not touch it.'
}
foreach ($registryRoot in @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
)) {
    $existingInstallation = Get-ItemProperty -Path ($registryRoot + '\*') -ErrorAction SilentlyContinue |
        Where-Object {
            $displayName = $_.PSObject.Properties['DisplayName']
            $null -ne $displayName -and $displayName.Value -eq $configuration.productName
        }
    if ($existingInstallation) {
        throw 'An existing application installation was found. The test will not upgrade or remove it.'
    }
}

$installer = (Get-Item -LiteralPath $InstallerPath -ErrorAction Stop).FullName
$runnerTemporaryDirectory = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
$installDirectory = Join-Path $runnerTemporaryDirectory ('eisenbart-package-test-' + [Guid]::NewGuid().ToString('N'))
if (Test-Path -LiteralPath $installDirectory) {
    throw 'The unique installation destination already exists.'
}
$artifactDirectory = Join-Path $runnerTemporaryDirectory 'eisenbart-windows-validation'
New-Item -ItemType Directory -Path $artifactDirectory -Force | Out-Null
$logFile = Join-Path $artifactDirectory 'package-smoke.log'
$utf8 = [Text.UTF8Encoding]::new($false)
$application = $null
$ownsApplicationData = $false
$failure = $null

function Write-Validation([string]$Message) {
    $line = [DateTimeOffset]::UtcNow.ToString('u') + ' ' + $Message
    Write-Host $line
    [IO.File]::AppendAllText($logFile, $line + [Environment]::NewLine, $utf8)
}

function Assert-PortableExecutable([string]$Path, [ushort[]]$AllowedMachines) {
    $stream = [IO.File]::OpenRead($Path)
    $reader = [IO.BinaryReader]::new($stream)
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
            throw 'The executable does not have a valid Windows MZ header.'
        }
        $stream.Position = 60
        $peOffset = $reader.ReadUInt32()
        if ($peOffset -lt 64 -or $peOffset + 24 -gt $stream.Length) {
            throw 'The executable has an invalid PE header location.'
        }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw 'The executable does not have a valid PE signature.'
        }
        $machine = $reader.ReadUInt16()
        if ($machine -notin $AllowedMachines) {
            throw ('Unexpected executable machine type: 0x{0:X4}.' -f $machine)
        }
        Write-Validation ('PASS executable header: {0}; machine 0x{1:X4}' -f [IO.Path]::GetFileName($Path), $machine)
    } finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

function Invoke-PackageCommand([string]$Path, [string]$Arguments, [string]$Description) {
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -PassThru
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(180)
        while (-not $process.WaitForExit(1000)) {
            if ([DateTime]::UtcNow -gt $deadline) {
                $process.Kill($true)
                $process.WaitForExit(10000) | Out-Null
                throw "$Description timed out."
            }
        }
        if ($process.ExitCode -ne 0) {
            throw "$Description exited with code $($process.ExitCode)."
        }
        Write-Validation "PASS $Description exit code 0"
    } finally {
        $process.Dispose()
    }
}

function Stop-TestApplication {
    if ($null -ne $application) {
        try {
            $application.Refresh()
            if (-not $application.HasExited) {
                $application.CloseMainWindow() | Out-Null
                if (-not $application.WaitForExit(15000)) {
                    # Only the process returned by our own Start-Process and its
                    # descendants may be stopped. Never kill by process name.
                    $application.Kill($true)
                    if (-not $application.WaitForExit(10000)) {
                        throw 'The test application did not stop.'
                    }
                }
            }
        } finally {
            $application.Dispose()
            $script:application = $null
        }
    }
}

try {
    Write-Validation 'START NSIS installation and native-window test on disposable Windows CI'
    # NSIS may use a 32-bit installer stub even when it installs an x64 app.
    Assert-PortableExecutable $installer @(0x014C, 0x8664)
    $signature = Get-AuthenticodeSignature -LiteralPath $installer
    if ($signature.Status -notin @('NotSigned', 'Valid')) {
        throw "Installer signature verification returned $($signature.Status)."
    }
    Write-Validation "Installer signature status: $($signature.Status)"
    # NSIS requires /D to be the last argument and its path must not be quoted.
    Invoke-PackageCommand $installer "/S /D=$installDirectory" 'silent installation'
    $installedExecutable = Join-Path $installDirectory 'eisenbart-counseling-suite.exe'
    if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf)) {
        throw 'The installer did not place the expected application executable.'
    }
    Assert-PortableExecutable $installedExecutable @(0x8664)
    # Repeat the known-folder safety check immediately before launching.
    if (Test-Path -LiteralPath $applicationData) {
        throw 'Application data appeared before our launch; preserving it and aborting.'
    }
    $ownsApplicationData = $true
    $application = Start-Process -FilePath $installedExecutable -WorkingDirectory $installDirectory -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        Start-Sleep -Milliseconds 250
        $application.Refresh()
        if ($application.HasExited) {
            throw "The installed application exited before displaying its window (code $($application.ExitCode))."
        }
        if ([DateTime]::UtcNow -gt $deadline) {
            throw 'The installed application did not display its expected native window.'
        }
    } until ($application.MainWindowHandle -ne [IntPtr]::Zero -and
        $application.MainWindowTitle.StartsWith('Eisenbart & Co.') -and $application.Responding)
    Start-Sleep -Seconds 2
    $application.Refresh()
    if ($application.HasExited -or $application.MainWindowHandle -eq [IntPtr]::Zero -or -not $application.Responding) {
        throw 'The installed application did not retain a responsive native window.'
    }
    Write-Validation "PASS installed x64 application displays responsive window: $($application.MainWindowTitle)"
    Write-Validation 'Scope: no setup/password/client workflow was entered; offline installation was not tested.'
    Stop-TestApplication
    Write-Validation 'PASS test-owned application stopped'
} catch {
    $failure = $_
    Write-Validation "FAIL $($_.Exception.Message)"
} finally {
    try {
        Stop-TestApplication
        $uninstaller = Join-Path $installDirectory 'uninstall.exe'
        if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
            Invoke-PackageCommand $uninstaller '/S' 'silent uninstallation'
            # NSIS copies its uninstaller to a private temporary location. Its
            # parent can exit before that copy finishes, so verify file removal.
            $deadline = [DateTime]::UtcNow.AddSeconds(45)
            do {
                $remainingFiles = @()
                if (Test-Path -LiteralPath $installDirectory) {
                    $remainingFiles = @(Get-ChildItem -LiteralPath $installDirectory -File -Recurse -Force)
                }
                if ($remainingFiles.Count -eq 0) { break }
                if ([DateTime]::UtcNow -gt $deadline) {
                    throw 'The uninstaller left files in its unique installation directory.'
                }
                Start-Sleep -Milliseconds 250
            } while ($true)
            Write-Validation 'PASS uninstallation removed the installed files'
        } elseif ($null -eq $failure) {
            throw 'The installed package did not provide its expected uninstaller.'
        }
        if ($ownsApplicationData -and (Test-Path -LiteralPath $applicationData)) {
            # This exact directory was absent twice before our own launch, and
            # this runner is disposable. No other user's directory is accepted.
            $expectedDataPath = [IO.Path]::GetFullPath((Join-Path $localAppData 'com.eisenbart.counseling'))
            if ($applicationData -ne $expectedDataPath) {
                throw 'Application-data cleanup safety check failed.'
            }
            Remove-Item -LiteralPath $applicationData -Recurse -Force
            Write-Validation 'PASS removed only application data created by this disposable test'
        }
        if ((Test-Path -LiteralPath $installDirectory) -and
            @(Get-ChildItem -LiteralPath $installDirectory -File -Recurse -Force).Count -eq 0) {
            Remove-Item -LiteralPath $installDirectory -Recurse -Force
        }
    } catch {
        Write-Validation "FAIL cleanup: $($_.Exception.Message)"
        if ($null -eq $failure) { $failure = $_ }
    }
}

if ($null -ne $failure) { throw $failure }
Write-Validation 'PASS Windows package installation, native-window launch, and uninstallation checks complete'
