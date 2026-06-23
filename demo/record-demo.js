// Records the aion-context-studio demo: drives the live studio (127.0.0.1:8787) through the eight
// scenes, holding each for its narration's measured duration (demo/durations.json) so the screen
// action tracks the per-scene ElevenLabs audio. Output: demo/playwright-video/*.webm.
const { chromium } = require('playwright');
const fs = require('fs');

const D = JSON.parse(fs.readFileSync('demo/durations.json', 'utf8'));
const BASE = 'http://127.0.0.1:8787';
const ms = (s) => Math.round(s * 1000);

(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 1,
    recordVideo: { dir: 'demo/playwright-video', size: { width: 1440, height: 900 } },
  });
  const page = await context.newPage();
  context.setDefaultTimeout(8000); // fail fast so a missing element can't blow the scene budget

  const POLICY = BASE + '/policy/payments-policy';
  const onPolicy = () => page.url().includes('/policy/payments-policy');
  const ensurePolicy = async () => {
    if (!onPolicy()) await page.goto(POLICY, { waitUntil: 'networkidle' });
  };

  const scene = async (n, fn) => {
    const t0 = Date.now();
    try { await fn(); } catch (e) { console.log(`scene ${n} action error: ${e.message}`); }
    const remaining = ms(D[String(n)]) - (Date.now() - t0);
    if (remaining > 0) await page.waitForTimeout(remaining);
    console.log(`scene ${n}: ${((Date.now() - t0) / 1000).toFixed(1)}s (target ${D[String(n)].toFixed(1)})`);
  };

  const panel = (heading) =>
    page.locator('.panel', { has: page.locator('h2', { hasText: heading }) });
  const reveal = async (loc) => { await loc.scrollIntoViewIfNeeded().catch(() => {}); await page.waitForTimeout(700); };

  // 1 — workspace home: settle on the policy list
  await scene(1, async () => {
    await page.goto(BASE + '/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1500);
    await page.locator('.policy-row').first().hover().catch(() => {});
  });

  // 2 — compose a policy in the builder, then create
  await scene(2, async () => {
    await page.goto(BASE + '/new', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1200);
    await page.locator('#pid').fill('payments-policy');
    await page.waitForTimeout(600);
    await page.locator('.rb-add').click(); // add rule 2
    const r2 = page.locator('.rb-rule').nth(1);
    await r2.locator('.rb-id').fill('deny-large');
    await r2.locator('.rb-cond .f').fill('amount_usd');
    await r2.locator('.rb-op').selectOption('gt');
    await r2.locator('.rb-cond .v').fill('500');
    await r2.getByRole('button', { name: 'Deny' }).click();
    await page.waitForTimeout(1600);
    await page.getByRole('button', { name: 'Create policy' }).click();
    await page.waitForURL('**/policy/payments-policy', { timeout: 9000 }).catch(() => {});
    await ensurePolicy(); // fallback if the client nav didn't land
    await panel('Verification').waitFor({ timeout: 9000 }).catch(() => {});
    await page.waitForTimeout(900);
  });

  // 3 — verification: four guarantees, valid
  await scene(3, async () => { await ensurePolicy(); await reveal(panel('Verification')); });

  // 4 — governance: K-of-N
  await scene(4, async () => { await ensurePolicy(); await reveal(panel('Governance')); });

  // 5 — simulate: an action over the limit → deny, with trace
  await scene(5, async () => {
    await ensurePolicy();
    const sim = panel('Simulate');
    await reveal(sim);
    await sim.locator('textarea').fill('{ "amount_usd": 800 }');
    await page.waitForTimeout(700);
    await sim.getByRole('button', { name: 'Evaluate' }).click();
    await page.waitForTimeout(1500);
  });

  // 6 — audit & export: the operation log + a compliance report
  await scene(6, async () => {
    await ensurePolicy();
    const aud = panel('Audit');
    await reveal(aud);
    await page.waitForTimeout(800);
    await aud.getByRole('button', { name: 'Compliance report' }).click().catch(() => {});
    await page.waitForTimeout(2000);
  });

  // 7 — copilot: ask it to draft a rule; the answer streams in
  await scene(7, async () => {
    await ensurePolicy();
    const cop = panel('Copilot');
    await reveal(cop);
    await cop.locator('input.text').fill('Add a rule that denies refunds over 1000 USD.');
    await page.waitForTimeout(500);
    await cop.getByRole('button', { name: 'Ask' }).click();
    await page.waitForTimeout(11000);
  });

  // 8 — close on the workspace
  await scene(8, async () => {
    await page.goto(BASE + '/', { waitUntil: 'networkidle' });
    await page.waitForTimeout(1500);
  });

  await context.close(); // finalizes the webm
  await browser.close();
  const file = fs.readdirSync('demo/playwright-video').find((f) => f.endsWith('.webm'));
  console.log('VIDEO:' + (file || 'none'));
})();
