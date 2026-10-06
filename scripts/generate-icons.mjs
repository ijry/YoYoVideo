#!/usr/bin/env node
// Renders the app mark to the raster formats the platforms need.
//
// The mark is geometry, not a bitmap: a rounded square with a play triangle, in
// the same sky-to-violet gradient as docs-site/docs/public/logo.svg. Drawing it
// here rather than checking in opaque binaries means the icon can be regenerated
// at any size, and a change to the shape is a reviewable diff instead of a new
// blob.
//
// PNG is encoded by hand (zlib + CRC32, both in Node) and the .ico is assembled
// from those PNGs. No image tooling, so this runs anywhere `node` does.

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, "..");

// Geometry, in the same 64x64 space as the SVG.
const VIEW = 64;
const CORNER_RADIUS = 15;
const TRIANGLE = [
  [25.5, 19.5],
  [46, 32],
  [25.5, 44.5],
];
const GRADIENT_FROM = [0x38, 0xbd, 0xf8]; // sky-400
const GRADIENT_TO = [0xa7, 0x8b, 0xfa]; // violet-400
const TRIANGLE_COLOR = [0x05, 0x07, 0x0b]; // the app's near-black

/** Samples per axis. 4 means 16 samples per pixel. */
const SUPERSAMPLE = 4;

function insideRoundedSquare(px, py, size, radius) {
  const r = radius;
  // Straight edges first.
  if (px < 0 || py < 0 || px > size || py > size) return false;

  const nearLeft = px < r;
  const nearRight = px > size - r;
  const nearTop = py < r;
  const nearBottom = py > size - r;
  if (!((nearLeft || nearRight) && (nearTop || nearBottom))) return true;

  const cx = nearLeft ? r : size - r;
  const cy = nearTop ? r : size - r;
  const dx = px - cx;
  const dy = py - cy;
  return dx * dx + dy * dy <= r * r;
}

function insideTriangle(px, py, points) {
  const [[ax, ay], [bx, by], [cx, cy]] = points;
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by);
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy);
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay);
  const hasNegative = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPositive = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNegative && hasPositive);
}

function gradientColor(t) {
  const clamped = Math.min(1, Math.max(0, t));
  return [
    Math.round(GRADIENT_FROM[0] + (GRADIENT_TO[0] - GRADIENT_FROM[0]) * clamped),
    Math.round(GRADIENT_FROM[1] + (GRADIENT_TO[1] - GRADIENT_FROM[1]) * clamped),
    Math.round(GRADIENT_FROM[2] + (GRADIENT_TO[2] - GRADIENT_FROM[2]) * clamped),
  ];
}

/** Renders one size into a straight RGBA byte array. */
function render(size) {
  const rgba = Buffer.alloc(size * size * 4);
  const scale = VIEW / size;
  const samples = SUPERSAMPLE * SUPERSAMPLE;

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let rSum = 0;
      let gSum = 0;
      let bSum = 0;
      let aSum = 0;

      for (let sy = 0; sy < SUPERSAMPLE; sy += 1) {
        for (let sx = 0; sx < SUPERSAMPLE; sx += 1) {
          // Sample at sub-pixel centres, in the 64x64 design space.
          const px = (x + (sx + 0.5) / SUPERSAMPLE) * scale;
          const py = (y + (sy + 0.5) / SUPERSAMPLE) * scale;

          if (!insideRoundedSquare(px, py, VIEW, CORNER_RADIUS)) continue;

          let color;
          if (insideTriangle(px, py, TRIANGLE)) {
            color = TRIANGLE_COLOR;
          } else {
            // Linear gradient along the diagonal, matching the SVG's
            // x1=0,y1=0 -> x2=1,y2=1.
            color = gradientColor((px + py) / (2 * VIEW));
          }
          rSum += color[0];
          gSum += color[1];
          bSum += color[2];
          aSum += 255;
        }
      }

      const offset = (y * size + x) * 4;
      if (aSum === 0) continue;
      const covered = aSum / 255;
      // Colours are averaged over covered samples only, so edges do not darken
      // toward transparent black.
      rgba[offset] = Math.round(rSum / covered);
      rgba[offset + 1] = Math.round(gSum / covered);
      rgba[offset + 2] = Math.round(bSum / covered);
      rgba[offset + 3] = Math.round(aSum / samples);
    }
  }
  return rgba;
}

// --- PNG -------------------------------------------------------------------

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) {
    c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const typeAndData = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typeAndData));
  return Buffer.concat([length, typeAndData, crc]);
}

function encodePng(size, rgba) {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type: RGBA
  ihdr[10] = 0; // deflate
  ihdr[11] = 0; // adaptive filtering
  ihdr[12] = 0; // no interlace

  // Each scanline is prefixed with its filter type; 0 means "none", which costs
  // a little size and keeps this readable.
  const stride = size * 4;
  const raw = Buffer.alloc((stride + 1) * size);
  for (let y = 0; y < size; y += 1) {
    raw[y * (stride + 1)] = 0;
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  return Buffer.concat([
    signature,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// --- ICO -------------------------------------------------------------------

function encodeIco(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // 1 = icon
  header.writeUInt16LE(entries.length, 4);

  const directory = Buffer.alloc(16 * entries.length);
  let offset = header.length + directory.length;

  entries.forEach((entry, index) => {
    const at = index * 16;
    // A stored dimension of 0 means 256, which is the only size that does not fit
    // in a byte.
    directory[at] = entry.size >= 256 ? 0 : entry.size;
    directory[at + 1] = entry.size >= 256 ? 0 : entry.size;
    directory[at + 2] = 0; // palette size
    directory[at + 3] = 0; // reserved
    directory.writeUInt16LE(1, at + 4); // colour planes
    directory.writeUInt16LE(32, at + 6); // bits per pixel
    directory.writeUInt32LE(entry.png.length, at + 8);
    directory.writeUInt32LE(offset, at + 12);
    offset += entry.png.length;
  });

  return Buffer.concat([header, directory, ...entries.map((entry) => entry.png)]);
}

// --- Output -----------------------------------------------------------------

const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];
const PNG_SIZES = [64, 128, 256, 512, 1024];

const pngDir = join(repoRoot, "apps/yoyovideo-desktop/assets/icons");
mkdirSync(pngDir, { recursive: true });

const cache = new Map();
function png(size) {
  if (!cache.has(size)) {
    cache.set(size, encodePng(size, render(size)));
  }
  return cache.get(size);
}

// A PNG beside the executable, for the Linux .desktop entry and for anything
// else that wants one raster.
writeFileSync(join(pngDir, "yoyovideo-256.png"), png(256));
writeFileSync(join(pngDir, "yoyovideo-512.png"), png(512));

// Windows: a single .ico carrying every size Explorer picks from.
const ico = encodeIco(ICO_SIZES.map((size) => ({ size, png: png(size) })));
writeFileSync(join(pngDir, "yoyovideo.ico"), ico);

// Slint loads @image-url at build time, and its SVG support is not guaranteed for
// every backend, so the window icon is the plain PNG.
writeFileSync(join(repoRoot, "apps/yoyovideo-desktop/ui/icons/app-icon.png"), png(128));

console.log(`wrote ${ICO_SIZES.length}-size ico (${ico.length} bytes)`);
console.log(`wrote PNGs: ${[...PNG_SIZES].join(", ")} available, 256 and 512 written`);