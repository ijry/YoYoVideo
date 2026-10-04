#!/usr/bin/env node
// Builds the GitHub Release body: the tag commit's changelog, followed by the
// assets actually produced by this run.
//
// The asset list is generated rather than hand-written so the page can never
// advertise a download that a failed or skipped build job did not produce.

import { createHash } from "node:crypto";
import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      throw new Error(`Unexpected argument: ${arg}`);
    }
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for ${arg}`);
    }
    options[arg.slice(2)] = value;
    index += 1;
  }
  for (const required of ["assets-dir", "tag", "repo", "output", "notes-file"]) {
    if (!options[required]) {
      throw new Error(`Missing required argument: --${required}`);
    }
  }
  return options;
}

async function collectAssets(root) {
  const entries = await readdir(root, { withFileTypes: true });
  const assets = [];
  for (const entry of entries) {
    const full = join(root, entry.name);
    if (entry.isDirectory()) {
      // A matrix run downloads into release-assets/<platform>/<file>, but the
      // release page should link to bare asset names, so keep only the file name.
      assets.push(...(await collectAssets(full)));
    } else if (entry.isFile()) {
      assets.push({ full, name: entry.name });
    }
  }
  return assets;
}

function formatSize(bytes) {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const rendered = value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1);
  return `${rendered} ${units[unit]}`;
}

const options = parseArgs(process.argv.slice(2));
const notes = (await readFile(options["notes-file"], "utf8")).trim();
const downloadsUrl = `${options.repo}/releases/tag/${options.tag}`;

const assets = (await collectAssets(options["assets-dir"])).sort((left, right) =>
  left.name.localeCompare(right.name),
);
if (assets.length === 0) {
  throw new Error(`No release assets found under ${options["assets-dir"]}`);
}

// Flattening to bare names only works while platform asset names are unique.
// They embed the platform, so a collision means a packaging bug worth failing on
// rather than a release page with two links to the same file.
const duplicates = assets
  .map((asset) => asset.name)
  .filter((name, index, names) => names.indexOf(name) !== index);
if (duplicates.length > 0) {
  throw new Error(`Duplicate release asset names: ${[...new Set(duplicates)].join(", ")}`);
}

const rows = [];
const checksums = [];
for (const asset of assets) {
  const bytes = await readFile(asset.full);
  const size = formatSize(bytes.length);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  // GitHub's release download URL is /download/<tag>/<asset path>.
  rows.push(`| [\`${asset.name}\`](${downloadsUrl}/download/${asset.name}) | ${size} |`);
  checksums.push(`${sha256}  ${asset.name}`);
}

const body = [
  notes,
  "",
  "## 下载 / Downloads",
  "",
  `本版本由 [\`${options.tag}\`](${downloadsUrl}) 自动构建发布。`,
  "",
  "| 文件 | 大小 |",
  "| --- | ---: |",
  ...rows,
  "",
  "### 校验和 / SHA-256",
  "",
  "```text",
  ...checksums,
  "```",
  "",
  "### 许可 / License",
  "",
  "YoYoVideo 以 GPL-3.0-or-later 发布，并捆绑 GPL-2.0-or-later 的 mpv / FFmpeg 运行时。",
  "每个发行包内的 `LICENSES/` 目录记录了运行时来源与许可证要点。",
  "",
].join("\n");

await writeFile(options.output, body, "utf8");
console.log(`Wrote ${options.output} with ${assets.length} asset(s)`);
