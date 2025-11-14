// @ts-check
import { test, expect, _electron as electron } from '@playwright/test';
import path from 'path';

test('create second test wallet testingW2', async () => {
  // Connect to the already running app via CDP
  const executablePath = path.join(process.cwd(), 'target/debug/btpc-desktop-app');

  // For Tauri apps, we need to connect to the WebView
  // The app is already running, so we'll connect to localhost
  // Tauri dev server typically runs on localhost:1420

  const { chromium } = require('playwright');
  const browser = await chromium.connectOverCDP('http://localhost:9222');
  const contexts = browser.contexts();

  if (contexts.length === 0) {
    throw new Error('No browser contexts found. Is the app running?');
  }

  const context = contexts[0];
  const pages = context.pages();

  if (pages.length === 0) {
    throw new Error('No pages found in the browser context.');
  }

  const page = pages[0];

  // Take initial screenshot
  await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/01-initial-state.png' });
  console.log('Initial screenshot saved');

  // Wait for app to be ready
  await page.waitForTimeout(1000);

  // Look for Settings or Wallet creation buttons
  // Based on the guide, we should navigate to Settings or create a new wallet

  // Try to find and click on Settings
  const settingsButton = page.locator('button:has-text("Settings"), a:has-text("Settings"), [data-testid="settings"]');
  const settingsExists = await settingsButton.count() > 0;

  if (settingsExists) {
    await settingsButton.first().click();
    await page.waitForTimeout(1000);
    await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/02-settings.png' });
    console.log('Navigated to Settings');
  }

  // Look for wallet creation option
  // Based on the guide, we need to find "Create a new wallet" or similar
  const createWalletButton = page.locator(
    'button:has-text("Create"), button:has-text("New Wallet"), ' +
    'a:has-text("Create new wallet"), [data-testid="create-wallet"]'
  );

  const createWalletExists = await createWalletButton.count() > 0;

  if (createWalletExists) {
    await createWalletButton.first().click();
    await page.waitForTimeout(1000);
    await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/03-create-wallet.png' });
    console.log('Clicked create wallet button');
  }

  // Look for wallet name input
  const walletNameInput = page.locator('input[type="text"], input[placeholder*="name" i], input[id*="name" i]');
  const nameInputExists = await walletNameInput.count() > 0;

  if (nameInputExists) {
    await walletNameInput.first().fill('testingW2');
    await page.waitForTimeout(500);
    await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/04-wallet-name-entered.png' });
    console.log('Entered wallet name: testingW2');
  }

  // Look for Next/Create/Continue button
  const continueButton = page.locator(
    'button:has-text("Create"), button:has-text("Next"), ' +
    'button:has-text("Continue"), [data-testid="continue"]'
  );

  const continueExists = await continueButton.count() > 0;

  if (continueExists) {
    await continueButton.first().click();
    await page.waitForTimeout(2000);
    await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/05-after-create.png' });
    console.log('Clicked continue/create button');
  }

  // Try to find and capture the wallet address
  // Addresses start with 4... (primary) or 8... (subaddress)
  const addressPattern = /[48][a-zA-Z0-9]{94,}/;
  const pageContent = await page.content();
  const addressMatch = pageContent.match(addressPattern);

  if (addressMatch) {
    console.log('New wallet address:', addressMatch[0]);
  }

  // Take final screenshot
  await page.screenshot({ path: '/home/bob/BTPC/BTPC/tests/screenshots/06-final-state.png' });
  console.log('Final screenshot saved');

  await browser.close();
});
