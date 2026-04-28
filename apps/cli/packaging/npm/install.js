#!/usr/bin/env node
/**
 * tok0 npm wrapper — downloads the right prebuilt binary from the
 * matching GitHub release and verifies its SHA-256.
 *
 * Runs on `npm install`. Fetches the release whose version matches
 * the package.json version, so tagging a new GitHub release and
 * bumping the npm package version stay in lockstep.
 */

const fs = require("fs");
const os = require("os");
const path = require("path");
const https = require("https");
const crypto = require("crypto");

const PKG = require("./package.json");
const VERSION = PKG.version;
const REPO = "prxm-labs/tok0";
const BIN_DIR = path.join(__dirname, "bin");

function detectTarget() {
  const { platform, arch } = process;
  if (platform === "darwin" && arch === "arm64") return "aarch64-apple-darwin";
  if (platform === "darwin" && arch === "x64")   return "x86_64-apple-darwin";
  if (platform === "linux"  && arch === "x64")   return "x86_64-unknown-linux-gnu";
  if (platform === "win32"  && arch === "x64")   return "x86_64-pc-windows-msvc";
  throw new Error(`Unsupported platform: ${platform}/${arch}`);
}

function suffix() {
  return process.platform === "win32" ? ".exe" : "";
}

function download(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        // GitHub releases redirect via 302
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return download(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function main() {
  const target = detectTarget();
  const ext = suffix();
  const name = `tok0-${target}${ext}`;
  const base = `https://github.com/${REPO}/releases/download/v${VERSION}`;
  const binUrl = `${base}/${name}`;
  const shaUrl = `${binUrl}.sha256`;

  console.log(`tok0: downloading ${name} for v${VERSION}...`);

  const [binary, shaText] = await Promise.all([
    download(binUrl),
    download(shaUrl),
  ]);

  const expected = shaText.toString("utf8").split(/\s+/)[0].trim();
  const actual = crypto.createHash("sha256").update(binary).digest("hex");
  if (actual !== expected) {
    throw new Error(
      `SHA-256 mismatch for ${name}:\n  expected ${expected}\n  actual   ${actual}`
    );
  }

  fs.mkdirSync(BIN_DIR, { recursive: true });
  const outPath = path.join(BIN_DIR, `tok0${ext}`);
  fs.writeFileSync(outPath, binary, { mode: 0o755 });

  // Also write the tiny Node shim that npm's `bin` entry points at.
  const shim = `#!/usr/bin/env node
const { spawn } = require("child_process");
const path = require("path");
const bin = path.join(__dirname, "tok0${ext}");
const child = spawn(bin, process.argv.slice(2), { stdio: "inherit" });
child.on("exit", (code) => process.exit(code ?? 1));
`;
  fs.writeFileSync(path.join(BIN_DIR, "tok0.js"), shim, { mode: 0o755 });

  console.log(`tok0: installed to ${outPath}`);
}

main().catch((err) => {
  console.error("tok0: install failed:", err.message);
  process.exit(1);
});
