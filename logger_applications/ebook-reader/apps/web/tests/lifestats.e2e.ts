/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { execFileSync } from 'node:child_process';
import { expect, test, type BrowserContext, type Page } from '@playwright/test';

const SERVER = process.env.LIFESTATS_E2E_URL || 'http://127.0.0.1:3920';
const EMBEDDED = !!process.env.LIFESTATS_E2E_EMBEDDED;
const CONTROL = process.env.LIFESTATS_E2E_CONTROL;
const FIXTURE = process.env.LIFESTATS_E2E_FIXTURE;
const LENGTH = 7908;

type Pairs = [number, number][];

function union(a: Pairs, b: Pairs): Pairs {
  const merged: Pairs = [];
  for (const [start, end] of [...a, ...b].sort((x, y) => x[0] - y[0])) {
    const last = merged[merged.length - 1];
    if (last && start <= last[1]) last[1] = Math.max(last[1], end);
    else merged.push([start, end]);
  }
  return merged;
}

const size = (pairs: Pairs) => pairs.reduce((total, [start, end]) => total + end - start, 0);

/** Every characters credited on the server must be in this union of all browser profiles. */
let everywhere: Pairs = [];
let baseline = 0;
let bookKey: string | undefined;
const liveHits: string[] = [];

function server(action: 'stop' | 'start') {
  if (!CONTROL) throw new Error('LIFESTATS_E2E_CONTROL must name a stop/start script');
  execFileSync(CONTROL, [action], { stdio: 'inherit' });
}

async function credited(day: string): Promise<number> {
  const statistics: { id: number; name: string }[] = await (
    await fetch(`${SERVER}/statistics`)
  ).json();
  const statistic = statistics.find((entry) => entry.name === '[Ebook] Characters Read');
  if (!statistic) return 0;
  const logged: { value?: number } = await (
    await fetch(`${SERVER}/statistics/${statistic.id}/logged/${day}`)
  ).json();
  return logged.value ?? 0;
}

async function profile(
  browser: import('@playwright/test').Browser,
  settings: { viewMode: 'paginated' | 'continuous'; writingMode: 'vertical-rl' | 'horizontal-tb' }
) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  await context.route(/^https?:\/\/(localhost|127\.0\.0\.1):3000\//, (route) => {
    liveHits.push(route.request().url());
    return route.abort();
  });
  await context.addInitScript((values) => {
    localStorage.setItem('viewMode', values.viewMode);
    localStorage.setItem('writingMode', values.writingMode);
  }, settings);
  const page = await context.newPage();
  await page.clock.install({ time: Date.now() });
  return { context, page };
}

async function importBook(page: Page) {
  await page.goto('/manage');
  await page
    .locator('input[type=file][accept*="epub"]')
    .first()
    .setInputFiles(FIXTURE ?? '');
  await expect(page.getByText('読書の小さな旅')).toBeVisible();
}

async function openBook(page: Page) {
  await page.goto('/b?id=1');
  await expect(page.getByText(`/ ${LENGTH}`)).toBeVisible();
  await page.waitForTimeout(500);
}

async function openPanel(page: Page) {
  await page.mouse.click(640, 8);
  await page.locator('[data-lifestats-status]').click();
  await expect(page.locator('[data-lifestats-panel]')).toBeVisible();
}

async function closePanel(page: Page) {
  await page.getByRole('button', { name: 'Close' }).click();
}

async function configure(page: Page) {
  await openPanel(page);
  await page.locator('[data-lifestats-url]').fill('http://user:secret@127.0.0.1:3920');
  await page.locator('[data-lifestats-save]').click();
  await expect(page.locator('[data-lifestats-result]')).toHaveText(
    'Remove the user name and password from the address.'
  );
  await page.locator('[data-lifestats-url]').fill(`${SERVER}/`);
  await page.locator('[data-lifestats-test]').click();
  await expect(page.locator('[data-lifestats-result]')).toContainText('Reachable');
  await page.locator('[data-lifestats-save]').click();
  await expect(page.locator('[data-lifestats-result]')).toHaveText(`Saved ${SERVER}.`);
  await closePanel(page);
  // Upstream ttu stops updating paginated positions after any dialog while Playwright
  // intercepts requests (it does on upstream 68c40f1c too), so read in a fresh page.
  await openBook(page);
  await page.mouse.click(640, 8);
  await expect(page.locator('[data-lifestats-status]')).toHaveAttribute(
    'data-lifestats-status',
    'synced'
  );
  await page.mouse.click(640, 400);
}

/** One page or one scroll step, then 30 seconds of reading time, about 20 characters a second. */
async function advance(page: Page, mode: 'page' | 'scroll', steps: number) {
  for (let i = 0; i < steps; i += 1) {
    if (mode === 'page') await page.keyboard.press('ArrowLeft');
    else await page.mouse.wheel(0, 240);
    await page.waitForTimeout(600);
    await page.clock.fastForward(30_000);
  }
  await page.waitForTimeout(300);
}

