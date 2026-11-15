import aliases from '../../../shared/language-aliases.json' with { type: 'json' };
export function languageKey(value:string):string {
 const key=value.trim().toLowerCase().split(/[-_]/)[0];
 return (aliases as Record<string,string>)[key] || key;
}
