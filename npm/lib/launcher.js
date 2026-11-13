// Finds the release binary for this platform, downloads it on the first
// run, and runs it with the arguments, the streams, and the exit code of
// this process. The package has no dependencies and no install script.
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const zlib = require("node:zlib");
const { spawn } = require("node:child_process");

const VERSION = require("../package.json").version;
const DEFAULT_BASE = "https://github.com/Benny-Gil/swe-diagrams/releases/download";

// Keep these in step with the matrix in .github/workflows/release.yml.
const TARGETS = {
  "linux-x64": "x86_64-unknown-linux-musl",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function target(platform, arch) {
  const t = TARGETS[`${platform}-${arch}`];
  if (!t) {
    throw new Error(
      `no prebuilt binary for ${platform}-${arch}. Use: cargo install swe-diagrams`,
    );
  }
  return t;
}

function assetName(t) {
  return t.includes("windows") ? `swe-diagrams-${t}.zip` : `swe-diagrams-${t}.tar.gz`;
}

function exeName(platform) {
  return platform === "win32" ? "swe-diagrams.exe" : "swe-diagrams";
}

function cacheDir(version, env, platform, home) {
  let root;
  if (platform === "win32") {
    root = path.win32.join(env.LOCALAPPDATA || path.win32.join(home, "AppData", "Local"), "swe-diagrams", "Cache");
    return path.win32.join(root, "bin", version);
  }
  root = path.posix.join(env.XDG_CACHE_HOME || path.posix.join(home, ".cache"), "swe-diagrams");
  return path.posix.join(root, "bin", version);
}

// The format is the output of sha256sum: "<hex>  <name>" or "<hex> *<name>".
function checksumFor(sums, name) {
  for (const line of sums.split(/\r?\n/)) {
    const m = /^([0-9a-fA-F]{64}) [ *](.+)$/.exec(line.trim());
    if (m && m[2] === name) return m[1].toLowerCase();
  }
  throw new Error(`SHA256SUMS has no line for ${name}`);
}

function extractTarGz(archive, member) {
  const tar = zlib.gunzipSync(archive);
  let at = 0;
  while (at + 512 <= tar.length) {
    const header = tar.subarray(at, at + 512);
    if (header.every((b) => b === 0)) break;
    const field = (start, len) => header.toString("latin1", start, start + len).replace(/\0.*$/s, "");
    const name = field(0, 100);
    const size = parseInt(field(124, 12).trim() || "0", 8);
    const type = field(156, 1);
    const data = at + 512;
    if ((type === "" || type === "0") && path.posix.basename(name) === member) {
      return tar.subarray(data, data + size);
    }
    at = data + Math.ceil(size / 512) * 512;
  }
  throw new Error(`the archive has no file ${member}`);
}

function extractZip(archive, member) {
  // The end record is in the last 64 KiB + 22 bytes, after a comment.
  let end = -1;
  for (let i = archive.length - 22; i >= Math.max(0, archive.length - 65557); i--) {
    if (archive.readUInt32LE(i) === 0x06054b50) {
      end = i;
      break;
    }
  }
  if (end < 0) throw new Error("the zip archive has no end record");
  const count = archive.readUInt16LE(end + 10);
  let at = archive.readUInt32LE(end + 16);
  for (let n = 0; n < count; n++) {
    if (archive.readUInt32LE(at) !== 0x02014b50) throw new Error("the zip archive is damaged");
    const method = archive.readUInt16LE(at + 10);
    const packed = archive.readUInt32LE(at + 20);
    const nameLen = archive.readUInt16LE(at + 28);
    const extraLen = archive.readUInt16LE(at + 30);
    const commentLen = archive.readUInt16LE(at + 32);
    const local = archive.readUInt32LE(at + 42);
    const name = archive.toString("utf8", at + 46, at + 46 + nameLen);
    if (path.posix.basename(name.replace(/\\/g, "/")) === member) {
      const data = local + 30 + archive.readUInt16LE(local + 26) + archive.readUInt16LE(local + 28);
      const bytes = archive.subarray(data, data + packed);
      if (method === 0) return bytes;
      if (method === 8) return zlib.inflateRawSync(bytes);
      throw new Error(`the zip archive uses method ${method}, which is not supported`);
    }
    at += 46 + nameLen + extraLen + commentLen;
  }
  throw new Error(`the archive has no file ${member}`);
}

async function download(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`cannot download ${url}: HTTP ${res.status}`);
  return Buffer.from(await res.arrayBuffer());
}

// Returns the path of the binary, and downloads it when the cache has no copy.
async function ensureBinary(opts = {}) {
  const version = opts.version || VERSION;
  const env = opts.env || process.env;
  const platform = opts.platform || process.platform;
  const arch = opts.arch || process.arch;
  const log = opts.log || ((msg) => process.stderr.write(`swe-diagrams: ${msg}\n`));

  const t = target(platform, arch);
  const dir = opts.dir || cacheDir(version, env, platform, opts.home || os.homedir());
  const exe = path.join(dir, exeName(platform));
  if (fs.existsSync(exe)) return exe;

  const base = (env.SWE_DIAGRAMS_DOWNLOAD_BASE || DEFAULT_BASE).replace(/\/+$/, "");
  const asset = assetName(t);
  log(`downloading v${version} for ${t}`);
  const [archive, sums] = await Promise.all([
    download(`${base}/v${version}/${asset}`),
    download(`${base}/v${version}/SHA256SUMS`),
  ]);
  const want = checksumFor(sums.toString("utf8"), asset);
  const got = crypto.createHash("sha256").update(archive).digest("hex");
  if (got !== want) throw new Error(`the checksum of ${asset} is wrong`);

  const bytes = asset.endsWith(".zip") ? extractZip(archive, exeName(platform)) : extractTarGz(archive, exeName(platform));
  fs.mkdirSync(dir, { recursive: true });
  // A rename is atomic, so a parallel run never sees half a file.
  const tmp = `${exe}.${process.pid}.tmp`;
  fs.writeFileSync(tmp, bytes, { mode: 0o755 });
  fs.renameSync(tmp, exe);
  return exe;
}

function fail(err) {
  process.stderr.write(`swe-diagrams: error: ${err.message}\n`);
  process.exit(1);
}

async function main(args) {
  let exe;
  try {
    exe = await ensureBinary();
  } catch (err) {
    fail(err);
  }
  const child = spawn(exe, args, { stdio: "inherit" });
  // Give a signal to the binary, so that "kill <pid>" stops both processes.
  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
    process.on(sig, () => child.kill(sig));
  }
  child.on("error", fail);
  child.on("exit", (code, signal) => {
    if (signal) {
      process.removeAllListeners(signal);
      process.kill(process.pid, signal);
    } else {
      process.exit(code);
    }
  });
}

module.exports = {
  VERSION,
  target,
  assetName,
  cacheDir,
  checksumFor,
  extractTarGz,
  extractZip,
  ensureBinary,
  main,
};
