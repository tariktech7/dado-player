// Both packages use these feature checks before loading the app.
if (!(String.prototype as any).replaceAll) Object.defineProperty(String.prototype,'replaceAll',{value:function(this:string,search:string|RegExp,replacement:string){
 if(search instanceof RegExp){if(!search.global)throw new TypeError('Global RegExp required');return this.replace(search,replacement);}
 return this.split(String(search)).join(replacement);
},configurable:true,writable:true});
if (!Array.prototype.flatMap) Object.defineProperty(Array.prototype,'flatMap',{value:function(this:unknown[],fn:(value:unknown,index:number,array:unknown[])=>unknown,scope?:unknown){
 return this.reduce<unknown[]>((out,value,index)=>{const result=fn.call(scope,value,index,this);return out.concat(result);},[]);
},configurable:true,writable:true});
if (!crypto.randomUUID) (crypto as any).randomUUID=()=>{
 const b=crypto.getRandomValues(new Uint8Array(16));b[6]=(b[6]&15)|64;b[8]=(b[8]&63)|128;
 const h=Array.from(b,n=>n.toString(16).padStart(2,'0')).join('');return `${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;
};
if (!(window as any).AbortController) {
 class Signal {
  aborted=false;private listeners=new Set<EventListener>();
  addEventListener(type:string,listener:EventListener){if(type==='abort')this.listeners.add(listener);}
  removeEventListener(type:string,listener:EventListener){if(type==='abort')this.listeners.delete(listener);}
  abort(){if(this.aborted)return;this.aborted=true;for(const listener of this.listeners)listener(new Event('abort'));}
 }
 (window as any).AbortController=class {signal=new Signal();abort(){this.signal.abort();}};
 const nativeFetch=window.fetch.bind(window);
 window.fetch=(input:RequestInfo|URL,init:RequestInit={})=>{
  const signal=init.signal as unknown as Signal|undefined;
  if(!signal)return nativeFetch(input,init);
  return new Promise<Response>((resolve,reject)=>{
   const xhr=new XMLHttpRequest(),abort=()=>{xhr.abort();reject(new DOMException('Request aborted','AbortError'));};
   if(signal.aborted){abort();return;}
   const request=input instanceof Request?input:null;
   xhr.open(init.method||request?.method||'GET',request?.url||String(input));xhr.responseType='arraybuffer';
   xhr.withCredentials=init.credentials==='include';
   new Headers(init.headers||request?.headers).forEach((value,key)=>xhr.setRequestHeader(key,value));
   const clean=()=>signal.removeEventListener('abort',abort);
   xhr.onload=()=>{clean();const headers=new Headers();xhr.getAllResponseHeaders().trim().split(/[\r\n]+/).forEach(line=>{const i=line.indexOf(':');if(i>0)headers.append(line.slice(0,i).trim(),line.slice(i+1).trim());});
    resolve(new Response([204,205,304].includes(xhr.status)?null:xhr.response,{status:xhr.status,statusText:xhr.statusText,headers}));};
   xhr.onerror=()=>{clean();reject(new TypeError('Network request failed'));};xhr.onabort=()=>{clean();reject(new DOMException('Request aborted','AbortError'));};
   signal.addEventListener('abort',abort);xhr.send((init.body||null) as XMLHttpRequestBodyInit|null);
  });
 };
}
// Chromium 63 ignores preventScroll; restore ancestor offsets before our own
// directional scrolling runs, rather than allowing native focus to jump first.
function supportsPreventScroll(): boolean {
 let supported=false;
 const probe=document.createElement('button');
 probe.style.cssText='position:fixed;left:-100px;top:0';
 document.body.appendChild(probe);
 probe.focus({ get preventScroll() { supported=true;return true; } });
 probe.remove();
 return supported;
}
const nativeFocus=HTMLElement.prototype.focus;
if (!supportsPreventScroll()) HTMLElement.prototype.focus=function(options?:FocusOptions){
 if(!options?.preventScroll){nativeFocus.call(this);return;}
 const positions:{el:HTMLElement;top:number;left:number}[]=[];
 for(let el=this.parentElement;el;el=el.parentElement)positions.push({el,top:el.scrollTop,left:el.scrollLeft});
 nativeFocus.call(this);for(const p of positions){p.el.scrollTop=p.top;p.el.scrollLeft=p.left;}
};
