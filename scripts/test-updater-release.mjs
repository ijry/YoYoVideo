import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, writeFile, rename, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';
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


// Run the actual release workflow's tag-resolution script against local Git repos.
// actions/checkout may bind the public tag ref to a peeled commit even at depth 0.
async function releaseTagFixture({ annotated = true, version = '0.0.1', shadowTag = false } = {}) {
  await mkdir('.cache', { recursive: true });
  const root = resolve(await mkdtemp('.cache/release-tag-contract-'));
  const seed = join(root, 'seed'), remote = join(root, 'origin.git'), work = join(root, 'work');
  await mkdir(seed); await mkdir(work);
  const config = join(root, 'gitconfig');
  await writeFile(config, '[core]\n  autocrlf = false\n');
  const env = { ...process.env, GIT_CONFIG_NOSYSTEM: '1', GIT_CONFIG_GLOBAL: config };
  const git = (cwd, ...args) => {
    const result = spawnSync('git', args, { cwd, env, encoding: 'utf8', windowsHide: true, timeout: 30000 });
    if (result.error) throw result.error;
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  git(seed, 'init', '--initial-branch=main');
  await writeFile(join(seed, 'Cargo.toml'), '[workspace.package]\nversion = "' + version + '"\n');
  git(seed, 'add', 'Cargo.toml');
  git(seed, '-c', 'user.name=Release fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'fixture source');
  if (annotated) git(seed, '-c', 'user.name=Release fixture', '-c', 'user.email=fixture@example.invalid', 'tag', '-a', 'v0.0.1', '-m', 'annotated release fixture notes');
  else git(seed, 'tag', 'v0.0.1');
  const commit = git(seed, 'rev-parse', 'HEAD');
  git(root, 'init', '--bare', remote);
  git(seed, 'remote', 'add', 'origin', pathToFileURL(remote).href);
  git(seed, 'push', 'origin', 'main', 'refs/tags/v0.0.1');
  git(work, 'init');
  git(work, 'remote', 'add', 'origin', pathToFileURL(remote).href);
  git(work, 'fetch', '--no-tags', '--depth=1', 'origin', shadowTag ? '+' + commit + ':refs/tags/v0.0.1' : commit);
  const yaml = await readFile(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  const run = yaml.match(/      - name: Resolve annotated tag, version and exact commit[\s\S]*?        run: \|\r?\n((?:          [^\r\n]*(?:\r?\n|$))*)/);
  assert.ok(run, 'The release tag-validation entry point must be runnable in this regression');
  await writeFile(join(work, 'prepare-release.sh'), run[1].split(/\r?\n/).map(line => line.slice(10)).join('\n'));
  return { root, work, commit, env, git };
}
function runReleaseTagFixture(fixture, tag = 'v0.0.1') {
  const bash = process.platform === 'win32' ? join(process.env.ProgramFiles || 'C:/Program Files', 'Git/bin/bash.exe') : 'bash';
  const result = spawnSync(bash, ['--noprofile', '--norc', 'prepare-release.sh'], {
    cwd: fixture.work, env: { ...fixture.env, TAG: tag, GITHUB_OUTPUT: 'outputs.txt' },
    encoding: 'utf8', windowsHide: true, timeout: 30000,
  });
  if (result.error) throw result.error;
  return result;
}
test('release resolves the real annotated tag despite checkout installing a peeled tag ref', async () => {
  const fixture = await releaseTagFixture({ shadowTag: true });
  assert.equal(fixture.git(fixture.work, 'cat-file', '-t', 'refs/tags/v0.0.1'), 'commit');
  const result = runReleaseTagFixture(fixture);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(await readFile(join(fixture.work, 'outputs.txt'), 'utf8'), 'tag=v0.0.1\nversion=0.0.1\ncommit=' + fixture.commit + '\n');
  assert.equal((await readFile(join(fixture.work, 'release-notes.md'), 'utf8')).trim(), 'annotated release fixture notes');
  assert.equal(fixture.git(fixture.work, 'cat-file', '-t', 'refs/tags/v0.0.1'), 'commit', 'Do not force-overwrite the checkout-owned tag');
});
test('release refuses a lightweight remote tag', async () => {
  const fixture = await releaseTagFixture({ annotated: false });
  const result = runReleaseTagFixture(fixture);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout + result.stderr, /Release notes require an annotated tag/);
  await assert.rejects(readFile(join(fixture.work, 'outputs.txt')), { code: 'ENOENT' });
});
test('release refuses a tag whose workspace version differs', async () => {
  const fixture = await releaseTagFixture({ version: '0.0.2' });
  const result = runReleaseTagFixture(fixture);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout + result.stderr, /Tag\/workspace version mismatch/);
  await assert.rejects(readFile(join(fixture.work, 'outputs.txt')), { code: 'ENOENT' });
});
test('release refuses unsafe or non-stable tag names before fetching', async () => {
  const fixture = await releaseTagFixture();
  const result = runReleaseTagFixture(fixture, 'v0.0.1/../../outside');
  assert.notEqual(result.status, 0);
  assert.match(result.stdout + result.stderr, /Expected stable vX.Y.Z/);
  await assert.rejects(readFile(join(fixture.work, 'outputs.txt')), { code: 'ENOENT' });
});
