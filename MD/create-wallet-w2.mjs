#!/usr/bin/env node

import { chromium } from '@playwright/test';
import fs from 'fs';
import path from 'path';

async function main() {
  console.log('Attempting to connect to the BTPC desktop app...');

  let browser;
  let page;

  try {
    // Try to connect via CDP (Chrome DevTools Protocol)
    // Tauri apps expose a WebView that can be debugged
    try {
      browser = await chromium.connectOverCDP('http://localhost:9222');
      console.log('Connected via CDP on port 9222');
    } catch (e) {
      console.log('Could not connect on port 9222, trying different approach...');

      // Launch a browser and navigate to the Tauri dev server
      browser = await chromium.launch({ headless: false });
      const context = await browser.newContext();
      page = await context.newPage();

      // Tauri dev server typically runs on localhost:1420
      await page.goto('http://localhost:1420');
      console.log('Navigated to http://localhost:1420');
    }

    if (!page) {
      // Get the first available context and page
      const contexts = browser.contexts();
      if (contexts.length === 0) {
        throw new Error('No browser contexts found');
      }
      const context = contexts[0];
      const pages = context.pages();
      if (pages.length === 0) {
        throw new Error('No pages found');
      }
      page = pages[0];
    }

    // Create screenshots directory
    const screenshotsDir = '/home/bob/BTPC/BTPC/tests/screenshots';
    if (!fs.existsSync(screenshotsDir)) {
      fs.mkdirSync(screenshotsDir, { recursive: true });
    }

    // Step 1: Take initial screenshot
    console.log('\nStep 1: Taking initial screenshot...');
    await page.screenshot({ path: path.join(screenshotsDir, '01-initial-state.png'), fullPage: true });
    console.log('✓ Screenshot saved to 01-initial-state.png');

    // Wait for page to be ready
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Step 2: Try to find the current state
    console.log('\nStep 2: Analyzing current page state...');
    const pageTitle = await page.title();
    console.log('Page title:', pageTitle);

    // Get all buttons and links on the page
    const buttons = await page.locator('button').allTextContents();
    console.log('Available buttons:', buttons);

    const links = await page.locator('a').allTextContents();
    console.log('Available links:', links);

    // Step 3: Look for wallet-related navigation
    console.log('\nStep 3: Looking for wallet creation options...');

    // Common selectors for wallet management
    const selectors = [
      'text=Settings',
      'text=Wallet',
      'text=Create',
      'text=New Wallet',
      '[data-testid="settings"]',
      '[data-testid="wallet"]',
      '[data-testid="create-wallet"]',
      'button:has-text("Wallet")',
      'button:has-text("Settings")',
      'a[href*="settings"]',
      'a[href*="wallet"]',
    ];

    let foundButton = null;
    for (const selector of selectors) {
      try {
        const element = page.locator(selector).first();
        if (await element.isVisible({ timeout: 1000 })) {
          foundButton = selector;
          console.log(`✓ Found clickable element: ${selector}`);
          break;
        }
      } catch (e) {
        // Element not found or not visible
      }
    }

    if (foundButton) {
      console.log(`\nStep 4: Clicking on "${foundButton}"...`);
      await page.locator(foundButton).first().click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: path.join(screenshotsDir, '02-after-click.png'), fullPage: true });
      console.log('✓ Screenshot saved to 02-after-click.png');
    }

    // Step 5: Look for create wallet option
    console.log('\nStep 5: Looking for "Create Wallet" option...');
    const createSelectors = [
      'text=Create new wallet',
      'text=Create wallet',
      'text=New wallet',
      'text=Create',
      'button:has-text("Create")',
      '[data-testid="create-wallet"]',
    ];

    let foundCreate = null;
    for (const selector of createSelectors) {
      try {
        const element = page.locator(selector).first();
        if (await element.isVisible({ timeout: 1000 })) {
          foundCreate = selector;
          console.log(`✓ Found create wallet element: ${selector}`);
          break;
        }
      } catch (e) {
        // Element not found
      }
    }

    if (foundCreate) {
      console.log(`\nStep 6: Clicking on "${foundCreate}"...`);
      await page.locator(foundCreate).first().click();
      await page.waitForTimeout(1000);
      await page.screenshot({ path: path.join(screenshotsDir, '03-create-wallet-form.png'), fullPage: true });
      console.log('✓ Screenshot saved to 03-create-wallet-form.png');
    }

    // Step 7: Look for wallet name input
    console.log('\nStep 7: Looking for wallet name input field...');
    const nameInputSelectors = [
      'input[type="text"]',
      'input[placeholder*="name" i]',
      'input[name*="name" i]',
      'input[id*="name" i]',
      '[data-testid="wallet-name"]',
    ];

    let foundInput = null;
    for (const selector of nameInputSelectors) {
      try {
        const element = page.locator(selector).first();
        if (await element.isVisible({ timeout: 1000 })) {
          foundInput = selector;
          console.log(`✓ Found input field: ${selector}`);
          break;
        }
      } catch (e) {
        // Element not found
      }
    }

    if (foundInput) {
      console.log('\nStep 8: Entering wallet name "testingW2"...');
      await page.locator(foundInput).first().fill('testingW2');
      await page.waitForTimeout(500);
      await page.screenshot({ path: path.join(screenshotsDir, '04-wallet-name-entered.png'), fullPage: true });
      console.log('✓ Screenshot saved to 04-wallet-name-entered.png');
      console.log('✓ Wallet name entered: testingW2');
    }

    // Step 9: Look for submit/continue button
    console.log('\nStep 9: Looking for submit/continue button...');
    const submitSelectors = [
      'button:has-text("Create")',
      'button:has-text("Next")',
      'button:has-text("Continue")',
      'button:has-text("Submit")',
      'button[type="submit"]',
      '[data-testid="submit"]',
      '[data-testid="continue"]',
    ];

    let foundSubmit = null;
    for (const selector of submitSelectors) {
      try {
        const element = page.locator(selector).first();
        if (await element.isVisible({ timeout: 1000 })) {
          foundSubmit = selector;
          console.log(`✓ Found submit button: ${selector}`);
          break;
        }
      } catch (e) {
        // Element not found
      }
    }

    if (foundSubmit) {
      console.log(`\nStep 10: Clicking "${foundSubmit}"...`);
      await page.locator(foundSubmit).first().click();
      await page.waitForTimeout(2000);
      await page.screenshot({ path: path.join(screenshotsDir, '05-after-submit.png'), fullPage: true });
      console.log('✓ Screenshot saved to 05-after-submit.png');
    }

    // Step 11: Look for the wallet address
    console.log('\nStep 11: Searching for wallet address...');
    await page.waitForTimeout(2000);

    const pageContent = await page.content();
    const addressPattern = /[48][a-zA-Z0-9]{94,}/g;
    const addresses = pageContent.match(addressPattern);

    if (addresses && addresses.length > 0) {
      console.log('\n✓ Found wallet address(es):');
      addresses.forEach((addr, i) => {
        console.log(`  ${i + 1}. ${addr}`);
      });

      // Save the address to a file
      const addressFile = '/home/bob/BTPC/BTPC/tests/testingW2-address.txt';
      fs.writeFileSync(addressFile, addresses[0]);
      console.log(`✓ Address saved to ${addressFile}`);
    } else {
      console.log('⚠ No address found yet. Taking final screenshot...');
    }

    // Final screenshot
    await page.screenshot({ path: path.join(screenshotsDir, '06-final-state.png'), fullPage: true });
    console.log('✓ Final screenshot saved to 06-final-state.png');

    console.log('\n=== Process Complete ===');
    console.log('Please check the screenshots in tests/screenshots/ directory');

  } catch (error) {
    console.error('Error:', error);
  } finally {
    if (browser) {
      // Don't close the browser if we're connected to the app
      if (!browser.isConnected || browser.isConnected()) {
        console.log('\nKeeping browser/app open...');
      }
    }
  }
}

main().catch(console.error);
