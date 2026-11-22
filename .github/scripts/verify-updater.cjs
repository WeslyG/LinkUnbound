const { readFileSync } = require("node:fs");
const { createHash, createPublicKey, verify } = require("node:crypto");
const config = require("../../app/src-tauri/tauri.conf.json");
const [installer, version] = process.argv.slice(2);

function decode(encoded) {
  return Buffer.from(encoded, "base64").toString("utf8").trim().split(/\r?\n/);
}

const key = Buffer.from(decode(config.plugins.updater.pubkey)[1], "base64");
const [, encodedSignature, comment, encodedGlobalSignature] = decode(
  readFileSync(installer + ".sig", "utf8"),
);
const signature = Buffer.from(encodedSignature, "base64");
const trustedComment = comment.replace(/^trusted comment: /, "");
const publicKey = createPublicKey({
  key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), key.subarray(10)]),
  format: "der",
  type: "spki",
});
const installerBytes = readFileSync(installer);
const algorithm = signature.subarray(0, 2).toString();
const message = algorithm === "ED"
  ? createHash("blake2b512").update(installerBytes).digest()
  : installerBytes;
const artifactSignature = signature.subarray(10);

if (
  !["ED", "Ed"].includes(algorithm) ||
  !key.subarray(2, 10).equals(signature.subarray(2, 10)) ||
  !verify(null, message, publicKey, artifactSignature) ||
  !verify(
    null,
    Buffer.concat([artifactSignature, Buffer.from(trustedComment)]),
    publicKey,
    Buffer.from(encodedGlobalSignature, "base64"),
  ) ||
  !trustedComment.split("\t").includes("version:" + version)
) {
  throw new Error("Installer signature does not match the updater key and version");
}

console.log("Verified updater signature for " + version);
