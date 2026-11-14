export const PRODUCT = 'dado-player';
export const TRIAL_SECONDS = 7 * 24 * 60 * 60;
export function encode(bytes) {
  return btoa(String.fromCharCode(...bytes)).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}
export function decode(text) {
  if (!/^[A-Za-z0-9_-]+$/.test(text)) throw Error('Ungültiges Lizenzformat');
  return Uint8Array.from(atob(text.replaceAll('-', '+').replaceAll('_', '/')), c => c.charCodeAt(0));
}
export async function fingerprint(platform, identifier) {
  const bytes = new TextEncoder().encode(`dado-device-v1:${platform}:${identifier}`);
  return [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(b => b.toString(16).padStart(2, '0')).join('');
}
export function deviceCode(subject) {
  if (!/^[a-f0-9]{64}$/.test(subject)) throw Error('Ungültige Gerätekennung');
  return 'D7-' + subject.slice(0, 24).toUpperCase().match(/.{4}/g).join('-');
}
export function normalizeMac(value) {
  const hex = String(value || '').replaceAll(':', '').replaceAll('-', '').toUpperCase();
  return /^[0-9A-F]{12}$/.test(hex) && !['000000000000','FFFFFFFFFFFF','020000000000'].includes(hex)
    ? hex.match(/../g).join(':') : null;
}
export async function macAlias(mac) {
  const normalized = normalizeMac(mac);
  return normalized ? fingerprint('mac-alias', normalized) : null;
}
export async function signLicense(claims, privateJwk, kid) {
  const header = encode(new TextEncoder().encode(JSON.stringify({alg:'ES256',typ:'DADO-LICENSE',kid})));
  const payload = encode(new TextEncoder().encode(JSON.stringify(claims)));
  const input = `${header}.${payload}`;
  const key = await crypto.subtle.importKey('jwk', privateJwk, {name:'ECDSA',namedCurve:'P-256'}, false, ['sign']);
  const signature = await crypto.subtle.sign({name:'ECDSA',hash:'SHA-256'}, key, new TextEncoder().encode(input));
  return `${input}.${encode(new Uint8Array(signature))}`;
}
export async function verifyLicense(token, keys, subject, platform, now) {
  if (typeof token !== 'string' || token.length > 8192) throw Error('Ungültige Lizenz');
  const parts = token.split('.');
  if (parts.length !== 3) throw Error('Ungültige Lizenz');
  const header = JSON.parse(new TextDecoder().decode(decode(parts[0])));
  if (header.alg !== 'ES256' || header.typ !== 'DADO-LICENSE' || !Object.prototype.hasOwnProperty.call(keys, header.kid)) throw Error('Unbekannter Lizenzschlüssel');
  const key = await crypto.subtle.importKey('jwk', keys[header.kid], {name:'ECDSA',namedCurve:'P-256'}, false, ['verify']);
  if (!await crypto.subtle.verify({name:'ECDSA',hash:'SHA-256'}, key, decode(parts[2]), new TextEncoder().encode(`${parts[0]}.${parts[1]}`))) throw Error('Lizenzsignatur ungültig');
  const c = JSON.parse(new TextDecoder().decode(decode(parts[1])));
  if (c.v !== 1 || c.product !== PRODUCT || c.subject !== subject || c.platform !== platform || !['trial','permanent'].includes(c.kind) || !Number.isSafeInteger(c.issued_at) || typeof c.license_id !== 'string') throw Error('Lizenz gehört nicht zu diesem Gerät');
  if (c.kind === 'permanent' && c.expires_at !== null) throw Error('Ungültige Dauerlizenz');
  if (c.kind === 'trial' && (!Number.isSafeInteger(c.started_at) || c.expires_at !== c.started_at + TRIAL_SECONDS)) throw Error('Ungültige Testzeit');
  return {...c, expired:c.kind === 'trial' && now >= c.expires_at};
}
export function statusText(claims, now) {
  if (!claims) return 'Testzeit noch nicht gestartet';
  if (claims.kind === 'permanent') return 'Unlimited';
  const left = Math.max(0, Math.ceil((claims.expires_at - now) / 86400));
  return left ? `Noch ${left} ${left === 1 ? 'Tag' : 'Tage'} Testzeit` : 'Testzeit abgelaufen';
}
