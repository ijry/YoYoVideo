#!/usr/bin/env node
// Collection is structural only. Production verification ALWAYS also runs the Rust
// signature verifier and archive validator; tests inject only external boundaries.
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { readdir, readFile, lstat, mkdtemp } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { spawnSync } from 'node:child_process';
export const PLATFORMS = Object.freeze(['windows-x64','macos-aarch64','macos-x86_64','linux-x64']);
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const secretNames = /^(YOYOVIDEO_UPDATER_PRIVATE_KEY(_PASSWORD)?|TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)?|GH_TOKEN|GITHUB_TOKEN)$/i;
export function validatePlatformSet(platforms) {
  if (platforms.length !== 4 || new Set(platforms).size !== 4 || platforms.some(p => !PLATFORMS.includes(p))) throw new Error('A release requires all four unique updater platforms');
}
function versionCheck(version) {
  if (!/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(version) || version.length > 64) throw new Error('Expected canonical stable version');
}
function assetName(name) {
  if (typeof name !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,239}$/.test(name) || name.includes('..') || /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(name)) throw new Error('Unsafe asset filename');
}
async function json(file) {
  if ((await lstat(file)).size > 8*1024*1024) throw new Error('Oversized metadata');
  return JSON.parse(await readFile(file, 'utf8'));
}
async function fingerprint(dir, name) {
  assetName(name);
  const file = join(dir, name), stat = await lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size <= 0 || stat.size > 2*1024*1024*1024) throw new Error('Invalid release file');
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return { name, size: stat.size, sha256: hash.digest('hex') };
}
export async function collectRelease(dir, version) {
  versionCheck(version);
  const files = await readdir(dir, {withFileTypes:true});
  if (files.some(f => !f.isFile() || f.isSymbolicLink())) throw new Error('Release directory must contain only regular files');
  const names = files.map(f => f.name); names.forEach(assetName);
  if (new Set(names.map(n=>n.toLowerCase())).size !== names.length) throw new Error('Duplicate asset names');
  const manifests = names.filter(n => /^yoyovideo-update\..+\.json$/.test(n));
  validatePlatformSet(manifests.map(n => n.slice('yoyovideo-update.'.length, -5)));
  const expected = new Set(), packages = [];
  for (const platform of PLATFORMS) {
    const channel = 'stable-'+platform, manifestName = 'yoyovideo-update.'+platform+'.json';
    const manifest = await json(join(dir,manifestName));
    if (manifest.schema_version !== 1 || manifest.app_id !== 'YoYoVideo' || manifest.platform !== platform || manifest.channel !== channel || manifest.version !== version || manifest.release_tag !== 'v'+version) throw new Error('Release manifest context mismatch');
    const feedName = 'releases.'+channel+'.json', feed = await json(join(dir,feedName));
    if (!isDeepStrictEqual(manifest.feed,feed) || !Array.isArray(feed.Assets) || feed.Assets.length !== 1) throw new Error('Unsigned feed differs from signed feed');
    const a=feed.Assets[0]; assetName(a.FileName);
    if (a.Type !== 'Full' || a.PackageId !== 'YoYoVideo' || a.Version !== version || a.FileName !== 'YoYoVideo-'+version+'-'+channel+'-full.nupkg') throw new Error('Wrong full-package identity');
    const packageFile=await fingerprint(dir,a.FileName);
    if (packageFile.size !== a.Size || packageFile.sha256 !== a.SHA256.toLowerCase()) throw new Error('Package hash or size mismatch');
    packages.push(packageFile);
    const indexName='assets.'+channel+'.json', index=await json(join(dir,indexName));
    if (!Array.isArray(index) || index.length !== 2) throw new Error('Unexpected Velopack asset index');
    const full=index.filter(i=>i.Type==='Full'), primary=index.filter(i=>i.Type===(platform==='windows-x64'?'Installer':'Portable'));
    if (full.length !== 1 || full[0].RelativeFileName !== a.FileName || primary.length !== 1) throw new Error('Incomplete native asset index');
    const installable=primary[0].RelativeFileName; assetName(installable);
    const primaryName=platform==='windows-x64'?'YoYoVideo-'+channel+'-Setup.exe':platform==='linux-x64'?'YoYoVideo.AppImage':'YoYoVideo-'+channel+'-Portable.zip';
    if(platform==='linux-x64' ? !/^YoYoVideo(?:-[A-Za-z0-9_.-]+)?\.AppImage$/.test(installable) : installable!==primaryName) throw new Error('Unexpected installable asset name');
    for (const name of [manifestName,manifestName+'.sig',feedName,indexName,'RELEASES-'+channel,a.FileName,installable]) {
      if (expected.has(name)) throw new Error('Cross-platform asset collision');
      expected.add(name);
    }
  }
  // Optional Debian package is explicitly manual/package-manager-only.
  if(names.includes('YoYoVideo-linux-x64.deb')) expected.add('YoYoVideo-linux-x64.deb');
  if (names.length !== expected.size || names.some(n=>!expected.has(n))) throw new Error('Missing or unlisted release assets');
  const known=new Map(packages.map(p=>[p.name,p]));
  return Promise.all([...expected].sort().map(n=>known.get(n) ?? fingerprint(dir,n)));
}
function command(file,args,{github=false}={}) {
  const env={...process.env};
  for(const key of Object.keys(env)) if(secretNames.test(key) && !(github && /^(GH_TOKEN|GITHUB_TOKEN)$/i.test(key))) delete env[key];
  const result=spawnSync(file,args,{cwd:root,env,encoding:'utf8',windowsHide:true,timeout:600000,maxBuffer:8*1024*1024});
  if(result.error || result.status !== 0) throw new Error('Command failed: '+file+' (exit '+result.status+')');
  return result.stdout;
}
export async function verifyRelease(options) {
  const files=await collectRelease(options.assetsDir,options.version);
  for(const platform of PLATFORMS) {
    command('pwsh',['-NoProfile','-File',join(root,'scripts/verify-velopack-package.ps1'),'-ReleaseDir',resolve(options.assetsDir),'-Platform',platform,'-Version',options.version,'-SignerPath',resolve(options.signer),'-PublicKeyPath',resolve(options.publicKey)]);
  }
  return files;
}
export async function publishRelease(options,{gh=(args)=>command('gh',args,{github:true}),verify=verifyRelease}={}) {
  versionCheck(options.version);
  if(!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(options.repo) || !/^[a-f0-9]{40}$/.test(options.commit)) throw new Error('Invalid repository/commit');
  const tag='v'+options.version, api='repos/'+options.repo;
  const before=await verify(options);
  const releases=JSON.parse(await gh(['api',api+'/releases','--paginate','--slurp'])).flat();
  if(releases.some(r=>r.tag_name===tag)) throw new Error('Release already exists; explicit operator cleanup/republish is required');
  async function assertTag() {
    let object=JSON.parse(await gh(['api',api+'/git/ref/tags/'+tag])).object;
    for(let depth=0;object.type==='tag' && depth<8;depth++) object=JSON.parse(await gh(['api',api+'/git/tags/'+object.sha])).object;
    if(object.type!=='commit' || object.sha!==options.commit) throw new Error('Remote tag moved or does not match the built commit');
  }
  await assertTag();
  await gh(['release','create',tag,'--repo',options.repo,'--draft','--verify-tag','--target',options.commit,'--title',tag,'--notes-file',resolve(options.notesFile)]);
  await gh(['release','upload',tag,...before.map(f=>join(resolve(options.assetsDir),f.name)),'--repo',options.repo]);
  const downloaded=await mkdtemp(join(tmpdir(),'yoyovideo-release-verify-'));
  await gh(['release','download',tag,'--repo',options.repo,'--dir',downloaded]);
  const after=await verify({...options,assetsDir:downloaded});
  if(!isDeepStrictEqual(before,after)) throw new Error('Downloaded draft assets differ from verified local files');
  await assertTag();
  await gh(['release','edit',tag,'--repo',options.repo,'--draft=false','--latest']);
  // Retain isolated download diagnostics; never delete user release directories.
}
if(process.argv[1] && import.meta.url===pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const [mode,...args]=process.argv.slice(2), values={};
    for(let i=0;i<args.length;i+=2){if(!args[i].startsWith('--') || !args[i+1] || args[i+1].startsWith('--'))throw new Error('Invalid CLI arguments');values[args[i].slice(2)]=args[i+1];}
    const options={assetsDir:values['assets-dir'],version:values.version,signer:values.signer,publicKey:values['public-key'],repo:values.repo,commit:values.commit,notesFile:values['notes-file']};
    for(const key of ['assetsDir','version','signer','publicKey']) if(!options[key]) throw new Error('Missing '+key);
    if(mode==='verify') await verifyRelease(options);
    else if(mode==='publish') await publishRelease(options);
    else throw new Error('Mode must be verify or publish');
    console.log('Release '+mode+' completed');
  } catch(error) { console.error(error.message); process.exitCode=1; }
}
