$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repository = $env:GITHUB_REPOSITORY
$version = $env:VERSION
$tag = "v$version"
if ($env:GITHUB_REF -cne "refs/tags/$tag") { throw 'Version does not match the pushed tag' }

$installerName = "LinkUnbound_$($version)_x64-setup.exe"
$installer = (Resolve-Path "release-assets/$installerName").Path
$signature = "$installer.sig"

# Read the stable channel before publishing. A maintenance release must never move it backwards.
gh auth setup-git --hostname github.com
git fetch --no-tags origin refs/heads/manifest
$feed = Join-Path $env:RUNNER_TEMP 'linkunbound-feed'
git worktree add --detach $feed FETCH_HEAD
$manifestPath = Join-Path $feed 'release-manifest.json'
$manifest = Get-Content -Raw $manifestPath | ConvertFrom-Json

$jsonFields = 'isDraft,publishedAt,assets'
# Release lookup can fail normally when the tag has not been published yet.
$PSNativeCommandUseErrorActionPreference = $false
$raw = gh release view $tag --repo $repository --json $jsonFields 2>$null
$exists = $LASTEXITCODE -eq 0
$PSNativeCommandUseErrorActionPreference = $true
if (!$exists) {
    gh release create $tag --repo $repository --verify-tag --draft --title "LinkUnbound $version" --generate-notes
}
if (!$exists -or ($raw | ConvertFrom-Json).isDraft) {
    node .github/scripts/verify-updater.cjs $installer $version
    gh release upload $tag $installer $signature --repo $repository --clobber
    gh release edit $tag --repo $repository --draft=false "--latest=$(([version]$version -ge [version]$manifest.latest).ToString().ToLowerInvariant())"
} else {
    # A rerun after publication reuses the signed assets already visible on GitHub.
    $published = Join-Path $env:RUNNER_TEMP 'linkunbound-published'
    New-Item -ItemType Directory -Path $published | Out-Null
    gh release download $tag --repo $repository --pattern $installerName --pattern "$installerName.sig" --dir $published
    $installer = Join-Path $published $installerName
    $signature = "$installer.sig"
    node .github/scripts/verify-updater.cjs $installer $version
}

$release = gh api "repos/$repository/releases/tags/$tag" | ConvertFrom-Json
$asset = @($release.assets | Where-Object name -CEQ $installerName)
$signatureAsset = @($release.assets | Where-Object name -CEQ "$installerName.sig")
$url = "https://github.com/$repository/releases/download/$tag/$installerName"
if ($release.draft -or $asset.Count -ne 1 -or $asset[0].browser_download_url -cne $url -or
    $asset[0].size -ne (Get-Item $installer).Length -or
    $signatureAsset.Count -ne 1 -or $signatureAsset[0].size -ne (Get-Item $signature).Length) {
    throw 'Published release does not contain the expected installer and signature'
}

if ([version]$version -lt [version]$manifest.latest) {
    Write-Host "Stable channel already points to $($manifest.latest)"
    return
}

$latest = @{
    version = $version
    notes = "LinkUnbound $version"
    pub_date = ([DateTimeOffset]$release.published_at).UtcDateTime.ToString("yyyy-MM-ddTHH:mm:ss'Z'")
    platforms = @{ 'windows-x86_64' = @{
        url = $url
        signature = (Get-Content -Raw $signature).Trim()
    } }
}
$manifest.latest = $version
$manifest | ConvertTo-Json | Set-Content -Encoding utf8 $manifestPath
$latest | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $feed 'latest.json')
git -C $feed add release-manifest.json latest.json
if (git -C $feed status --porcelain) {
    git -C $feed -c user.name='github-actions[bot]' -c user.email='41898282+github-actions[bot]@users.noreply.github.com' commit -m "Publish LinkUnbound $version update"
    git -C $feed push origin HEAD:refs/heads/manifest
}
