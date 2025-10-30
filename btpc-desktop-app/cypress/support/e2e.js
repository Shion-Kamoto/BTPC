// Cypress E2E Support File for BTPC Desktop App

import './commands';

// Global configuration
Cypress.on('uncaught:exception', (err, runnable) => {
  // Prevent Cypress from failing on uncaught exceptions from the app
  if (err.message.includes('ResizeObserver loop limit exceeded') ||
      err.message.includes('Non-Error promise rejection captured')) {
    return false;
  }
  return true;
});

// Custom commands for BTPC testing
Cypress.Commands.add('waitForAppLoad', () => {
  cy.get('#status-container', { timeout: 10000 }).should('be.visible');
  cy.get('#node-status').should('contain.text', 'Stopped');
});

Cypress.Commands.add('setupBtpc', () => {
  cy.get('#setup-btn').click();
  cy.contains('BTPC setup completed', { timeout: 30000 }).should('be.visible');
});

Cypress.Commands.add('createTestWallet', () => {
  cy.get('#create-wallet-btn').click();
  cy.contains('Wallet created successfully', { timeout: 15000 }).should('be.visible');
});

Cypress.Commands.add('loginTestUser', () => {
  cy.get('#username').type(Cypress.env('test_user'));
  cy.get('#password').type(Cypress.env('test_password'));
  cy.get('#login-btn').click();
  cy.contains('Successfully logged', { timeout: 5000 });
});

Cypress.Commands.add('startTestNode', () => {
  cy.get('#start-node-btn').click();
  cy.contains('Node started successfully', { timeout: 20000 }).should('be.visible');
  cy.get('#node-status').should('contain.text', 'Running');
});

Cypress.Commands.add('stopTestNode', () => {
  cy.get('#stop-node-btn').click();
  cy.contains('Node stopped successfully', { timeout: 10000 }).should('be.visible');
  cy.get('#node-status').should('contain.text', 'Stopped');
});

Cypress.Commands.add('startTestMining', (blocks = 5) => {
  cy.get('#mining-address').clear().type(Cypress.env('test_address'));
  cy.get('#mining-blocks').clear().type(blocks.toString());
  cy.get('#start-mining-btn').click();
  cy.contains('Mining started', { timeout: 10000 }).should('be.visible');
});

Cypress.Commands.add('stopTestMining', () => {
  cy.get('#stop-mining-btn').click();
  cy.contains('Mining stopped successfully', { timeout: 10000 }).should('be.visible');
});

Cypress.Commands.add('checkWalletBalance', () => {
  cy.get('#get-balance-btn').click();
  cy.get('#wallet-balance').should('not.contain.text', 'Unknown');
});

Cypress.Commands.add('sendTestTransaction', (amount = 1.0) => {
  cy.get('#send-address').clear().type(Cypress.env('test_address'));
  cy.get('#send-amount').clear().type(amount.toString());
  cy.get('#send-password').clear().type(Cypress.env('test_password'));
  cy.get('#send-btn').click();
});

// Global beforeEach for all tests
beforeEach(() => {
  // Clear any previous state
  cy.clearLocalStorage();
  cy.clearCookies();

  // Visit the app
  cy.visit('/');

  // Wait for app to load
  cy.waitForAppLoad();
});

// Global afterEach for cleanup
afterEach(() => {
  // Take screenshot on failure
  if (cy.state('test').state === 'failed') {
    cy.screenshot();
  }
});