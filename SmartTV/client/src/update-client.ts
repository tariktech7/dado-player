import { maxEnvelope, readSlot, saveSlot, verifyBundle, type SignedBundle } from './update-format';

const repo = 'https://api.github.com/repos/tariktech7/dado-player';
export type CheckResult = 'current' | 'ready' | 'reinstall' | 'unavailable' | 'blocked';

export async function stageUpdate(token: string): Promise<CheckResult> {
  const boot = window.dadoTvBoot;
  if (!boot || !token.trim()) return 'unavailable';
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 20_000);
  try {
    const response = await fetch(`${repo}/contents/dado-tizen-${boot.installed.variant}.json?ref=tizen-updates`, {
      headers: { Accept: 'application/vnd.github+json', Authorization: `Bearer ${token.trim()}` },
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
    const blocked = readSlot().blocked;
    if (blocked && bundle.build <= blocked) return 'blocked';
    saveSlot({ bundle: signed, blocked });
    return 'ready';
  } finally {
    clearTimeout(timer);
  }
}
