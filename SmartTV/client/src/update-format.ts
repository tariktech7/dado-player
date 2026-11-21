import publicKey from '../tizen/update-key.json' with { type: 'json' };

export const maxEnvelope = 900_000;
export const updateStorage = 'dado.tv-update.v1';
export const accessStorage = 'dado.tv-update-access.v1';
export const signingContext = 'Dado Player Tizen update v1\n';

export type Variant = 'modern' | 'legacy';
type Build = { build: number; version: string; variant: Variant; compatibility: string };
export type Bundle = Build & { script: string; css: string };
export type SignedBundle = { payload: string; signature: string };
export type UpdateSlot = { bundle?: SignedBundle; attempting?: number; blocked?: number };
export type Boot = {
  installed: Build;
  running: Build;
  fromUpdate: boolean;
  ready: () => void;
};

declare global {
  interface Window { dadoTvBoot?: Boot }
}

export function readSlot(): UpdateSlot {
  try {
    return JSON.parse(localStorage.getItem(updateStorage) || '{}') || {};
  } catch {
    return {};
  }
}

export function saveSlot(value: UpdateSlot): void {
  localStorage.setItem(updateStorage, JSON.stringify(value));
}

export async function verifyBundle(signed: SignedBundle, key: JsonWebKey = publicKey): Promise<Bundle> {
  if (typeof signed?.payload !== 'string' || signed.payload.length > maxEnvelope ||
      typeof signed.signature !== 'string' || !/^[A-Za-z0-9_-]{86}$/.test(signed.signature)) {
    throw new Error('Invalid update envelope');
  }
  const signature = Uint8Array.from(atob(signed.signature.replace(/-/g, '+').replace(/_/g, '/') + '=='), c => c.charCodeAt(0));
  const imported = await crypto.subtle.importKey('jwk', key, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify']);
  const verified = await crypto.subtle.verify({ name: 'ECDSA', hash: 'SHA-256' }, imported, signature,
    new TextEncoder().encode(signingContext + signed.payload));
  if (!verified) throw new Error('Invalid update signature');
  const bundle = JSON.parse(signed.payload) as Bundle;
  if (!bundle || !Number.isSafeInteger(bundle.build) || bundle.build < 1 ||
      !/^\d+\.\d+\.\d+$/.test(bundle.version) ||
      !/^[a-f0-9]{64}$/.test(bundle.compatibility) || !['modern', 'legacy'].includes(bundle.variant) ||
      typeof bundle.script !== 'string' || !bundle.script || typeof bundle.css !== 'string' || !bundle.css) {
    throw new Error('Invalid update bundle');
  }
  return bundle;
}
