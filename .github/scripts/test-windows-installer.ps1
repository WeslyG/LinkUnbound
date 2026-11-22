$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$version = $env:VERSION
function Invoke-Installer($path, $arguments) {
    $process = Start-Process -FilePath $path -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (!$process.WaitForExit(90000) -or $process.ExitCode -ne 0) {
        throw "Setup failed: $path"
    }
}

$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
$newKey = Join-Path $registry 'LinkUnbound (WeslyG)'
$installed = Join-Path $env:LOCALAPPDATA 'Programs\LinkUnbound (WeslyG)'
$oldInstalled = Join-Path $env:LOCALAPPDATA 'Programs\LinkUnbound'
$classes = 'HKCU:\Software\Classes'
$legacy = Join-Path $env:RUNNER_TEMP 'LinkUnbound_2.1.0_x64-setup.exe'
$url = 'https://github.com/WeslyG/LinkUnbound/releases/download/v2.1.0/LinkUnbound_2.1.0_x64-setup.exe'
Invoke-WebRequest -Uri $url -OutFile $legacy
if ((Get-FileHash $legacy).Hash -ne 'AEA7FF2C17C1AC9CAF4EF81AAE727D040282A3F8DE6AF572B4FE68799F55B67C') {
    throw 'Unexpected legacy installer hash'
}
Invoke-Installer $legacy '/S'
if (!(Test-Path (Join-Path $registry 'LinkUnbound'))) { throw 'Legacy fixture did not install' }
$oldCache = Join-Path $env:LOCALAPPDATA 'LinkUnbound\domain-lists'
New-Item -ItemType Directory -Path $oldCache -Force | Out-Null
Set-Content -LiteralPath (Join-Path $oldCache 'ru-inside.txt') -Value 'migration.example'
$setup = (Resolve-Path "release-assets/LinkUnbound_$($version)_x64-setup.exe").Path
Invoke-Installer $setup '/S /UPDATE'

$entry = Get-ItemProperty -LiteralPath $newKey
if ($entry.DisplayVersion -ne $version -or $entry.Publisher -ne 'WeslyG' -or
    !(Test-Path -LiteralPath (Join-Path $installed 'linkunbound-weslyg-settings.exe')) -or
    !(Test-Path -LiteralPath (Join-Path $installed 'linkunbound-weslyg-shell.exe')) -or
    !(Test-Path -LiteralPath (Join-Path $oldInstalled 'linkunbound-settings.exe')) -or
    !(Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundURL')) -or
    !(Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundWeslyGURL'))) {
    throw 'Installer identities overlap or registration is incomplete'
}
$copied = Join-Path $env:LOCALAPPDATA 'LinkUnboundWeslyG\domain-lists\ru-inside.txt'
if ((Get-Content -Raw -LiteralPath $copied).Trim() -ne 'migration.example') {
    throw 'Legacy domain list cache was not imported'
}
Invoke-Installer $setup '/S /UPDATE'
Invoke-Installer (Join-Path $installed 'uninstall.exe') '/S'
# NSIS re-launches the uninstaller from a temporary directory.
$deadline = [DateTime]::UtcNow.AddSeconds(30)
while ((Test-Path -LiteralPath $newKey) -and [DateTime]::UtcNow -lt $deadline) {
    Start-Sleep -Milliseconds 250
}
if ((Test-Path -LiteralPath $newKey) -or (Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundWeslyGURL')) -or
    !(Test-Path -LiteralPath (Join-Path $registry 'LinkUnbound')) -or
    !(Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundURL'))) {
    throw ('Uninstall verification failed. New package: {0}, new ProgID: {1}, old package: {2}, old ProgID: {3}' -f (Test-Path -LiteralPath $newKey), (Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundWeslyGURL')), (Test-Path -LiteralPath (Join-Path $registry 'LinkUnbound')), (Test-Path -LiteralPath (Join-Path $classes 'LinkUnboundURL')))
}