async function position(page: Page): Promise<number> {
  return Number((await page.locator('#ttu-page-footer').innerText()).split(' ')[0]);
}

async function stored(page: Page) {
  return page.evaluate(
    () =>
      new Promise<{ book: string; day: string; length: number; spans: [number, number][] }[]>(
        (resolve, reject) => {
          const open = indexedDB.open('ttu-lifestats');
          open.onerror = () => reject(open.error);
          open.onsuccess = () => {
            const request = open.result.transaction('days').objectStore('days').getAll();
            request.onsuccess = () => {
              resolve(request.result);
              open.result.close();
            };
          };
        }
      )
  );
}

async function spansOf(page: Page): Promise<Pairs> {
  const records = await stored(page);
  for (const record of records) {
    expect(record.length).toBe(LENGTH);
    bookKey ??= record.book;
    expect(record.book).toBe(bookKey);
  }
  return records.reduce<Pairs>((all, record) => union(all, record.spans), []);
}

async function drain(page: Page) {
  return page.evaluate(() =>
    (
      globalThis as unknown as { ttuLifestats: { drain(ms: number): Promise<string> } }
    ).ttuLifestats.drain(10_000)
  );
}

function readingDay(at: Date): string {
  const day = new Date(at.getFullYear(), at.getMonth(), at.getDate());
  if (at.getHours() < 4) day.setDate(day.getDate() - 1);
  return [day.getFullYear(), day.getMonth() + 1, day.getDate()]
    .map((part, i) => String(part).padStart(i ? 2 : 4, '0'))
    .join('-');
}

async function today(page: Page) {
  return readingDay(new Date(await page.evaluate(() => Date.now())));
}

async function expectServerMatches(page: Page) {
  everywhere = union(everywhere, await spansOf(page));
  expect(await credited(await today(page))).toBe(baseline + size(everywhere));
}

let contexts: BrowserContext[] = [];

test.describe.configure({ mode: 'serial' });

test.beforeAll(async () => {
  expect(FIXTURE, 'LIFESTATS_E2E_FIXTURE names the fixture EPUB').toBeTruthy();
  const descriptor = await (await fetch(`${SERVER}/integrations/ebook`)).json();
  expect(descriptor.protocol).toBe(1);
  baseline = await credited(readingDay(new Date()));
});

test.afterEach(async () => {
  await Promise.all(contexts.map((context) => context.close()));
  contexts = [];
});

test.afterAll(() => {
  expect(liveHits, 'no request may reach the live server on :3000').toEqual([]);
});

