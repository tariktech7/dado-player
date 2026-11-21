import { createPublicKey, verify } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';

const directory = resolve(process.argv[2] || '../../Releases/Tizen-Updates');
const ref = 'refs/heads/tizen-updates';
const release = JSON.parse(await readFile('../../release.json', 'utf8'));
const key = createPublicKey({ key: JSON.parse(await readFile('tizen/update-key.json', 'utf8')), format: 'jwk' });
function git(args, input) {
  const result = spawnSync('git', args, { input, encoding: 'utf8', maxBuffer: 2_000_000 });
  if (result.status !== 0) throw new Error(result.stderr || `Git failed: ${args[0]}`);
  return result.stdout.trim();
}

const remote = git(['ls-remote', 'origin', ref]);
const parent = remote ? remote.split(/\s/)[0] : null;
if (parent) git(['fetch', '--no-tags', 'origin', ref]);
const entries = [];
for (const variant of ['legacy', 'modern']) {
  const name = `dado-tizen-${variant}.json`;
  const path = join(directory, name);
  const bytes = await readFile(path);
  if (bytes.length > 900_000) throw new Error(`Too large: ${name}`);
  const envelope = JSON.parse(bytes);
  const bundle = JSON.parse(envelope.payload);
  if (bundle.variant !== variant || bundle.version !== release.version || bundle.build !== release.tizenBuild ||
      !verify('sha256', Buffer.from('Dado Player Tizen update v1\n' + envelope.payload),
        { key, dsaEncoding: 'ieee-p1363' }, Buffer.from(envelope.signature, 'base64url'))) {
    throw new Error(`Failed verification: ${name}`);
  }
  if (parent) {
    const previous = git(['show', `${parent}:${name}`]);
    const old = JSON.parse(JSON.parse(previous).payload);
    if (old.build >= bundle.build && previous !== bytes.toString('utf8').trim()) {
      throw new Error(`Increase tizenBuild before replacing ${name}`);
    }
  }
  entries.push(`100644 blob ${git(['hash-object', '-w', '--', path])}\t${name}\n`);
}
const tree = git(['mktree'], entries.join(''));
if (parent && tree === git(['rev-parse', `${parent}^{tree}`])) {
  console.log('The signed bundles are already published.');
} else {
  const commit = git(['commit-tree', tree, ...(parent ? ['-p', parent] : []), '-m', `Tizen ${release.version} build ${release.tizenBuild}`]);
  git(['push', 'origin', `${commit}:${ref}`]);
  if (!git(['ls-remote', 'origin', ref]).startsWith(commit + '\t')) throw new Error('Update branch verification failed');
  console.log(`Published both TV profiles at ${commit}`);
}
