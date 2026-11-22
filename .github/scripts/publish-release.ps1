$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repository = $env:GITHUB_REPOSITORY
$version = $env:VERSION
$tag = "v$version"
if ($env:GITHUB_REF -cne "refs/tags/$tag") { throw 'Version does not match the pushed tag' }

$notes = Join-Path $env:RUNNER_TEMP 'release-notes.md'
node .github/scripts/release-notes.cjs $version $notes
$windows = "LinkUnbound_$($version)_x64-setup.exe"
$base = "LinkUnbound_$($version)_macOS"
$required = @($windows, "$($base)_aarch64.dmg", "$($base)_x86_64.dmg")
$optional = @("$($base)_aarch64.app.tar.gz", "$($base)_x86_64.app.tar.gz")
$files = @($required) + @($optional | Where-Object { Test-Path "release-assets/$_" })
$names = @($files | ForEach-Object { $_; "$_.sig" })

# Read the stable channel before publishing so older releases cannot move it backwards.
gh auth setup-git --hostname github.com
git fetch --no-tags origin refs/heads/manifest
$feed = Join-Path $env:RUNNER_TEMP 'linkunbound-feed'
git worktree add --detach $feed FETCH_HEAD
$manifestPath = Join-Path $feed 'release-manifest.json'
$manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json

$PSNativeCommandUseErrorActionPreference = $false
$raw = gh release view $tag --repo $repository --json isDraft 2>$null
$exists = $LASTEXITCODE -eq 0
$PSNativeCommandUseErrorActionPreference = $true
$assetDir = 'release-assets'
if (!$exists -or ($raw | ConvertFrom-Json).isDraft) {
    foreach ($file in $files) {
        node .github/scripts/verify-updater.cjs "$assetDir/$file" $version
    }
    if (!$exists) {
        gh release create $tag --repo $repository --verify-tag --draft --title "LinkUnbound $version" --notes-file $notes
    }
    $paths = @($names | ForEach-Object { "$assetDir/$_" })
    gh release upload $tag @paths --repo $repository --clobber
    $makeLatest = ([version]$version -ge [version]$manifest.latest).ToString().ToLowerInvariant()
    gh release edit $tag --repo $repository --notes-file $notes --draft=false "--latest=$makeLatest"
} else {
    # Keep the published assets when resuming a failed manifest update.
    $published = gh api "repos/$repository/releases/tags/$tag" | ConvertFrom-Json
    $files = @($required) + @($optional | Where-Object { $_ -cin $published.assets.name })
    $names = @($files | ForEach-Object { $_; "$_.sig" })
    $assetDir = Join-Path $env:RUNNER_TEMP 'linkunbound-published'
    New-Item -ItemType Directory -Path $assetDir | Out-Null
    $patterns = @($names | ForEach-Object { '--pattern'; $_ })
    gh release download $tag --repo $repository @patterns --dir $assetDir
    foreach ($file in $files) {
        node .github/scripts/verify-updater.cjs "$assetDir/$file" $version
    }
}

$release = gh api "repos/$repository/releases/tags/$tag" | ConvertFrom-Json
if ($release.draft -or $release.prerelease) { throw 'Expected a published stable release' }
$urls = @{}
foreach ($name in $names) {
    $asset = @($release.assets | Where-Object name -CEQ $name)
    $url = "https://github.com/$repository/releases/download/$tag/$name"
    if ($asset.Count -ne 1 -or $asset[0].browser_download_url -cne $url -or
        $asset[0].size -ne (Get-Item "$assetDir/$name").Length) {
        throw "Published asset does not match: $name"
    }
    $urls[$name] = $url
}

if ([version]$version -lt [version]$manifest.latest) {
    Write-Host "Stable channel already points to $($manifest.latest)"
    return
}

$updaters = @{
    'windows-x86_64' = $windows
    'darwin-aarch64' = "$($base)_aarch64.app.tar.gz"
    'darwin-x86_64' = "$($base)_x86_64.app.tar.gz"
}
$platforms = @{}
foreach ($platform in $updaters.Keys) {
    $file = $updaters[$platform]
    if ($file -cin $files) {
        $platforms[$platform] = @{
            url = $urls[$file]
            signature = (Get-Content -Raw "$assetDir/$file.sig").Trim()
        }
    }
}
$latest = @{
    version = $version
    notes = (Get-Content -Raw $notes).Trim()
    pub_date = ([DateTimeOffset]$release.published_at).UtcDateTime.ToString("yyyy-MM-ddTHH:mm:ss'Z'")
    platforms = $platforms
}
$manifest.latest = $version
$manifest | ConvertTo-Json | Set-Content -Encoding utf8 $manifestPath
$latest | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $feed 'latest.json')
git -C $feed add release-manifest.json latest.json
if (git -C $feed status --porcelain) {
    git -C $feed -c user.name='github-actions[bot]' -c user.email='41898282+github-actions[bot]@users.noreply.github.com' commit -m "Publish LinkUnbound $version update"
    git -C $feed push origin HEAD:refs/heads/manifest
}
