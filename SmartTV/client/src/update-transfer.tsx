import qrcode from 'qrcode-generator';
import { useEffect, useRef, useState } from 'preact/hooks';
import config from '../../../connect/public-config.json';

const encode = new TextEncoder();
const toHex = (bytes: Uint8Array) =>
  Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
const randomHex = () => toHex(crypto.getRandomValues(new Uint8Array(32)));
const digest = async (text: string) =>
  toHex(new Uint8Array(await crypto.subtle.digest('SHA-256', encode.encode(text))));
const unpack = (text: string) =>
  Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/')), (char) => char.charCodeAt(0));
const validToken = /^(?:github_pat_|ghp_)[A-Za-z0-9_]{20,500}$/;

const request = (action: string, data: object, signal?: AbortSignal) =>
  fetch(config.apiUrl + '/v1/update-transfer/' + action, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
    credentials: 'omit',
    cache: 'no-store',
    signal,
  });

export function UpdateTokenTransfer({ onToken, t }: {
  onToken: (value: string) => boolean;
  t: (de: string, en: string) => string;
}) {
  const [active, setActive] = useState(false);
  const [image, setImage] = useState('');
  const [status, setStatus] = useState('');
  const latest = useRef({ onToken, t });
  latest.current = { onToken, t };

  useEffect(() => {
    if (!active) return;
    const controller = new AbortController();
    const readKey = randomHex();
    const secret = randomHex();
    const translate = latest.current.t;
    let pollTimer: ReturnType<typeof setTimeout>;
    let started = false;

    const finish = (message: string) => {
      if (controller.signal.aborted) return;
      setStatus(message);
      setImage('');
      setActive(false);
    };
    const expired = () => finish(translate('QR-Code abgelaufen. Bitte erneut starten.', 'QR code expired. Please start again.'));
    const failed = () => finish(translate('Übertragung fehlgeschlagen. Bitte erneut versuchen.', 'Transfer failed. Please try again.'));
    const expiry = setTimeout(expired, 300_000);
    const close = () => void request('close', { readKey }).catch(() => {});

    void (async () => {
      try {
        const id = await digest(readKey);
        const key = await crypto.subtle.importKey('raw',
          Uint8Array.from(secret.match(/../g)!, (pair) => parseInt(pair, 16)),
          'AES-GCM', false, ['decrypt']);
        const writeHash = await digest(await digest('send:' + secret));
        if (controller.signal.aborted) return;
        const response = await request('start', { readKey, writeHash }, controller.signal);
        if (!response.ok) throw new Error('Could not start transfer');
        started = true;
        if (controller.signal.aborted) { close(); return; }
        const url = new URL('/update-transfer', config.portalUrl);
        url.hash = id + '.' + secret;
        const qr = qrcode(0, 'M');
        qr.addData(url.href);
        qr.make();
        setImage(qr.createDataURL(4, 16));
        setStatus(translate('Scanne den QR-Code mit der Handykamera und füge den Token ein.',
          'Scan the QR code with your phone camera and paste the token.'));
        const poll = async () => {
          try {
            const reply = await request('poll', { readKey }, controller.signal);
            if (reply.status === 410) { expired(); return; }
            if (!reply.ok) throw new Error('Could not check transfer');
            const { cipher } = await reply.json();
            if (controller.signal.aborted) return;
            if (cipher) {
              const [iv, encrypted] = String(cipher).split('.');
              const decoded = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: unpack(iv), additionalData: encode.encode(id) },
                key, unpack(encrypted));
              const value = new TextDecoder().decode(decoded);
              if (controller.signal.aborted) return;
              if (!validToken.test(value) || !latest.current.onToken(value)) { failed(); return; }
              finish(translate('GitHub-Token gespeichert.', 'GitHub token saved.'));
              return;
            }
          } catch {
            if (controller.signal.aborted) return;
          }
          pollTimer = setTimeout(poll, 3000);
        };
        void poll();
      } catch {
        failed();
      }
    })();
    return () => {
      controller.abort();
      clearTimeout(expiry);
      clearTimeout(pollTimer);
      if (started) close();
    };
  }, [active]);

  return <>
    <button type="button" onClick={() => {
      setImage('');
      setStatus(active ? '' : t('QR-Code wird erstellt …', 'Creating QR code …'));
      setActive(!active);
    }}>{active ? t('Übertragung abbrechen', 'Cancel transfer') : t('Token per Handy übertragen', 'Transfer token from phone')}</button>
    {image && <img class="update-transfer-code" src={image} alt={t('QR-Code zur Token-Übertragung', 'Token transfer QR code')} />}
    {status && <p role="status">{status}</p>}
  </>;
}
