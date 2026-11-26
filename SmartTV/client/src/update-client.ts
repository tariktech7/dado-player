import { maxEnvelope, readSlot, saveSlot, verifyBundle, type SignedBundle } from './update-format';

const previousStorage = 'dado.tv-previous-update.v1';
const repo = 'https://api.github.com/repos/tariktech7/dado-player';
export type CheckResult = 'current' | 'ready' | 'reinstall' | 'unavailable' | 'blocked';

export async function stageUpdate(): Promise<CheckResult> {
  const boot = window.dadoTvBoot;
  if (!boot) return 'unavailable';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 20_000);
  try {
    const response = await fetch(`${repo}/contents/dado-tizen-${boot.installed.variant}.json?ref=tizen-updates`, {
      headers: { Accept: 'application/vnd.github+json' },
      cache: 'no-store',
      redirect: 'error',
      signal: controller.signal,
    });
    if (response.status === 404 || response.status === 401 || response.status === 403) return 'unavailable';
    if (!response.ok) throw new Error('Update request failed');
    const file = await response.json() as { encoding: string; content: string; size: number };
    if (file.encoding !== 'base64' || file.size < 1 || file.size > maxEnvelope ||
        typeof file.content !== 'string' || file.content.length > maxEnvelope * 1.5) throw new Error('Invalid update size');
    const bytes = Uint8Array.from(atob(file.content), c => c.charCodeAt(0));
    if (bytes.length !== file.size) throw new Error('Incomplete update');
    const signed = JSON.parse(new TextDecoder().decode(bytes)) as SignedBundle;
    const bundle = await verifyBundle(signed);
    if (bundle.variant !== boot.installed.variant) throw new Error('Incorrect TV profile');
    if (bundle.build <= boot.running.build) return 'current';
    if (bundle.compatibility !== boot.installed.compatibility) return 'reinstall';
    const slot = readSlot();
    const blocked = slot.blocked;
    if (blocked && bundle.build <= blocked) return 'blocked';
    if (boot.fromUpdate && slot.bundle) {
      localStorage.setItem(previousStorage, JSON.stringify({ forBuild: bundle.build, bundle: slot.bundle }));
    } else {
      localStorage.removeItem(previousStorage);
    }
    saveSlot({ bundle: signed, blocked });
    return 'ready';
  } finally {
    clearTimeout(timer);
  }
}

export async function restorePrevious(): Promise<boolean> {
  const boot = window.dadoTvBoot;
  if (!boot?.fromUpdate) return false;

  const { blocked } = readSlot();
  let previous: SignedBundle | undefined;
  try {
    const saved = JSON.parse(localStorage.getItem(previousStorage) || 'null');
    if (saved?.forBuild === boot.running.build) {
      const bundle = await verifyBundle(saved.bundle);
      if (bundle.variant === boot.installed.variant &&
          bundle.compatibility === boot.installed.compatibility &&
          bundle.build > Math.max(boot.installed.build, blocked || 0) &&
          bundle.build < boot.running.build) previous = saved.bundle;
    }
  } catch { /* A missing or invalid backup falls back to the signed installed package. */ }
  saveSlot({ bundle: previous, blocked });
  return true;
}
