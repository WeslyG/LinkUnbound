import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";

const [caskPath] = process.argv.slice(2);
if (!caskPath) throw new Error("Pass the cask file path");

const repository = "WeslyG/LinkUnbound";
const releaseApi = await fetch("https://api.github.com/repos/" + repository + "/releases/latest", {
  headers: {
    Accept: "application/vnd.github+json",
    ...(process.env.GH_TOKEN ? { Authorization: "Bearer " + process.env.GH_TOKEN } : {}),
  },
  signal: AbortSignal.timeout(30_000),
});
if (!releaseApi.ok) throw new Error("Could not read latest release: " + releaseApi.status);
const release = await releaseApi.json();

const stableVersion = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const version = release.tag_name?.slice(1);
if (release.draft || release.prerelease || !release.tag_name?.startsWith("v") || !stableVersion.test(version)) {
  throw new Error("Latest release does not have a stable version tag");
}

const cask = await readFile(caskPath, "utf8");
const versionLine = /^  version "([^"]+)"$/m;
const shaLines = /^  sha256 arm: +"[a-f0-9]{64}",\r?\n +intel: +"[a-f0-9]{64}"$/m;
const current = cask.match(versionLine)?.[1];
if (!stableVersion.test(current) || !shaLines.test(cask)) {
  throw new Error("Unexpected version or checksum fields in the LinkUnbound cask");
}

function compareVersions(left, right) {
  const a = left.split(".").map(BigInt);
  const b = right.split(".").map(BigInt);
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) return a[i] > b[i] ? 1 : -1;
  }
  return 0;
}

if (compareVersions(version, current) < 0) {
  console.log("Cask already points to a newer release: " + current);
  process.exit(0);
}

async function checksum(arch) {
  const name = "LinkUnbound_" + version + "_macOS_" + arch + ".dmg";
  const url = "https://github.com/" + repository + "/releases/download/" + release.tag_name + "/" + name;
  const matches = release.assets.filter((asset) => asset.name === name && asset.state === "uploaded");
  if (matches.length !== 1 || matches[0].browser_download_url !== url || !matches[0].size) {
    throw new Error("Missing or unexpected release asset: " + name);
  }
  const asset = matches[0];
  const download = await fetch(url, { signal: AbortSignal.timeout(120_000) });
  if (!download.ok) throw new Error("Download failed for " + name + ": " + download.status);
  const bytes = Buffer.from(await download.arrayBuffer());
  const hash = createHash("sha256").update(bytes).digest("hex");
  if (bytes.length !== asset.size || (asset.digest && asset.digest !== "sha256:" + hash)) {
    throw new Error("Release download does not match GitHub metadata: " + name);
  }
  return hash;
}

const [arm, intel] = await Promise.all([checksum("aarch64"), checksum("x86_64")]);
const updated = cask.replace(versionLine, '  version "' + version + '"')
  .replace(shaLines, '  sha256 arm:   "' + arm + '",\n         intel: "' + intel + '"');

if (cask === updated) {
  console.log("Cask already matches " + release.tag_name + " and both downloaded DMGs");
} else if (current === version) {
  throw new Error("Published checksums changed without a version bump: " + release.tag_name);
} else {
  await writeFile(caskPath, updated);
  console.log("Cask updated to " + release.tag_name + " with both downloaded DMG checksums");
}