test.describe('browser profile', () => {
  test.skip(EMBEDDED, 'the embedded build runs only its own check');

  test('configures the server in the panel, then paging credits exactly the stored union', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'paginated',
      writingMode: 'vertical-rl'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    await advance(page, 'page', 6);

    expect(await drain(page)).toBe('idle');
    expect(size(await spansOf(page))).toBeGreaterThan(1000);
    await expectServerMatches(page);
    expect(await drain(page)).toBe('idle');
    await expectServerMatches(page);
  });

  test('scrolling in another profile credits only what that profile adds to the union', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'continuous',
      writingMode: 'horizontal-tb'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    await advance(page, 'scroll', 25);

    expect(await drain(page)).toBe('idle');
    expect(size(await spansOf(page))).toBeGreaterThan(1000);
    await expectServerMatches(page);
  });

  test('two tabs reading the same pages never exceed the union', async ({ browser }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'paginated',
      writingMode: 'vertical-rl'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    const second = await context.newPage();
    await openBook(second);
    for (let turn = 0; turn < 8; turn += 1) {
      await page.keyboard.press('ArrowLeft');
      await second.keyboard.press('ArrowLeft');
      await page.waitForTimeout(600);
      await page.clock.fastForward(30_000);
    }
    await page.waitForTimeout(300);

    expect(await drain(page)).toBe('idle');
    expect(await drain(second)).toBe('idle');
    const records = await stored(page);
    expect(new Set(records.map((record) => `${record.book}|${record.day}`)).size).toBe(
      records.length
    );
    await expectServerMatches(page);
  });

  test('while the server is down the indicator waits with a reason and count, then delivers once', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'continuous',
      writingMode: 'horizontal-tb'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    server('stop');
    try {
      await advance(page, 'scroll', 20);
      expect(await drain(page)).toBe('pending');
      await page.mouse.click(640, 8);
      const indicator = page.locator('[data-lifestats-status]');
      await expect(indicator).toHaveAttribute('data-lifestats-status', 'waiting');
      await expect(indicator).toHaveAttribute(
        'title',
        /1 day · [\d,]+ characters waiting\. Lifestats is unreachable/
      );
      await page.locator('[data-lifestats-status]').click();
      await expect(page.locator('[data-lifestats-summary]')).toContainText(
        `Lifestats is unreachable at ${SERVER}.`
      );
      await expect(page.locator('[data-lifestats-pending]')).toHaveText(
        /^1 day · [\d,]+ characters waiting$/
      );
      await page.screenshot({ path: test.info().outputPath('waiting-panel.png') });
    } finally {
      server('start');
    }
    await page.locator('[data-lifestats-retry]').click();
    await expect(page.locator('[data-lifestats-summary]')).toHaveText('Lifestats: up to date');
    await closePanel(page);
    await expectServerMatches(page);
    expect(await drain(page)).toBe('idle');
    await expectServerMatches(page);
  });

  test('a chapter jump, a bookmark return, a reload and a resize credit nothing', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'paginated',
      writingMode: 'vertical-rl'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    await advance(page, 'page', 3);
    const bookmarked = await position(page);
    await page.mouse.click(640, 8);
    await page.getByTitle('Create Bookmark').click();
    await page.mouse.click(640, 400);
    expect(await drain(page)).toBe('idle');
    const before = await spansOf(page);
    expect(before).toEqual([[expect.any(Number), bookmarked]]);

    await page.clock.fastForward(120_000);
    // In vertical writing ttu maps n to the next chapter and m to the previous one.
    await page.keyboard.press('n');
    await page.waitForTimeout(800);
    const chapter = await position(page);
    expect(chapter - bookmarked).toBeGreaterThan(0);
    expect(chapter - bookmarked).toBeLessThan(2700);
    await page.keyboard.press('m');
    await page.waitForTimeout(800);
    await page.keyboard.press('m');
    await page.waitForTimeout(800);
    expect(await position(page)).toBeLessThan(bookmarked);
    await page.clock.fastForward(120_000);
    await page.mouse.click(640, 8);
    await page.getByTitle('Return to Bookmark').click();
    await page.waitForTimeout(800);
    expect(await position(page)).toBe(bookmarked);
    await page.clock.fastForward(120_000);
    await page.reload();
    await expect(page.getByText(`${bookmarked} / ${LENGTH}`)).toBeVisible();
    await page.clock.fastForward(120_000);
    await page.setViewportSize({ width: 900, height: 700 });
    await page.waitForTimeout(1000);
    await page.clock.fastForward(120_000);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.waitForTimeout(1000);
    expect(await drain(page)).toBe('idle');

    expect(await spansOf(page)).toEqual(before);
    await expectServerMatches(page);
  });

  test('a skip far ahead credits nothing, and reading resumes from the new place', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'continuous',
      writingMode: 'horizontal-tb'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await configure(page);
    await advance(page, 'scroll', 3);
    await page.clock.fastForward(120_000);
    const before = await spansOf(page);
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight * 0.8));
    await page.waitForTimeout(500);
    await page.clock.fastForward(60_000);
    expect(await drain(page)).toBe('idle');
    expect(await spansOf(page)).toEqual(before);

    await advance(page, 'scroll', 2);
    expect(await drain(page)).toBe('idle');
    const after = await spansOf(page);
    expect(after.length).toBe(before.length + 1);
    expect(after[after.length - 1][0]).toBeGreaterThan(before[before.length - 1][1] + 2700);
    await expectServerMatches(page);
  });

  test('without a server address, reading is recorded locally and the indicator says so', async ({
    browser
  }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'paginated',
      writingMode: 'vertical-rl'
    });
    contexts.push(context);
    const serverHits: string[] = [];
    page.on('request', (request) => {
      if (request.url().startsWith(SERVER)) serverHits.push(request.url());
    });
    await importBook(page);
    await openBook(page);
    await advance(page, 'page', 4);

    expect(await drain(page)).toBe('pending');
    expect(size(await spansOf(page))).toBeGreaterThan(500);
    await page.mouse.click(640, 8);
    await expect(page.locator('[data-lifestats-status]')).toHaveAttribute(
      'data-lifestats-status',
      'not_configured'
    );
    expect(serverHits).toEqual([]);
  });
});

test.describe('embedded profile', () => {
  test.skip(!EMBEDDED, 'needs the embedded build');

  test('warns that localhost means this phone', async ({ browser }) => {
    const { context, page } = await profile(browser, {
      viewMode: 'paginated',
      writingMode: 'vertical-rl'
    });
    contexts.push(context);
    await importBook(page);
    await openBook(page);
    await openPanel(page);
    await page.locator('[data-lifestats-url]').fill('http://localhost:3000');
    await expect(page.locator('[data-lifestats-hint]')).toContainText(
      'localhost means the phone itself'
    );
    await page.locator('[data-lifestats-url]').fill('http://laptop:3000');
    await expect(page.locator('[data-lifestats-hint]')).toHaveCount(0);
    expect(
      await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length)
    ).toBe(0);
  });
});
