import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

const source = "WeslyG/LinkUnbound";
const catalog = "microsoft/winget-pkgs";
const fork = "WeslyG/winget-pkgs";
const packageId = "WeslyG.LinkUnbound";
const packagePath = "manifests/w/WeslyG/LinkUnbound";
const manifestNames = [packageId + ".yaml", packageId + ".installer.yaml", packageId + ".locale.en-US.yaml"];
const stable = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const dryRun = process.argv[2] === "--dry-run";
const output = process.argv[3];

async function api(path, { token = process.env.GH_TOKEN, method = "GET", body, missing = false } = {}) {
  const response = await fetch("https://api.github.com/" + path, {
    method,
    headers: {
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      ...(token ? { Authorization: "Bearer " + token } : {}),
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body && JSON.stringify(body),
    signal: AbortSignal.timeout(25_000),
  });
  if (missing && response.status === 404) return undefined;
  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(method + " " + path + ": " + response.status + " " + (error.message ?? ""));
  }
  return response.json();
}

function order(a, b) {
  const left = a.split(".").map(BigInt);
  const right = b.split(".").map(BigInt);
  for (let i = 0; i < left.length; i += 1) {
    if (left[i] !== right[i]) return left[i] > right[i] ? 1 : -1;
  }
  return 0;
}

function field(text, name, value) {
  const pattern = new RegExp("^([ \\t]*(?:- )?" + name + ":)[ \\t]*[^\\n]+$", "gm");
  if ([...text.matchAll(pattern)].length !== 1) {
    throw new Error("Expected exactly one " + name + " field in the x64 WinGet manifest");
  }
  return text.replace(pattern, (_, key) => key + " " + value);
}

const published = await api("repos/" + source + "/releases/latest");
const version = published.tag_name?.slice(1);
if (published.prerelease || published.draft || !stable.test(version) || published.tag_name !== "v" + version) {
  throw new Error("Latest release is not a stable vX.Y.Z release");
}
const filename = "LinkUnbound_" + version + "_x64-setup.exe";
const url = "https://github.com/" + source + "/releases/download/" + published.tag_name + "/" + filename;
const assets = published.assets.filter((asset) => asset.name === filename && asset.state === "uploaded");
if (assets.length !== 1 || assets[0].browser_download_url !== url || !assets[0].size) {
  throw new Error("Release is missing the unique Windows x64 installer");
}

const branch = "winget/linkunbound-" + version;
const prQuery = new URLSearchParams({ state: "all", head: "WeslyG:" + branch, base: "master" });
const pulls = await api("repos/" + catalog + "/pulls?" + prQuery);
if (!dryRun && pulls.length) {
  console.log("A PR already exists for " + version + ": " + pulls[0].html_url);
  process.exit(0);
}

const versions = await api("repos/" + catalog + "/contents/" + packagePath, { missing: true });
const baseline = versions?.filter((item) => item.type === "dir" && stable.test(item.name))
  .map((item) => item.name).sort(order).at(-1);
if (!dryRun && !baseline) {
  throw new Error("The first WinGet package must be merged before publishing automatic updates");
}
if (!dryRun && order(version, baseline) <= 0) {
  console.log("WinGet already has version " + baseline);
  process.exit(0);
}

const download = await fetch(url, { signal: AbortSignal.timeout(120_000) });
if (!download.ok) throw new Error("Windows installer download failed: " + download.status);
const bytes = Buffer.from(await download.arrayBuffer());
const sha = createHash("sha256").update(bytes).digest("hex");
if (bytes.length !== assets[0].size || (assets[0].digest && assets[0].digest !== "sha256:" + sha)) {
  throw new Error("Downloaded installer does not match GitHub release metadata");
}

const baseRepository = baseline ? catalog : fork;
const baseVersion = baseline ?? "2.1.1";
const ref = baseline ? "" : "?ref=winget%2Flinkunbound-2.1.1";
const tree = await Promise.all(manifestNames.map(async (name) => {
  const old = await api("repos/" + baseRepository + "/contents/" + packagePath + "/" + baseVersion + "/" + name + ref);
  let text = Buffer.from(old.content, "base64").toString("utf8").replace(/\r\n/g, "\n");
  if (!text.includes("PackageIdentifier: " + packageId + "\n")) throw new Error("Incorrect package identifier");
  if (dryRun) {
    text = field(text, "PackageVersion", "0.0.0");
    if (name.endsWith(".installer.yaml")) text = field(text, "InstallerSha256", "0".repeat(64));
  }
  text = field(text, "PackageVersion", version);
  if (name.endsWith(".installer.yaml")) {
    text = field(text, "InstallerUrl", url);
    text = field(text, "InstallerSha256", sha.toUpperCase());
  }
  if (name.endsWith(".locale.en-US.yaml")) {
    text = field(text, "ReleaseNotesUrl", "https://github.com/" + source + "/releases/tag/v" + version);
  }
  return { path: packagePath + "/" + version + "/" + name, type: "blob", mode: "100644", content: text };
}));

if (dryRun) {
  if (!output) throw new Error("Pass a directory for the dry-run manifests");
  await mkdir(output, { recursive: true });
  for (const [i, name] of manifestNames.entries()) {
    await writeFile(join(output, name), tree[i].content);
  }
  console.log("Rendered " + packageId + " " + version + " from a stale fixture. SHA-256 " + sha);
} else {
  const token = process.env.WINGET_TOKEN;
  if (!token) throw new Error("Set the WINGET_TOKEN Actions secret to enable PR submission");

  const owner = await api("user", { token });
  const repository = await api("repos/" + fork, { token });
  if (owner.login.toLowerCase() !== "weslyg" || repository.parent?.full_name !== catalog) {
    throw new Error("WINGET_TOKEN must belong to the owner of the existing WinGet fork");
  }
  const head = await api("repos/" + catalog + "/git/ref/heads/master");
  const parent = await api("repos/" + catalog + "/git/commits/" + head.object.sha);
  const createdTree = await api("repos/" + fork + "/git/trees", {
    token, method: "POST", body: { base_tree: parent.tree.sha, tree },
  });
  const title = "Update: " + packageId + " to " + version;
  const existing = await api("repos/" + fork + "/git/ref/heads/" + branch, { token, missing: true });
  if (existing) {
    const commit = await api("repos/" + fork + "/git/commits/" + existing.object.sha, { token });
    if (commit.tree.sha !== createdTree.sha) throw new Error("Existing submission branch has a different tree");
  } else {
    const commit = await api("repos/" + fork + "/git/commits", {
      token, method: "POST", body: { message: title, parents: [head.object.sha], tree: createdTree.sha },
    });
    await api("repos/" + fork + "/git/refs", {
      token, method: "POST", body: { ref: "refs/heads/" + branch, sha: commit.sha },
    });
  }
  const pr = await api("repos/" + catalog + "/pulls", {
    token, method: "POST",
    body: {
      title, head: "WeslyG:" + branch, base: "master", maintainer_can_modify: true,
      body: "Updates the manifest from the [published stable release](" +
        published.html_url + "). The Windows x64 installer was downloaded and its SHA-256 verified.",
    },
  });
  console.log(pr.html_url);
}
