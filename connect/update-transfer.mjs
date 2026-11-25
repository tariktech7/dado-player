import { body, fail, hash, json } from './protocol.mjs';

const hex = /^[a-f0-9]{64}$/;
const cipherPattern = /^[A-Za-z0-9_-]{16}\.[A-Za-z0-9_-]{30,720}$/;

export async function transferUpdateToken(request, db, action, now) {
  const input = await body(request);
  if (!input || typeof input !== 'object') fail('Ungültige Übertragung.');
  if (action === 'send') {
    const { id, proof, cipher } = input;
    if (typeof id !== 'string' || !hex.test(id) || typeof proof !== 'string' ||
        !hex.test(proof) || typeof cipher !== 'string' || !cipherPattern.test(cipher))
      fail('Ungültige Übertragung.');
    const sent = await db.prepare(
      'UPDATE update_transfers SET cipher=? WHERE id=? AND write_hash=? AND expires>? AND cipher IS NULL RETURNING id'
    ).bind(cipher, id, await hash(proof), now).first();
    if (!sent) fail('Der QR-Code ist abgelaufen oder wurde bereits verwendet.', 410);
    return json({ ok: true });
  }

  const { readKey, writeHash } = input;
  if (typeof readKey !== 'string' || !hex.test(readKey)) fail('Ungültige Übertragung.');
  const id = await hash(readKey);
  if (action === 'start') {
    if (typeof writeHash !== 'string' || !hex.test(writeHash)) fail('Ungültige Übertragung.');
    await db.prepare('INSERT INTO update_transfers(id,write_hash,expires) VALUES(?,?,?)')
      .bind(id, writeHash, now + 300).run();
    return json({ id }, 201);
  }
  if (action === 'poll') {
    const row = await db.prepare('SELECT cipher FROM update_transfers WHERE id=? AND expires>?')
      .bind(id, now).first();
    if (!row) fail('Der QR-Code ist abgelaufen.', 410);
    return json({ cipher: row.cipher });
  }
  if (action === 'close') {
    await db.prepare('DELETE FROM update_transfers WHERE id=?').bind(id).run();
    return json({ ok: true });
  }
  fail('Nicht gefunden.', 404);
}

export const updateTransferPage = `<!doctype html>
<html lang="de"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dado Player · GitHub-Token übertragen</title><link rel="stylesheet" href="/style.css">
<script defer src="/update-transfer.js"></script></head>
<body><main><section class="glass"><h1>GitHub-Token übertragen</h1>
<p>Füge deinen GitHub-Lese-Token ein. Das Handy verschlüsselt ihn für den TV, der den QR-Code anzeigt.</p>
<form id="transfer"><label>GitHub-Lese-Token<input id="token" type="password" autocomplete="off" autocapitalize="off" spellcheck="false" required maxlength="512"></label>
<button type="submit">An den TV senden</button></form>
<p id="result" role="status"></p></section></main></body></html>`;

export const updateTransferScript = String.raw`
const form = document.getElementById('transfer');
const field = document.getElementById('token');
const result = document.getElementById('result');
const [id, secret] = location.hash.slice(1).split('.');
history.replaceState(null, '', location.pathname);
const hex = /^[a-f0-9]{64}$/;
const encoder = new TextEncoder();
const b64 = bytes => btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
if (!hex.test(id) || !hex.test(secret)) {
  form.hidden = true;
  result.textContent = 'Öffne diese Seite über einen neuen QR-Code am Fernseher.';
}
form.addEventListener('submit', async event => {
  event.preventDefault();
  const token = field.value.trim();
  if (!/^(?:github_pat_|ghp_)[A-Za-z0-9_]{20,500}$/.test(token)) {
    result.textContent = 'Bitte den vollständigen GitHub-Lese-Token einfügen.';
    return;
  }
  const button = form.querySelector('button');
  button.disabled = true;
  result.textContent = 'Wird übertragen …';
  try {
    const key = await crypto.subtle.importKey('raw', Uint8Array.from(secret.match(/../g), pair => parseInt(pair, 16)), 'AES-GCM', false, ['encrypt']);
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv, additionalData: encoder.encode(id) }, key, encoder.encode(token));
    const proof = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', encoder.encode('send:' + secret))), byte => byte.toString(16).padStart(2, '0')).join('');
    const response = await fetch('/v1/update-transfer/send', { method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id, proof, cipher: b64(iv) + '.' + b64(new Uint8Array(encrypted)) })
    });
    if (response.status === 410) {
      field.value = '';
      form.hidden = true;
      result.textContent = 'Dieser QR-Code ist abgelaufen oder wurde bereits verwendet.';
    } else if (response.ok) {
      field.value = '';
      form.hidden = true;
      result.textContent = 'Gesendet. Der Fernseher bestätigt den Empfang.';
    } else throw new Error();
  } catch {
    result.textContent = 'Übertragung fehlgeschlagen. Bitte erneut versuchen.';
  } finally {
    button.disabled = false;
  }
});`;
