import { test, expect } from '@playwright/test';

const uniqueUser = () => `e2e_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;

test.describe('Auth - Register & Login', () => {
  test('register new user → auto login → projects page', async ({ page }) => {
    const user = uniqueUser();
    await page.goto('/register');
    await page.fill('#reg-username', user);
    await page.fill('#reg-password', 'password123');
    await page.fill('#reg-confirm', 'password123');
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/projects/, { timeout: 10000 });
  });

  test('login existing user → token persists after reload', async ({ page, request }) => {
    const user = uniqueUser();
    const pwd = 'mypassword1';
    await request.post('/api/v1/auth/register', { data: { username: user, password: pwd } });

    await page.goto('/login');
    await page.fill('#username', user);
    await page.fill('#password', pwd);
    await page.click('button[type="submit"]');

    await expect(page).toHaveURL(/\/projects/, { timeout: 10000 });

    await page.reload();
    await expect(page).toHaveURL(/\/projects/);
  });

  test('GitHub button hidden when providers.github=false', async ({ page }) => {
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
    await page.goto('/login');
    await page.fill('#username', 'nonexistent_user_xyz');
    await page.fill('#password', 'wrongpass1');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toBeVisible({ timeout: 10000 });
    await expect(page.locator('[role="alert"]')).toContainText('invalid username or password');
  });

  test('register validation - password too short', async ({ page }) => {
    await page.goto('/register');
    await page.fill('#reg-username', 'testuser');
    await page.fill('#reg-password', 'short');
    await page.fill('#reg-confirm', 'short');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toContainText('密码至少 8 个字符');
  });

  test('register validation - passwords do not match', async ({ page }) => {
    await page.goto('/register');
    await page.fill('#reg-username', 'testuser');
    await page.fill('#reg-password', 'password123');
    await page.fill('#reg-confirm', 'different1');
    await page.click('button[type="submit"]');

    await expect(page.locator('[role="alert"]')).toContainText('两次密码输入不一致');
  });

  test('logged-in user visiting /login redirects to /projects', async ({ page, request }) => {
    const user = uniqueUser();
    const pwd = 'password123';
    const res = await request.post('/api/v1/auth/register', { data: { username: user, password: pwd } });
    const { token, user: userData } = await res.json();

    await page.goto('/login');
    await page.evaluate(({ token, user }) => {
      const state = { state: { token, user: { id: user.id, username: user.username } }, version: 0 };
      localStorage.setItem('omniagp-auth', JSON.stringify(state));
    }, { token, user: userData });

    await page.goto('/login');
    await expect(page).toHaveURL(/\/projects/, { timeout: 10000 });
  });
});
