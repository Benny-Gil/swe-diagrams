// Tests of the launcher. A local HTTP server gives fake release assets, so
// the tests do not use the network.
"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const http = require("node:http");
const os = require("node:os");
const path = require("node:path");
const zlib = require("node:zlib");
const { spawn } = require("node:child_process");
const { test, before, after } = require("node:test");

const launcher = require("../lib/launcher.js");

const VERSION = launcher.VERSION;
const BIN = path.join(__dirname, "..", "bin", "swe-diagrams.js");
const LINUX_ASSET = "swe-diagrams-x86_64-unknown-linux-musl.tar.gz";
const WINDOWS_ASSET = "swe-diagrams-x86_64-pc-windows-msvc.zip";

// The fake binary shows its arguments, copies stdin, and exits with 7.
const FAKE = `#!/bin/sh
echo "args: $*"
cat
echo "to stderr" >&2
exit 7
`;

function tarGz(name, data) {
  const header = Buffer.alloc(512);
  header.write(name, 0, "latin1");
  header.write("0000755\0", 100, "latin1");
  header.write("0000000\0", 108, "latin1");
  header.write("0000000\0", 116, "latin1");
  header.write(data.length.toString(8).padStart(11, "0") + "\0", 124, "latin1");
  header.write("00000000000\0", 136, "latin1");
  header.write("0", 156, "latin1");
  header.write("ustar\0" + "00", 257, "latin1");
  header.fill(" ", 148, 156);
  let sum = 0;
  for (const b of header) sum += b;
  header.write(sum.toString(8).padStart(6, "0") + "\0 ", 148, "latin1");
  const pad = Buffer.alloc((512 - (data.length % 512)) % 512);
  return zlib.gzipSync(Buffer.concat([header, data, pad, Buffer.alloc(1024)]));
}

function zip(name, data, deflate) {
  const body = deflate ? zlib.deflateRawSync(data) : data;
  const method = deflate ? 8 : 0;
  const nameBuf = Buffer.from(name);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(method, 8);
  local.writeUInt32LE(body.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(nameBuf.length, 26);
  const central = Buffer.alloc(46);
  central.writeUInt32LE(0x02014b50, 0);
  central.writeUInt16LE(method, 10);
  central.writeUInt32LE(body.length, 20);
  central.writeUInt32LE(data.length, 24);
  central.writeUInt16LE(nameBuf.length, 28);
  central.writeUInt32LE(0, 42);
  const centralAt = local.length + nameBuf.length + body.length;
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(1, 8);
  end.writeUInt16LE(1, 10);
  end.writeUInt32LE(central.length + nameBuf.length, 12);
  end.writeUInt32LE(centralAt, 16);
  return Buffer.concat([local, nameBuf, body, central, nameBuf, end]);
}

function sha256(buf) {
  return crypto.createHash("sha256").update(buf).digest("hex");
}

let server;
let base;
let requests = [];
const files = new Map();

function publish(assets, sums = assets) {
  files.clear();
  const lines = [];
  for (const [name, data] of Object.entries(assets)) files.set(`/v${VERSION}/${name}`, data);
  for (const [name, data] of Object.entries(sums)) lines.push(`${sha256(data)}  ${name}`);
  files.set(`/v${VERSION}/SHA256SUMS`, Buffer.from(lines.join("\n") + "\n"));
}

const temps = [];
function tempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "swe-diagrams-test-"));
  temps.push(dir);
  return dir;
}

function opts(cache, extra = {}) {
  return {
    env: { XDG_CACHE_HOME: cache, SWE_DIAGRAMS_DOWNLOAD_BASE: base },
    platform: "linux",
    arch: "x64",
    home: cache,
    log: () => {},
    ...extra,
  };
}

