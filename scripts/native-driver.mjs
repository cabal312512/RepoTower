import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { setTimeout as delay } from 'node:timers/promises';

export const root = path.resolve(import.meta.dirname, '..');
export async function launch(executable, data, args = []) {
  if (data) await fs.mkdir(data, { recursive: true });
  await fs.mkdir(path.join(root, '.tmp'), { recursive: true });
  const child = spawn(executable, ['--automation-stdio', ...data ? ['--data-dir', data] : [], ...args], {
    cwd: root, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, TEMP: path.join(root, '.tmp'), TMP: path.join(root, '.tmp'), TMPDIR: path.join(root, '.tmp') },
  });
  const pending = new Map(); let serial = 0; let stderr = ''; let closed = false; let state;
  child.stderr.on('data', b => { stderr = (stderr + b.toString()).slice(-30_000); });
  child.on('error', error => { for (const p of pending.values()) {clearTimeout(p.timeout);p.reject(error);} pending.clear(); });
  child.on('exit', (code, signal) => {
    closed = true;
    for (const p of pending.values()) {clearTimeout(p.timeout);p.reject(new Error(`Native app exited (${code}, ${signal}): ${stderr}`));}
    pending.clear();
  });
  createInterface({ input: child.stdout }).on('line', line => {
    let response; try { response = JSON.parse(line); } catch { return; }
    const promise = pending.get(response.id); if (!promise) return;
    pending.delete(response.id); clearTimeout(promise.timeout);
    if (response.state) state = response.state;
    promise.resolve(response);
  });
  async function send(command) {
    if (closed) throw new Error(`App is closed. ${stderr}`);
    const id = ++serial;
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`Timeout: ${JSON.stringify(command)}\n${stderr}`)); }, 30_000);
      pending.set(id, { resolve, reject, timeout });
      child.stdin.write(JSON.stringify({ ...command, id }) + '\n');
    });
  }
  async function snapshot() { return (await send({ action: 'state' })).state; }
  async function clickAt(x, y, button = 'left') {
    await send({ action: 'move', x, y });
    await send({ action: 'button', x, y, button, down: true });
    await send({ action: 'button', x, y, button, down: false });
    return snapshot();
  }
  async function click(id, button = 'left') {
    const s = await snapshot(); const r = s.controls[id];
    if (!r) throw new Error(`Control not found: ${id}; available: ${Object.keys(s.controls).join(', ')}`);
    return clickAt(r[0] + r[2] / 2, r[1] + r[3] / 2, button);
  }
  async function waitFor(predicate, timeout = 20_000) {
    const until = Date.now() + timeout;
    while (Date.now() < until) { const current = await snapshot(); if (predicate(current)) return current; await delay(50); }
    throw new Error(`State timeout: ${JSON.stringify(state).slice(0, 2500)}\n${stderr}`);
  }
  async function key(key, ctrl = false) { await send({ action: 'key', key, ctrl, down: true }); await send({ action: 'key', key, ctrl, down: false }); return snapshot(); }
  async function screenshot(file) { await fs.mkdir(path.dirname(file), { recursive: true }); const r = await send({ action: 'screenshot', path: file }); if (!r.screenshot) throw new Error(r.error || 'Screenshot failed'); }
  async function close() { if (!closed) { await send({ action: 'quit' }).catch(() => {}); await delay(150); if (!closed) child.kill(); } }
  await snapshot();
  return { send, snapshot, clickAt, click, key, waitFor, screenshot, close, child, get stderr() { return stderr; } };
}
