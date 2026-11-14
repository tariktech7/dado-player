import {writeFileSync,existsSync} from 'node:fs';
if(existsSync('.local/signing-key.dpapi')) throw Error('Signing key already exists; do not overwrite');
const pair=await crypto.subtle.generateKey({name:'ECDSA',namedCurve:'P-256'},true,['sign','verify']);
const pub=await crypto.subtle.exportKey('jwk',pair.publicKey);
const secret=await crypto.subtle.exportKey('jwk',pair.privateKey);
writeFileSync('public-config.json',JSON.stringify({enabled:false,apiUrl:'https://dado-license.dado-licensing.workers.dev',keys:{'production-2026-11':pub}},null,2)+'\n');
process.stdout.write(JSON.stringify(secret));
