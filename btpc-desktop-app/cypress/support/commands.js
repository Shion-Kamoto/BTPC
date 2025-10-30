// Custom Cypress commands for BTPC Desktop App testing

// Command to validate BTPC address format
Cypress.Commands.add('validateBtpcAddress', (address) => {
  expect(address).to.match(/^[a-fA-F0-9]{128}$/);
});

// Command to validate BTP amount format
Cypress.Commands.add('validateBtpAmount', (amount) => {
  expect(amount).to.match(/^\d+ base units \(\d+\.\d{8} BTP\)$/);
});

// Command to wait for Tauri backend response
Cypress.Commands.add('waitForTauriResponse', (timeout = 10000) => {
  cy.get('#result-text', { timeout }).should('not.be.empty');
});

// Command to check system status
Cypress.Commands.add('checkSystemStatus', () => {
  cy.get('#node-status').should('be.visible');
  cy.get('#mining-status').should('be.visible');
  cy.get('#wallet-balance').should('be.visible');
  cy.get('#binaries-status').should('be.visible');
});

// Command to verify error handling
Cypress.Commands.add('expectError', (errorMessage) => {
  cy.get('#result-text').should('contain.text', errorMessage);
});

// Command to verify success message
Cypress.Commands.add('expectSuccess', (successMessage) => {
  cy.get('#result-text').should('contain.text', successMessage);
});

// Command to simulate mining completion
Cypress.Commands.add('waitForMiningCompletion', (timeout = 60000) => {
  cy.get('#mining-status', { timeout }).should('contain.text', 'Stopped');
});

// Command to check wallet operations
Cypress.Commands.add('performWalletOperation', (operation) => {
  switch (operation) {
    case 'create':
      cy.get('#create-wallet-btn').click();
      break;
    case 'balance':
      cy.get('#get-balance-btn').click();
      break;
    case 'address':
      cy.get('#get-address-btn').click();
      break;
    default:
      throw new Error(`Unknown wallet operation: ${operation}`);
  }
});

// Command to fill transaction form
Cypress.Commands.add('fillTransactionForm', (address, amount, password) => {
  cy.get('#send-address').clear().type(address);
  cy.get('#send-amount').clear().type(amount.toString());
  cy.get('#send-password').clear().type(password);
});

// Command to verify transaction validation
Cypress.Commands.add('verifyTransactionValidation', (shouldPass = true) => {
  if (shouldPass) {
    cy.expectSuccess('Transaction created successfully');
  } else {
    cy.expectError('Invalid');
  }
});

// Command to monitor resource usage
Cypress.Commands.add('checkPerformance', () => {
  cy.window().then((win) => {
    if (win.performance && win.performance.memory) {
      const memory = win.performance.memory;
      cy.log(`Memory usage: ${Math.round(memory.usedJSHeapSize / 1024 / 1024)}MB`);

      // Warn if memory usage is too high
      if (memory.usedJSHeapSize > 100 * 1024 * 1024) { // 100MB
        cy.log('Warning: High memory usage detected');
      }
    }
  });
});

// Command to test responsive behavior
Cypress.Commands.add('testResponsive', () => {
  // Test mobile viewport
  cy.viewport(375, 667);
  cy.checkSystemStatus();

  // Test tablet viewport
  cy.viewport(768, 1024);
  cy.checkSystemStatus();

  // Return to desktop
  cy.viewport(1000, 700);
  cy.checkSystemStatus();
});

// Command to simulate slow network
Cypress.Commands.add('simulateSlowNetwork', () => {
  cy.intercept('**', { delay: 2000 });
});

// Command for accessibility testing
Cypress.Commands.add('checkAccessibility', () => {
  // Check for proper ARIA labels
  cy.get('button').each(($btn) => {
    cy.wrap($btn).should('have.attr', 'aria-label')
      .or('have.text');
  });

  // Check for proper focus management
  cy.get('input, button, select, textarea').each(($element) => {
    cy.wrap($element).should('be.visible')
      .and('not.have.attr', 'tabindex', '-1');
  });
});

// Command to test keyboard navigation
Cypress.Commands.add('testKeyboardNavigation', () => {
  cy.get('body').tab();
  cy.focused().should('be.visible');

  // Test Enter key activation
  cy.focused().type('{enter}');
});

// Command to clear application state
Cypress.Commands.add('resetAppState', () => {
  // Stop any running processes
  cy.get('#stop-node-btn').then(($btn) => {
    if (!$btn.prop('disabled')) {
      cy.wrap($btn).click();
      cy.waitForTauriResponse();
    }
  });

  cy.get('#stop-mining-btn').then(($btn) => {
    if (!$btn.prop('disabled')) {
      cy.wrap($btn).click();
      cy.waitForTauriResponse();
    }
  });

  // Clear form fields
  cy.get('input[type="text"], input[type="number"], input[type="password"]')
    .clear();
});