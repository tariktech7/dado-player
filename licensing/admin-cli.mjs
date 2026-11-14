// Owner-only local recovery tool. Uses Wrangler's existing account login;
// no API token or signing key is sent to a browser or written to a new file.
import {readFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createInterface} from 'node:readline/promises';
import {macAlias} from './shared/protocol.mjs';
process.chdir(fileURLToPath(new URL('.',import.meta.url)));
const run=args=>{
 const p=spawnSync(process.execPath,['node_modules/wrangler/bin/wrangler.js',...args],{encoding:'utf8'});
 if(p.status!==0)throw Error('Cloudflare-Anmeldung fehlt oder ist abgelaufen. Im Lizenzordner npx wrangler login ausführen.');
 return JSON.parse(p.stdout);
};
const session=run(['auth','token','--json']),account=run(['whoami','--json']);
if(account.accounts.length!==1)throw Error('Mehrere Cloudflare-Konten vorhanden. Kontoauswahl muss eingerichtet werden.');
const config=JSON.parse(readFileSync('wrangler.jsonc','utf8'));
const query=async(sql,params)=>{
 const response=await fetch(`https://api.cloudflare.com/client/v4/accounts/${account.accounts[0].id}/d1/database/${config.d1_databases[0].database_id}/query`,{method:'POST',headers:{Authorization:`Bearer ${session.token}`,'Content-Type':'application/json'},body:JSON.stringify({sql,params})});
 const data=await response.json();if(!response.ok||!data.success)throw Error('Datenbankabfrage fehlgeschlagen. Keine Zugangsdaten weitergeben.');
 return data.result[0].results;
};
const args=process.argv.slice(2),interactive=args.length===0;
const rl=interactive?createInterface({input:process.stdin,output:process.stdout}):null;
try{
 const command=interactive?'approve':args[0];
 const value=(interactive?await rl.question('Geräte-ID oder MAC-Adresse: '):args[1]||'').trim().toUpperCase();
 const alias=await macAlias(value);
 const rows=alias?await query('SELECT devices.* FROM devices JOIN aliases ON aliases.subject=devices.subject WHERE aliases.mac_hash=?',[alias]):await query('SELECT * FROM devices WHERE code=? OR subject=?',[value,value.toLowerCase()]);
 if(!rows.length)throw Error('Gerät nicht gefunden. App zuerst online registrieren; Geräte-ID kontrollieren.');
 console.log(JSON.stringify(rows.map(r=>({device:r.code,platform:r.platform,status:r.approved_at?'Unlimited':'Testversion',started:new Date(r.started_at*1000).toISOString()})),null,2));
 if(command==='lookup')process.exitCode=0;
 else if(command==='approve'){
  const index=args.indexOf('--confirm');const confirmation=interactive?await rl.question('Zahlung geprüft? Zum dauerhaften Freischalten die Geräte-ID exakt wiederholen: '):(index>=0?args[index+1]:'');
  const row=rows.find(r=>r.code===confirmation?.trim().toUpperCase());if(!row)throw Error('Keine eindeutige Bestätigung. Nichts freigeschaltet.');
  const ri=args.indexOf('--reference');const reference=interactive?await rl.question('Zahlungsreferenz (optional): '):(ri>=0?args[ri+1]:'');
  if(reference.length>160)throw Error('Referenz zu lang.');
  const now=Math.floor(Date.now()/1000);
  await query('UPDATE devices SET approved_at=COALESCE(approved_at,?), reference=? WHERE subject=?',[now,reference,row.subject]);
  try{await query('INSERT INTO audit(at,actor,subject,action,reference) VALUES(?,?,?,?,?)',[now,account.email,row.subject,'owner-cli-approve',reference]);}
  catch{console.error('Freischaltung gespeichert; Audit-Protokoll konnte nicht ergänzt werden.');}
  console.log('Freigeschaltet: '+row.code+'. Kunde kann jetzt in der App die Freischaltung abrufen.');
 }else throw Error('Befehl muss lookup oder approve sein.');
}catch(e){console.error(e.message);process.exitCode=1;}finally{rl?.close();}
