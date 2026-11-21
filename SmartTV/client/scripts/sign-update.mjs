import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { resolve, join } from 'node:path';
import { homedir } from 'node:os';

const out = resolve(process.argv[2] || '../../Releases');
const metadata = JSON.parse(await readFile('dist/version.json', 'utf8'));
if (metadata.licenseTestBuild || !metadata.update) throw new Error('A production TV build is required');
const key = createPrivateKey(await readFile(process.env.DADO_TV_KEY_FILE || join(homedir(), '.dado-player/tizen-update-private.pem')));
const publicKey = JSON.parse(await readFile('tizen/update-key.json', 'utf8'));
const actual = createPublicKey(key).export({ format: 'jwk' });
if (actual.x !== publicKey.x || actual.y !== publicKey.y) throw new Error('Incorrect update signing key');
const payload = JSON.stringify({ ...metadata.update,
  script: await readFile('dist/app.js', 'utf8'), css: await readFile('dist/app.css', 'utf8') });
const signature = sign('sha256', Buffer.from('Dado Player Tizen update v1\n' + payload),
  { key, dsaEncoding: 'ieee-p1363' }).toString('base64url');
const body = JSON.stringify({ payload, signature });
if (Buffer.byteLength(body) > 900_000) throw new Error('Update exceeds the maximum size');
await mkdir(out, { recursive: true });
const file = join(out, `dado-tizen-${metadata.update.variant}.json`);
await writeFile(file, body);
console.log(file);