before(async () => {
  server = http.createServer((req, res) => {
    requests.push(req.url);
    const data = files.get(req.url);
    res.statusCode = data ? 200 : 404;
    res.end(data || "not found");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  base = `http://127.0.0.1:${server.address().port}/`;
});

after(() => {
  server.close();
  for (const dir of temps) fs.rmSync(dir, { recursive: true, force: true });
});

test("each supported platform has a target and an asset name", () => {
  assert.equal(launcher.target("linux", "x64"), "x86_64-unknown-linux-musl");
  assert.equal(launcher.target("linux", "arm64"), "aarch64-unknown-linux-musl");
  assert.equal(launcher.target("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(launcher.target("darwin", "x64"), "x86_64-apple-darwin");
  assert.equal(launcher.assetName(launcher.target("win32", "x64")), WINDOWS_ASSET);
  assert.throws(() => launcher.target("freebsd", "x64"), /cargo install swe-diagrams/);
});

test("the asset names agree with the release workflow", () => {
  const workflow = fs.readFileSync(path.join(__dirname, "..", "..", ".github", "workflows", "release.yml"), "utf8");
  for (const t of ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl", "aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc"]) {
    assert.match(workflow, new RegExp(`target: ${t} `));
  }
});

test("the cache folder follows the platform", () => {
  assert.equal(launcher.cacheDir("1.2.3", {}, "linux", "/home/a"), "/home/a/.cache/swe-diagrams/bin/1.2.3");
  assert.equal(launcher.cacheDir("1.2.3", { XDG_CACHE_HOME: "/x" }, "darwin", "/Users/a"), "/x/swe-diagrams/bin/1.2.3");
  assert.equal(
    launcher.cacheDir("1.2.3", { LOCALAPPDATA: "C:\\Users\\a\\AppData\\Local" }, "win32", "C:\\Users\\a"),
    "C:\\Users\\a\\AppData\\Local\\swe-diagrams\\Cache\\bin\\1.2.3",
  );
});

test("checksumFor reads the sha256sum format", () => {
  const a = "a".repeat(64);
  const b = "B".repeat(64);
  const sums = `${a}  one.tar.gz\r\n${b} *two.zip\n`;
  assert.equal(launcher.checksumFor(sums, "one.tar.gz"), a);
  assert.equal(launcher.checksumFor(sums, "two.zip"), b.toLowerCase());
  assert.throws(() => launcher.checksumFor(sums, "three.zip"), /no line for three.zip/);
});

test("the extractors find the binary in tar.gz and zip archives", () => {
  const data = Buffer.from("binary bytes ".repeat(100));
  assert.deepEqual(launcher.extractTarGz(tarGz("swe-diagrams", data), "swe-diagrams"), data);
  assert.deepEqual(launcher.extractZip(zip("swe-diagrams.exe", data, false), "swe-diagrams.exe"), data);
  assert.deepEqual(launcher.extractZip(zip("swe-diagrams.exe", data, true), "swe-diagrams.exe"), data);
  assert.throws(() => launcher.extractTarGz(tarGz("other", data), "swe-diagrams"), /no file swe-diagrams/);
});

test("the first run downloads the binary and the next run uses the cache", async () => {
  const cache = tempDir();
  publish({ [LINUX_ASSET]: tarGz("swe-diagrams", Buffer.from(FAKE)) });
  requests = [];
  const exe = await launcher.ensureBinary(opts(cache));
  assert.equal(exe, path.join(cache, "swe-diagrams", "bin", VERSION, "swe-diagrams"));
  assert.equal(fs.readFileSync(exe, "utf8"), FAKE);
  assert.deepEqual(requests.sort(), [`/v${VERSION}/${LINUX_ASSET}`, `/v${VERSION}/SHA256SUMS`].sort());
  requests = [];
  assert.equal(await launcher.ensureBinary(opts(cache)), exe);
  assert.deepEqual(requests, []);
});

test("the Windows asset is a zip file", async () => {
  const cache = tempDir();
  publish({ [WINDOWS_ASSET]: zip("swe-diagrams.exe", Buffer.from("MZ fake"), true) });
  const exe = await launcher.ensureBinary(opts(cache, { platform: "win32", dir: cache }));
  assert.equal(path.basename(exe), "swe-diagrams.exe");
  assert.equal(fs.readFileSync(exe, "utf8"), "MZ fake");
});

test("a wrong checksum stops the install", async () => {
  const cache = tempDir();
  const good = tarGz("swe-diagrams", Buffer.from(FAKE));
  publish({ [LINUX_ASSET]: tarGz("swe-diagrams", Buffer.from("evil")) }, { [LINUX_ASSET]: good });
  await assert.rejects(launcher.ensureBinary(opts(cache)), /checksum of .* is wrong/);
  assert.equal(fs.existsSync(path.join(cache, "swe-diagrams")), false);
});

test("a missing asset gives the URL and the HTTP status", async () => {
  const cache = tempDir();
  publish({});
  await assert.rejects(launcher.ensureBinary(opts(cache)), new RegExp(`${LINUX_ASSET}: HTTP 404`));
});

test("the launcher passes the arguments, the streams, and the exit code", { skip: process.platform === "win32" }, async () => {
  const cache = tempDir();
  const env = { ...process.env, XDG_CACHE_HOME: cache, SWE_DIAGRAMS_DOWNLOAD_BASE: base };
  // The launcher picks the asset of the real platform; give it the same fake.
  const real = launcher.assetName(launcher.target(process.platform, process.arch));
  publish({ [real]: tarGz("swe-diagrams", Buffer.from(FAKE)) });

  const run = (input) =>
    new Promise((resolve) => {
      const child = spawn(process.execPath, [BIN, "render", "a b.sd"], { env });
      let stdout = "";
      let stderr = "";
      child.stdout.on("data", (d) => (stdout += d));
      child.stderr.on("data", (d) => (stderr += d));
      child.on("exit", (code) => resolve({ code, stdout, stderr }));
      child.stdin.end(input);
    });

  const first = await run("from stdin\n");
  assert.equal(first.code, 7);
  assert.equal(first.stdout, "args: render a b.sd\nfrom stdin\n");
  assert.match(first.stderr, new RegExp(`^swe-diagrams: downloading v${VERSION} for `));
  assert.match(first.stderr, /to stderr\n$/);

  const second = await run("");
  assert.equal(second.code, 7);
  assert.equal(second.stderr, "to stderr\n");
});

test("a download error gives exit code 1 and a message", async () => {
  const cache = tempDir();
  publish({});
  const env = { ...process.env, XDG_CACHE_HOME: cache, LOCALAPPDATA: cache, SWE_DIAGRAMS_DOWNLOAD_BASE: base };
  const result = await new Promise((resolve) => {
    const child = spawn(process.execPath, [BIN, "--version"], { env });
    let stderr = "";
    child.stderr.on("data", (d) => (stderr += d));
    child.on("exit", (code) => resolve({ code, stderr }));
  });
  assert.equal(result.code, 1);
  assert.match(result.stderr, /swe-diagrams: error: cannot download .*HTTP 404/);
});

test("npm/LICENSE is a copy of the root LICENSE", () => {
  // npm cannot pack files outside npm/, so the package keeps a copy.
  const read = (p) => fs.readFileSync(path.join(__dirname, "..", p), "utf8");
  assert.equal(read("LICENSE"), read("../LICENSE"));
});
