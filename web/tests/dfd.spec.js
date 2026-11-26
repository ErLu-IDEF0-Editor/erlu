import { readFile } from 'node:fs/promises';
import { expect, test } from '@playwright/test';
import { call, clickUi, ready } from './helpers.js';

const example = await readFile(new URL('../../examples/order-fulfilment.dfd', import.meta.url), 'utf8');

async function clickCanvas(page, x, y) {
 const target = await page.evaluate(() => window.erluRuntime.uiSnapshot().targets['dfd.canvas']);
 const canvas = await page.locator('#erlu').boundingBox();
 const scale = Number(target.value);
 await page.mouse.click(canvas.x + target.x + target.width / 2 + x * scale,
  canvas.y + target.y + target.height / 2 + y * scale);
}

test('DFD editor creates a document and opens a decomposition on the canvas', async ({ page }) => {
 await ready(page);
 await clickUi(page, 'startup.new_dfd');
 await expect.poll(async () => (await call(page, 'document_state')).response.result.document.format).toBe('erlu-dfd');
 await clickUi(page, 'dfd.add.external');
 await clickCanvas(page, -68, 0);
 await clickUi(page, 'dfd.properties.save');
 await clickUi(page, 'dfd.flow');
 await clickCanvas(page, -68, 0);
 await clickCanvas(page, 0, 0);
 await clickUi(page, 'dfd.properties.title');
 await page.keyboard.insertText('Order request');
 await clickUi(page, 'dfd.properties.save');
 await expect.poll(async () => Object.values((await call(page, 'document_state')).response.result.document.diagrams[1].flows)[0]?.label).toBe('Order request');

 await clickCanvas(page, 0, 0);
 await clickUi(page, 'dfd.decompose');
 await expect.poll(async () => (await call(page, 'document_state')).response.result.view.diagram).not.toBe(1);
 await clickUi(page, 'dfd.style');
 await clickUi(page, 'dfd.style.yourdon');
 await expect.poll(async () => (await call(page, 'document_state')).response.result.document.style).toBe('yourdon_demarco');
 expect((await call(page, 'validate')).response.result.map(item => item.code)).toContain('dfd_unconnected_boundary');
 await call(page, 'undo');
 expect((await call(page, 'document_state')).response.result.document.style).toBe('gane_sarson');
 await call(page, 'redo');
 expect((await call(page, 'document_state')).response.result.document.style).toBe('yourdon_demarco');
});

test('DFD SDK validates and edits a composite balance, then reopens the saved JSON', async ({ page }) => {
 await ready(page);
 expect((await call(page, 'document_open', { json: example, discard: true })).ok).toBe(true);
 await clickUi(page, 'menu.ai');
 await clickUi(page, 'ai.enabled');
 await clickUi(page, 'ai.origin');
 await page.keyboard.insertText(new URL(page.url()).origin);
 await expect.poll(() => page.evaluate(() => window.erluRuntime.connectionStatus().enabled)).toBe(true);

 const result = await page.evaluate(async () => {
  const { connect } = await import('./sdk/erlu-agent.js');
  const agent = await connect(window, location.origin, { timeout: 5000 });
  try {
   const before = await agent.call('validate');
   await agent.call('objects_delete', { ids: [16] }, before.revision);
   const broken = await agent.call('validate');
   await agent.call('undo');
   const fixed = await agent.call('validate');
   const file = (await agent.call('document_serialize')).result.json;
   await agent.call('document_open', { json: file, discard: true });
   const pdf = (await agent.call('export_pdf', { all: true })).result.bytes;
   const markdown = (await agent.call('export_markdown')).result.text;
   return { before: before.result, broken: broken.result.map(d => d.code), fixed: fixed.result,
    format: JSON.parse(file).format, pdf: String.fromCharCode(...pdf.slice(0, 4)), markdown };
  } finally { agent.close(); }
 });
 expect(result.before).toEqual([]);
 expect(result.broken).toContain('dfd_missing_component');
 expect(result.fixed).toEqual([]);
 expect(result.format).toBe('erlu-dfd');
 expect(result.pdf).toBe('%PDF');
 expect(result.markdown).toContain('Data dictionary');
});

test('browser recovery restores an unsaved DFD and its dirty state', async ({ page }) => {
 await ready(page);
 await call(page, 'document_open', { json: example, discard: true });
 await call(page, 'dfd_edit', { edit: { operation: 'style', style: 'yourdon_demarco' } });
 await expect.poll(() => page.evaluate(() => new Promise((resolve, reject) => {
  const connection = indexedDB.open('erlu', 1);
  connection.onerror = () => reject(connection.error);
  connection.onsuccess = () => {
   const db = connection.result;
   const request = db.transaction('recovery').objectStore('recovery').get('current');
   request.onerror = () => { db.close(); reject(request.error); };
   request.onsuccess = () => { const value = JSON.parse(request.result ?? 'null'); db.close(); resolve(value?.document?.style); };
  };
 }))).toBe('yourdon_demarco');
 page.on('dialog', dialog => dialog.accept());
 await page.reload();
 await page.locator('#recovery').getByRole('button', { name: 'Restore' }).click();
 const state = (await call(page, 'document_state')).response;
 expect(state.result.document.format).toBe('erlu-dfd');
 expect(state.result.document.style).toBe('yourdon_demarco');
 expect(state.dirty).toBe(true);
});
