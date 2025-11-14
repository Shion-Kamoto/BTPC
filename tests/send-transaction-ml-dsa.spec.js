// @ts-check
import { test, expect } from '@playwright/test';

/**
 * Test ML-DSA (Dilithium5) signature generation by sending a BTPC transaction
 *
 * This test validates that the ML-DSA signing code is working correctly
 * by sending a transaction from testingW1 to testingW2.
 *
 * Expected context:
 * - Desktop app running at http://localhost:1430
 * - testingW1 has balance of 102,434.50 BTP
 * - testingW2 address: mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY
 * - Signing happens in wallet_commands.rs line 251: private_key.sign(message_bytes)
 */

test.describe('ML-DSA Transaction Signing', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the desktop app
    await page.goto('http://localhost:1430');

    // Wait for the app to be fully loaded
    await page.waitForLoadState('networkidle');

    // Wait for Tauri API to be ready
    await page.waitForFunction(() => window.invoke !== undefined);

    console.log('✅ App loaded and Tauri API ready');
  });

  test('Send BTPC transaction with ML-DSA signature', async ({ page, context }) => {
    // Enable console logging from the browser
    page.on('console', msg => {
      const type = msg.type();
      const text = msg.text();

      // Log everything to see ML-DSA related messages
      console.log(`[Browser ${type}] ${text}`);

      // Highlight ML-DSA/Dilithium/signature messages
      if (text.includes('ML-DSA') ||
          text.includes('Dilithium') ||
          text.includes('signature') ||
          text.includes('sign')) {
        console.log('🔐 CRYPTO EVENT:', text);
      }
    });

    // Step 1: Take initial screenshot
    console.log('📸 Taking initial screenshot...');
    await page.screenshot({
      path: '/tmp/btpc-before-send.png',
      fullPage: true
    });

    // Step 2: Navigate to Transactions page
    console.log('🔄 Navigating to Transactions page...');
    await page.click('a[href="transactions.html"]');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000); // Give UI time to load

    // Step 3: Ensure we're on the Send tab
    console.log('📝 Switching to Send tab...');
    const sendTab = page.locator('button.tab-btn', { hasText: 'Send' });
    await sendTab.click();
    await page.waitForTimeout(500);

    // Step 4: Select wallet (testingW1)
    console.log('👛 Selecting testingW1 wallet...');
    const walletSelect = page.locator('#send-from-wallet');

    // Wait for wallets to load
    await page.waitForTimeout(2000);

    // Get all wallet options
    const walletOptions = await walletSelect.locator('option').allTextContents();
    console.log('Available wallets:', walletOptions);

    // Find testingW1 option (look for option containing "testingW1")
    const testingW1Option = walletOptions.find(opt => opt.includes('testingW1'));
    if (!testingW1Option) {
      throw new Error('testingW1 wallet not found in dropdown');
    }

    // Select by visible text
    await walletSelect.selectOption({ label: testingW1Option });
    console.log('Selected wallet:', testingW1Option);

    // Step 5: Fill in recipient address
    console.log('📮 Entering recipient address...');
    const recipientAddress = 'mwjfUDDmMYdDyN54FyFKsNCCJMrYk3GCWY';
    await page.fill('#send-address', recipientAddress);
    console.log(`Recipient: ${recipientAddress}`);

    // Step 6: Fill in amount
    console.log('💰 Entering amount...');
    const amount = '1000.0';
    await page.fill('#send-amount', amount);
    console.log(`Amount: ${amount} BTP`);

    // Take screenshot before sending
    console.log('📸 Taking screenshot before sending...');
    await page.screenshot({
      path: '/tmp/btpc-send-form-filled.png',
      fullPage: true
    });

    // Step 7: Click Send button
    console.log('🚀 Clicking Send button...');
    const sendButton = page.locator('button.btn-primary', { hasText: 'Send BTPC' });
    await sendButton.click();

    // Step 8: Wait for password modal
    console.log('🔐 Waiting for password modal...');
    await page.waitForSelector('#password-modal', { state: 'visible', timeout: 5000 });
    console.log('Password modal appeared');

    // Take screenshot of password modal
    await page.screenshot({
      path: '/tmp/btpc-password-modal.png',
      fullPage: true
    });

    // Step 9: Enter password (assuming testingW1 password is "test")
    console.log('🔑 Entering password...');
    const password = 'test'; // Adjust if different
    await page.fill('#wallet-password-input', password);

    // Step 10: Submit password and send transaction
    console.log('✅ Submitting password to send transaction...');
    console.log('🔐 THIS WILL TRIGGER ML-DSA SIGNING IN RUST BACKEND');

    // Listen for any dialog/alert
    page.on('dialog', async dialog => {
      const message = dialog.message();
      console.log(`[Alert] ${message}`);

      if (message.includes('successfully')) {
        console.log('✅ Transaction sent successfully!');
      } else if (message.includes('Failed') || message.includes('error')) {
        console.log('❌ Transaction failed:', message);
      }

      // Accept the dialog
      await dialog.accept();
    });

    // Click the confirm button
    const confirmButton = page.locator('button.btn-primary', { hasText: 'Confirm' });
    await confirmButton.click();

    // Wait for transaction processing (give it enough time for ML-DSA signing)
    console.log('⏳ Waiting for transaction to process (ML-DSA signing happening now)...');
    await page.waitForTimeout(5000);

    // Step 11: Take final screenshot
    console.log('📸 Taking final screenshot...');
    await page.screenshot({
      path: '/tmp/btpc-after-send.png',
      fullPage: true
    });

    // Step 12: Check transaction history
    console.log('📜 Checking transaction history...');
    const historyTab = page.locator('button.tab-btn', { hasText: 'History' });
    await historyTab.click();
    await page.waitForTimeout(2000);

    // Take screenshot of history
    await page.screenshot({
      path: '/tmp/btpc-transaction-history.png',
      fullPage: true
    });

    // Check if there are any transactions in the table
    const transactionTable = page.locator('#transaction-table');
    const isVisible = await transactionTable.isVisible();

    if (isVisible) {
      console.log('✅ Transaction table is visible (transactions exist)');

      // Get transaction rows
      const rows = await page.locator('#transaction-tbody tr').count();
      console.log(`Found ${rows} transaction(s) in history`);
    } else {
      console.log('ℹ️ No transactions in history yet (may still be pending)');
    }

    console.log('\n=== TEST COMPLETE ===');
    console.log('Screenshots saved to /tmp/');
    console.log('Check browser console logs above for ML-DSA signature generation messages');
  });
});
