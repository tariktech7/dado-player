// Samsung, not the WGT compiler, must issue the TMG and widget.license.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {zipSync} from 'fflate';
const version=JSON.parse(await readFile('../../release.json','utf8')).version;
const release=path.resolve('../../Releases'),source=path.join(release,`Dado-Player-Tizen-${version}.wgt`);
const wgt=await readFile(source),hash=data=>createHash('sha256').update(data).digest('hex');
const arg=name=>{const i=process.argv.indexOf(name);return i<0?undefined:process.argv[i+1];};
const tmgPath=arg('--tmg'),licensePath=arg('--license');
if(!!tmgPath!==!!licensePath)throw Error('Both Samsung-issued --tmg and --license files are required.');
const guide=`Dado Player ${version}: Samsung USB\n\nA WGT file cannot be installed directly from a USB stick on a consumer Samsung TV.\nSamsung must convert this WGT to a .tmg and widget.license using its TV Seller Office USB Demo Packaging facility, if your account/model supports that facility.\nThe developer signing profile alone cannot create these Samsung-issued files.\n\nSource WGT SHA-256: ${hash(wgt)}\nOfficial source: https://developer.samsung.com/smarttv/develop/faq/application-installation.html\n\nAfter obtaining both files: npm run usb -- --tmg <Samsung-file.tmg> --license <widget.license>\nCopy only the resulting userwidget folder to the root of the USB stick. The Samsung license may expire independently of the Dado entitlement. Actual TV installation must be tested.\n`;
await mkdir(release,{recursive:true});
if(!tmgPath){
 const output=path.join(release,`Dado-Player-Samsung-USB-Vorbereitung-${version}.zip`);
 await writeFile(output,zipSync({'README-NICHT-INSTALLIERBAR.txt':new TextEncoder().encode(guide),'status.json':new TextEncoder().encode(JSON.stringify({version,installable:false,reason:'Samsung-issued TMG and widget.license missing',sourceSha256:hash(wgt)},null,2)),[path.basename(source)]:new Uint8Array(wgt)},{level:9}));
 console.log(output);console.log('PREPARATION ONLY: no installable TV USB package has been created.');
}else{
 if(!tmgPath.toLowerCase().endsWith('.tmg')||!licensePath.toLowerCase().endsWith('.license'))throw Error('Expected .tmg and .license files issued by Samsung.');
 const tmg=await readFile(tmgPath),license=await readFile(licensePath);
 if(!tmg.length||!license.length||hash(tmg)===hash(wgt))throw Error('Empty Samsung files or renamed WGT rejected.');
 const output=path.join(release,`Dado-Player-Samsung-USB-${version}.zip`);
 await writeFile(output,zipSync({[`userwidget/${path.basename(tmgPath)}`]:new Uint8Array(tmg),'userwidget/widget.license':new Uint8Array(license)},{level:0}));
 await writeFile(output+'.README.txt',guide+'\nSamsung file signatures and TV compatibility were not independently verified by this bundler.\n');
 console.log(output);console.log('Bundled supplied Samsung-issued files. TV installation and validity remain to be tested.');
}
