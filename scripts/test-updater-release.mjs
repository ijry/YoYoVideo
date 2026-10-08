import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, writeFile, rename, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { PLATFORMS, validatePlatformSet, collectRelease, publishRelease } from './verify-updater-release.mjs';

const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
async function fixture() {
  await mkdir('.cache', { recursive: true });
  const dir = await mkdtemp('.cache/release-contract-');
  for (const platform of PLATFORMS) {
    const channel = 'stable-' + platform;
    const full = 'YoYoVideo-0.0.1-' + channel + '-full.nupkg';
    const primary = platform === 'windows-x64' ? 'YoYoVideo-' + channel + '-Setup.exe' : platform === 'linux-x64' ? 'YoYoVideo.AppImage' : 'YoYoVideo-' + channel + '-Portable.zip';
    const feed = { Assets: [{ PackageId: 'YoYoVideo', Version: '0.0.1', Type: 'Full', FileName: full, Size: 3, SHA1: 'a'.repeat(40), SHA256: sha('abc') }] };
    const files = {
      [full]: 'abc', [primary]: 'not a native package - structural fixture only',
      ['RELEASES-' + channel]: 'legacy feed',
      ['releases.' + channel + '.json']: JSON.stringify(feed),
      ['assets.' + channel + '.json']: JSON.stringify([{ Type: 'Full', RelativeFileName: full }, { Type: platform === 'windows-x64' ? 'Installer' : 'Portable', RelativeFileName: primary }]),
      ['yoyovideo-update.' + platform + '.json']: JSON.stringify({ schema_version: 1, app_id: 'YoYoVideo', platform, channel, version: '0.0.1', release_tag: 'v0.0.1', feed }),
      ['yoyovideo-update.' + platform + '.json.sig']: 'test-only signature placeholder',
    };
    for (const [name, contents] of Object.entries(files)) await writeFile(join(dir, name), contents);
  }
  return dir;
}
test('requires four unique exact targets', () => {
  assert.doesNotThrow(() => validatePlatformSet(PLATFORMS));
  for (const platforms of [[], ['windows-x64'], [...PLATFORMS, 'linux-x64'], [...PLATFORMS.slice(0, 3), 'unknown']]) assert.throws(() => validatePlatformSet(platforms));
});
test('collects complete assets and hashes without claiming cryptographic verification', async () => {
  const assets = await collectRelease(await fixture(), '0.0.1');
  assert.equal(assets.length, 28);
  assert.ok(assets.every(a => /^[a-f0-9]{64}$/.test(a.sha256)));
});
test('missing signature is not publishable', async () => {
  const dir = await fixture();
  await rename(join(dir, 'yoyovideo-update.windows-x64.json.sig'), join(dir, 'missing.sig'));
  await assert.rejects(collectRelease(dir, '0.0.1'));
});
test('mixed versions and unlisted artifacts are refused', async () => {
  const dir = await fixture();
  await assert.rejects(collectRelease(dir, '0.0.2'));
  await writeFile(join(dir, 'private.key'), 'test-only marker');
  await assert.rejects(collectRelease(dir, '0.0.1'));
});
test('changed package hash is refused', async () => {
  const dir = await fixture();
  await writeFile(join(dir, 'YoYoVideo-0.0.1-stable-linux-x64-full.nupkg'), 'bad');
  await assert.rejects(collectRelease(dir, '0.0.1'));
});
function boundary({ existing = false, failure, changedDownload = false } = {}) {
  const calls = [];
  let checks = 0;
  const gh = async (args) => {
    calls.push(args);
    if (failure && args[0] === 'release' && args[1] === failure) throw new Error('fake gh failure');
    if (args[0] === 'api' && args[1].endsWith('/releases')) return JSON.stringify([existing ? [{ tag_name: 'v0.0.1', draft: false }] : []]);
    if (args[0] === 'api') return JSON.stringify({ object: { type: 'commit', sha: 'a'.repeat(40) } });
    return '';
  };
  const verify = async () => {
    checks++;
    if (failure === 'verify' || (failure === 'download-verify' && checks === 2)) throw new Error('verification failure');
    return [{ name: 'fixture', sha256: changedDownload && checks === 2 ? 'changed' : 'expected', size: 1 }];
  };
  return { calls, gh, verify };
}
const options = { assetsDir: '.cache', version: '0.0.1', repo: 'ijry/YoYoVideo', commit: 'a'.repeat(40), notesFile: 'notes.md' };
test('publishes only after draft upload, download and independent re-verification', async () => {
  const fake = boundary();
  await publishRelease(options, fake);
  assert.deepEqual(fake.calls.filter(c => c[0] === 'release').map(c => c[1]), ['create', 'upload', 'download', 'edit']);
  assert.ok(fake.calls.find(c => c[1] === 'create').includes('--draft'));
  assert.ok(fake.calls.find(c => c[1] === 'edit').includes('--draft=false'));
});
test('verification/upload/download failures and existing releases never publish', async () => {
  for (const settings of [{existing:true},{failure:'verify'},{failure:'upload'},{failure:'download'},{failure:'download-verify'},{changedDownload:true}]) {
    const fake = boundary(settings);
    await assert.rejects(publishRelease(options, fake));
    assert.ok(!fake.calls.some(c => c[0] === 'release' && c[1] === 'edit'));
  }
});

test('a tag moved after upload prevents publication', async () => {
  const fake = boundary();
  const original = fake.gh;
  let tagReads = 0;
  fake.gh = async (args) => {
    const response = await original(args);
    if (args[0] === 'api' && args[1].includes('/git/ref/tags/') && ++tagReads === 2) {
      return JSON.stringify({ object: { type: 'commit', sha: 'b'.repeat(40) } });
    }
    return response;
  };
  await assert.rejects(publishRelease(options, fake), /tag moved/);
  assert.ok(!fake.calls.some(c => c[0] === 'release' && c[1] === 'edit'));
});

test('portable AppImage filename comes from the native asset index', async () => {
  const dir = await fixture();
  const name = 'YoYoVideo-stable-linux-x64.AppImage';
  await rename(join(dir, 'YoYoVideo.AppImage'), join(dir, name));
  const indexPath = join(dir, 'assets.stable-linux-x64.json');
  const index = JSON.parse(await readFile(indexPath, 'utf8'));
  index.find(a => a.Type === 'Portable').RelativeFileName = name;
  await writeFile(indexPath, JSON.stringify(index));
  const files = await collectRelease(dir, '0.0.1');
  assert.ok(files.some(f => f.name === name));
});

test('native feed spelling/defaults match the SDK-normalized signed feed', async () => {
  const dir = await fixture();
  for (const platform of PLATFORMS) {
    const manifestPath = join(dir, 'yoyovideo-update.' + platform + '.json');
    const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
    manifest.feed.Assets[0].NotesMarkdown = '';
    manifest.feed.Assets[0].NotesHtml = '<p>notes</p>';
    await writeFile(manifestPath, JSON.stringify(manifest));
    const feedPath = join(dir, 'releases.stable-' + platform + '.json');
    const feed = JSON.parse(await readFile(feedPath, 'utf8'));
    feed.Assets[0].NotesHTML = '<p>notes</p>';
    await writeFile(feedPath, JSON.stringify(feed));
  }
  await collectRelease(dir, '0.0.1');
  const feedPath = join(dir, 'releases.stable-windows-x64.json');
  const feed = JSON.parse(await readFile(feedPath, 'utf8'));
  feed.Assets[0].NotesHTML = 'tampered';
  await writeFile(feedPath, JSON.stringify(feed));
  await assert.rejects(collectRelease(dir, '0.0.1'));
});
