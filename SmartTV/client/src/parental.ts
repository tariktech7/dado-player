import type {Item} from './types';
export interface ParentalPolicy {salt:number[];hash:number[];blocked:string[]}
export const emptyPolicy=():ParentalPolicy=>({salt:[],hash:[],blocked:[]});
export const validPin=(pin:string)=>/^[0-9]{4}$/.test(pin);
export function lockKey(profile:string,item:Item):string {
  const kind=item.kind==='episode'?'series':item.kind;
  const id=item.kind==='episode'?item.seriesId:item.id;
  return JSON.stringify([profile,kind,id]);
}
async function derive(pin:string,salt:number[]) {
  const key=await crypto.subtle.importKey('raw',new TextEncoder().encode(pin),'PBKDF2',false,['deriveBits']);
  return new Uint8Array(await crypto.subtle.deriveBits({name:'PBKDF2',salt:new Uint8Array(salt),iterations:100000,hash:'SHA-256'},key,256));
}
export async function withPin(policy:ParentalPolicy,pin:string):Promise<ParentalPolicy> {
  if(!validPin(pin))throw Error('Bitte genau vier Ziffern eingeben.');
  const salt=Array.from(crypto.getRandomValues(new Uint8Array(16)));
  return {...policy,salt,hash:Array.from(await derive(pin,salt))};
}
export async function verifyPin(policy:ParentalPolicy,pin:string) {
  if(!validPin(pin)||policy.hash.length!==32)return false;
  const actual=await derive(pin,policy.salt);let different=0;
  actual.forEach((n,i)=>different|=n^policy.hash[i]);return different===0;
}
/** Separate from provider/settings storage, so imports/logout cannot disable it. */
export class ParentalControls {
  policy:ParentalPolicy;open=false;pending:Item|null=null;changed=()=>{};
  private resolve:((allowed:boolean)=>void)|null=null;
  private attempts=0;private cooldown=0;
  revision=0;
  constructor(private storage:Pick<Storage,'getItem'|'setItem'>=localStorage) {
    try {
      const raw=storage.getItem('dado.parental.v1');this.policy=raw?JSON.parse(raw):emptyPolicy();
      if(!Array.isArray(this.policy.hash)||!Array.isArray(this.policy.blocked)||!Array.isArray(this.policy.salt))throw Error('Invalid protection');
    } catch { this.policy={salt:[],hash:Array(32).fill(0),blocked:['*']}; }
  }
  get enabled(){return this.policy.hash.length>0;}
  locked(profile:string,item:Item){return this.enabled&&(this.policy.blocked.includes('*')||this.policy.blocked.includes(lockKey(profile,item)));}
  persist(next:ParentalPolicy){this.storage.setItem('dado.parental.v1',JSON.stringify(next));this.policy=next;this.changed();}
  manage(){this.close();this.open=true;this.changed();}
  close(){this.revision++;this.resolve?.(false);this.resolve=null;this.open=false;this.pending=null;this.changed();}
  request(profile:string,item:Item):Promise<boolean>{
    if(!this.locked(profile,item))return Promise.resolve(true);
    this.close();this.pending=item;this.open=true;
    return new Promise(resolve=>{this.resolve=resolve;this.changed();});
  }
  async authenticate(pin:string){
    const revision=this.revision;
    if(performance.now()<this.cooldown)throw Error('Zu viele Versuche. Bitte 30 Sekunden warten.');
    const valid=await verifyPin(this.policy,pin);
    if(revision!==this.revision||!this.open)throw Error('Abgebrochen.');
    if(!valid){
      if(++this.attempts>=5){this.attempts=0;this.cooldown=performance.now()+30000;throw Error('Zu viele Versuche. Bitte 30 Sekunden warten.');}
      throw Error('Falsche PIN.');
    }
    this.attempts=0;
    if(this.resolve){const done=this.resolve;this.resolve=null;this.open=false;this.pending=null;done(true);this.changed();}
  }
}
