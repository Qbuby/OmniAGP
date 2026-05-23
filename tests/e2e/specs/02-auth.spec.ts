import { test, expect } from '@playwright/test';

test.describe('Auth - Register & Login', () => {
  test('register new user → auto login → projects page', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );
    await page.route('**/api/v1/auth/register', (route) =>
      route.fulfill({
        json: { token: 'test-token-123', user: { id: 'u1', username: 'newuser' } },
      })
    );
    await page.route('**/api/v1/projects*', (route) =>
      route.fulfill({ json: { projects: [], total: 0 } })
    );

    await page.goto('/register');
    await page.fill('#reg-username', 'newuser');
    await page.fill('#reg-password', 'password123');
    await page.fill('#reg-confirm', 'password123');
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/projects/);
  });

  test('login existing user → token persists after reload', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );
    await page.route('**/api/v1/auth/login', (route) =>
      route.fulfill({
        json: { token: 'persist-token', user: { id: 'u2', username: 'existinguser' } },
      })
    );
    await page.route('**/api/v1/projects*', (route) =>
      route.fulfill({ json: { projects: [], total: 0 } })
    );

    await page.goto('/login');
    await page.fill('#username', 'existinguser');
    await page.fill('#password', 'mypassword1');
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/projects/);

    // Reload and verify still logged in
    await page.reload();
    await expect(page).toHaveURL(/\/projects/);
  });

  test('GitHub button hidden when providers.github=false', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );

    await page.goto('/login');
    await expect(page.locator('text=使用 GitHub 登录')).not.toBeVisible();
  });

  test('GitHub button visible when providers.github=true', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: true } })
    );

    await page.goto('/login');
    await expect(page.locator('text=使用 GitHub 登录')).toBeVisible();
  });

  test('login error displays message', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );
    await page.route('**/api/v1/auth/login', (route) =>
      route.fulfill({ status: 401, json: { error: 'unauthorized', message: '用户名或密码错误' } })
    );

    await page.goto('/login');
    await page.fill('#username', 'baduser');
    await page.fill('#password', 'wrongpass');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toContainText('用户名或密码错误');
  });

  test('register validation - password too short', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );

    await page.goto('/register');
    await page.fill('#reg-username', 'testuser');
    await page.fill('#reg-password', 'short');
    await page.fill('#reg-confirm', 'short');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toContainText('密码至少 8 个字符');
  });

  test('register validation - passwords do not match', async ({ page }) => {
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );

    await page.goto('/register');
    await page.fill('#reg-username', 'testuser');
    await page.fill('#reg-password', 'password123');
    await page.fill('#reg-confirm', 'different1');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toContainText('两次密码输入不一致');
  });

  test('logged-in user visiting /login redirects to /projects', async ({ page }) => {
    // Pre-set auth state in localStorage
    await page.goto('/login');
    await page.evaluate(() => {
      const state = { state: { token: 'existing-token', user: { id: 'u1', username: 'user1' } }, version: 0 };
      localStorage.setItem('omniagp-auth', JSON.stringify(state));
    });
    await page.route('**/api/v1/auth/providers', (route) =>
      route.fulfill({ json: { local: true, github: false } })
    );
    await page.route('**/api/v1/projects*', (route) =>
      route.fulfill({ json: { projects: [], total: 0 } })
    );

    await page.goto('/login');
    await expect(page).toHaveURL(/\/projects/);
  });
});
