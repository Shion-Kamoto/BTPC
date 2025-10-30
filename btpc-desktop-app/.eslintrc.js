module.exports = {
  env: {
    browser: true,
    es2021: true,
    node: true,
    jest: true,
    'cypress/globals': true
  },
  extends: [
    'eslint:recommended'
  ],
  plugins: [
    'cypress'
  ],
  parserOptions: {
    ecmaVersion: 12,
    sourceType: 'module'
  },
  rules: {
    // Error Prevention
    'no-console': 'warn',
    'no-debugger': 'error',
    'no-alert': 'error',
    'no-unused-vars': 'error',
    'no-undef': 'error',
    'no-unreachable': 'error',
    'no-duplicate-case': 'error',
    'no-empty': 'error',

    // Code Quality
    'prefer-const': 'error',
    'no-var': 'error',
    'eqeqeq': 'error',
    'curly': 'error',
    'brace-style': ['error', '1tbs'],
    'comma-dangle': ['error', 'never'],
    'indent': ['error', 2],
    'quotes': ['error', 'single'],
    'semi': ['error', 'always'],

    // Security
    'no-eval': 'error',
    'no-implied-eval': 'error',
    'no-new-func': 'error',
    'no-script-url': 'error',

    // Best Practices
    'default-case': 'error',
    'no-fallthrough': 'error',
    'no-multi-spaces': 'error',
    'no-multiple-empty-lines': ['error', { max: 2 }],
    'no-trailing-spaces': 'error',
    'space-before-function-paren': ['error', 'never'],
    'keyword-spacing': 'error',
    'space-infix-ops': 'error',

    // Cypress specific rules
    'cypress/no-assigning-return-values': 'error',
    'cypress/no-unnecessary-waiting': 'warn',
    'cypress/assertion-before-screenshot': 'warn',
    'cypress/no-force': 'warn'
  },
  overrides: [
    {
      files: ['cypress/**/*.js'],
      rules: {
        'no-unused-expressions': 'off',
        'cypress/no-unnecessary-waiting': 'off' // Allow waits in E2E tests
      }
    },
    {
      files: ['tests/**/*.js'],
      env: {
        jest: true
      },
      rules: {
        'no-console': 'off' // Allow console in tests
      }
    }
  ],
  globals: {
    '__TAURI__': 'readonly',
    'Cypress': 'readonly',
    'cy': 'readonly',
    'expect': 'readonly',
    'test': 'readonly',
    'describe': 'readonly',
    'beforeEach': 'readonly',
    'afterEach': 'readonly',
    'beforeAll': 'readonly',
    'afterAll': 'readonly',
    'jest': 'readonly'
  }
};