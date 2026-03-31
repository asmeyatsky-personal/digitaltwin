import { test, expect } from "@playwright/test";

test.describe("Login Page", () => {
  test("renders login form with username and password fields", async ({
    page,
  }) => {
    await page.goto("/login");

    await expect(
      page.getByRole("heading", { name: "Digital Twin Dashboard" })
    ).toBeVisible();
    await expect(page.getByLabel("Username")).toBeVisible();
    await expect(page.getByLabel("Password")).toBeVisible();
    await expect(page.getByRole("button", { name: "Sign In" })).toBeVisible();
  });

  test("valid credentials redirect to /chat", async ({ page }) => {
    await page.goto("/login");

    await page.getByLabel("Username").fill("admin");
    await page.getByLabel("Password").fill("password123");

    // Mock the login API to return a successful response
    await page.route("**/api/auth/login", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          token: "test-token",
          refreshToken: "test-refresh",
          user: { id: "1", username: "admin", roles: ["Admin", "User"] },
        }),
      })
    );

    await page.getByRole("button", { name: "Sign In" }).click();

    await expect(page).toHaveURL(/\/chat/);
  });

  test("invalid credentials show error message", async ({ page }) => {
    await page.goto("/login");

    await page.getByLabel("Username").fill("wrong");
    await page.getByLabel("Password").fill("wrong");

    // Mock the login API to return an error
    await page.route("**/api/auth/login", (route) =>
      route.fulfill({
        status: 401,
        contentType: "application/json",
        body: JSON.stringify({ message: "Invalid credentials" }),
      })
    );

    await page.getByRole("button", { name: "Sign In" }).click();

    // The error div should appear with an error message
    const errorBanner = page.locator(".bg-red-50");
    await expect(errorBanner).toBeVisible({ timeout: 5000 });
  });

  test("unauthenticated user visiting /chat is redirected to /login", async ({
    page,
  }) => {
    // Ensure no auth tokens exist by clearing storage
    await page.goto("/login");
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });

    await page.goto("/chat");

    // The auth guard should redirect back to /login
    await expect(page).toHaveURL(/\/login/, { timeout: 5000 });
  });
});
