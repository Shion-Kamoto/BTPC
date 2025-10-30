const { defineConfig } = require('cypress');

module.exports = defineConfig({
  e2e: {
    baseUrl: 'http://localhost:1420', // Tauri dev server default port
    supportFile: 'cypress/support/e2e.js',
    specPattern: 'cypress/e2e/**/*.cy.js',
    videosFolder: 'cypress/videos',
    screenshotsFolder: 'cypress/screenshots',
    video: true,
    screenshot: true,
    viewportWidth: 1000,
    viewportHeight: 700,
    defaultCommandTimeout: 10000,
    requestTimeout: 10000,
    responseTimeout: 10000,
    setupNodeEvents(on, config) {
      // implement node event listeners here
      on('task', {
        log(message) {
          console.log(message);
          return null;
        }
      });

      // Handle Tauri app lifecycle
      on('before:browser:launch', (browser = {}, launchOptions) => {
        console.log('Launching browser:', browser.name);
        return launchOptions;
      });
    },
  },

  component: {
    devServer: {
      framework: 'create-react-app',
      bundler: 'webpack',
    },
  },

  env: {
    // Test configuration
    test_user: 'testuser',
    test_password: 'testpassword123',
    test_address: 'a1b2c3d4e5f6789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890',
  },

  // Retry configuration
  retries: {
    runMode: 2,
    openMode: 0,
  },

  // Performance settings
  numTestsKeptInMemory: 10,
  experimentalMemoryManagement: true,
});