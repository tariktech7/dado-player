import {useEffect,useMemo,useState} from 'preact/hooks';
import type {Catalog,Profile} from './types';
import {ParentalControls,emptyPolicy,lockKey,withPin} from './parental';
import {TVInput} from './input';
import {TVSelect} from './select';
export function ParentalPanel({controls,catalog,profile,onChange}:{controls:ParentalControls;catalog:Catalog;profile:Profile|null;onChange:()=>void}) {
  const [,render]=useState(0),[authenticated,auth]=useState(false),[pin,setPin]=useState(''),[first,setFirst]=useState(''),[changing,change]=useState(false),[error,setError]=useState(''),[busy,setBusy]=useState(false),[kind,setKind]=useState<'live'|'vod'|'series'>('live'),[query,setQuery]=useState(''),[limit,setLimit]=useState(40);
  controls.changed=()=>{render(n=>n+1);onChange();};
  useEffect(()=>{
    auth(false);setPin('');setFirst('');change(false);setError('');setQuery('');setLimit(40);
    if(controls.open)requestAnimationFrame(()=>document.querySelector<HTMLButtonElement>('[data-parental] button')?.focus());
  },[controls.open,controls.pending]);
  const rows=useMemo(()=>catalog[kind].filter(i=>i.name.toLocaleLowerCase().includes(query.toLocaleLowerCase())),[catalog,kind,query]);
  if(!controls.open)return null;
  const setup=!controls.enabled||changing;
  const run=async(action:()=>Promise<void>|void)=>{setBusy(true);setError('');try{await action();}catch(e){setError((e as Error).message);}finally{setBusy(false);setPin('');}};
  return <div class="scrim parental-scrim"><section class="modal glass parental-modal" role="dialog" aria-modal="true" aria-label="Elternsperre" data-parental>
    <h1>Elternsperre</h1><button onClick={()=>controls.close()}>Zurück / Abbrechen</button>
    <p role="status">{error}</p>
    {setup||!authenticated||controls.pending?<>
      <p>{setup?(first?'Neue PIN bestätigen':'Neue PIN: genau vier Ziffern'):'Geschützt · Eltern-PIN eingeben'}</p>
      {controls.pending&&<h2>{controls.pending.name}</h2>}
      <output aria-label="PIN-Eingabe">{'●'.repeat(pin.length)}{'○'.repeat(4-pin.length)}</output>
      <div class="parental-keypad">{['1','2','3','4','5','6','7','8','9','Löschen','0'].map(n=><button aria-label={n} disabled={busy} onClick={()=>setPin(v=>n==='Löschen'?v.slice(0,-1):(v+n).slice(0,4))}>{n==='Löschen'?'⌫':n}</button>)}</div>
      <button class="primary" disabled={busy||pin.length!==4} onClick={()=>void run(async()=>{
        if(setup){
          if(!first){setFirst(pin);return;}
          if(first!==pin){setFirst('');throw Error('PINs stimmen nicht überein. Bitte erneut einrichten.');}
          const revision=controls.revision,next=await withPin(controls.policy,pin);
          if(revision!==controls.revision||!controls.open)return;
          controls.persist(next);setFirst('');change(false);auth(true);
        }else{await controls.authenticate(pin);auth(true);}
      })}>{setup?'PIN speichern / bestätigen':'Entsperren'}</button>
    </>:<>
      <p>Gesperrte Serien schützen alle Staffeln und Folgen. Die Auswahl gilt für das jeweilige Anbieterprofil auf diesem Gerät.</p>
      <button disabled={busy} onClick={()=>change(true)}>PIN ändern</button>
      <button disabled={busy} onClick={()=>void run(()=>{controls.persist(emptyPolicy());controls.close();})}>Elternsperre ausschalten</button>
      <TVSelect value={kind} options={[["live","Sender"],["vod","Filme"],["series","Serien"]]} onChange={v=>{setKind(v as typeof kind);setLimit(40);}}/>
      <TVInput aria-label="Inhalte suchen" placeholder="Inhalte suchen …" value={query} onInput={e=>{setQuery(e.currentTarget.value);setLimit(40);}}/>
      {!profile?<p>Zuerst ein Anbieterprofil anmelden.</p>:<div class="parental-items">{rows.slice(0,limit).map(item=>{
        const key=lockKey(profile.id,item),locked=controls.locked(profile.id,item);
        return <button disabled={busy} aria-pressed={locked} onClick={()=>void run(()=>controls.persist({...controls.policy,blocked:locked?controls.policy.blocked.filter(k=>k!==key):[...controls.policy.blocked,key]}))}>{locked?'Gesperrt · ':'Freigegeben · '}{item.name}</button>;
      })}{rows.length>limit&&<button onClick={()=>setLimit(n=>n+40)}>Weitere Inhalte anzeigen</button>}</div>}
    </>}
  </section></div>;
}
