import { test, expect } from '@playwright/test';

test('smoke: app shell renders', async ({ page }) => {
  await page.goto('/');
  // index.html <title> is "shiori" — one resilient assertion on the app shell.
  await expect(page).toHaveTitle(/shiori/i);
});
