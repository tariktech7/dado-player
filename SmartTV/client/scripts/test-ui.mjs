import {mkdir,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
const local=new URL('../../../licensing/.local/',import.meta.url);await mkdir(local,{recursive:true});
const pair=await crypto.subtle.generateKey({name:'ECDSA',namedCurve:'P-256'},true,['sign','verify']);
const pub=await crypto.subtle.exportKey('jwk',pair.publicKey),secret=await crypto.subtle.exportKey('jwk',pair.privateKey);
const config=new URL('ui-config.json',local),keys=new URL('ui-keys.json',local);
await writeFile(config,JSON.stringify({enabled:true,apiUrl:'https://license-fixture.invalid',keys:{'ui-test':pub}}));
await writeFile(keys,JSON.stringify(secret));
const run=(args,env=process.env)=>{const p=spawnSync(process.execPath,args,{stdio:'inherit',env});if(p.error)throw p.error;return p.status??1;};
let code=1;
try{
 if(run(['scripts/build.mjs'],{...process.env,DADO_LICENSE_TEST_CONFIG:fileURLToPath(config)})!==0)throw Error('UI fixture build failed');
 code=run(['node_modules/@playwright/test/cli.js','test',...process.argv.slice(2)],{...process.env,DADO_LICENSE_TEST_KEYS:fileURLToPath(keys)});
}finally{if(run(['scripts/build.mjs'])!==0)throw Error('Production build restoration failed');}
process.exitCode=code;
