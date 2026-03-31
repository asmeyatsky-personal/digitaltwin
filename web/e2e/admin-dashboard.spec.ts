import { test, expect } from "@playwright/test";

/**
 * Helper to inject admin auth tokens into localStorage.
 */
async function loginAsAdmin(page: import("@playwright/test").Page) {
  await page.goto("/login");
  await page.evaluate(() => {
    localStorage.setItem("token", "test-token");
    localStorage.setItem("refreshToken", "test-refresh");
    localStorage.setItem(
      "user",
      JSON.stringify({ id: "1", username: "admin", roles: ["Admin", "User"] })
    );
  });
}

test.describe("Admin Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);

    // Mock API calls the admin page makes on load
    await page.route("**/api/moderation/stats", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ data: { pendingCount: 5 } }),
      })
    );
    await page.route("**/api/learning-paths", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ data: { paths: [{ id: 1 }, { id: 2 }] } }),
      })
    );
  });

  test("admin dashboard loads with stat cards", async ({ page }) => {
    await page.goto("/admin");

    await expect(
      page.getByRole("heading", { name: "Dashboard" })
    ).toBeVisible();

    // Verify the four stat cards are displayed
    await expect(page.getByText("Total Users")).toBeVisible();
    await expect(page.getByText("Active Conversations")).toBeVisible();
    await expect(page.getByText("Pending Reports")).toBeVisible();
    await expect(page.getByText("Learning Paths")).toBeVisible();
  });

  test("navigation to moderation works", async ({ page }) => {
    await page.goto("/admin");

    const moderationLink = page.getByRole("link", {
      name: "Review Moderation Queue",
    });
    await expect(moderationLink).toBeVisible();

    // Verify the link points to the correct URL
    await expect(moderationLink).toHaveAttribute("href", "/admin/moderation");
  });

  test("navigation to settings works", async ({ page }) => {
    await page.goto("/admin");

    const settingsLink = page.getByRole("link", {
      name: "System Settings",
    });
    await expect(settingsLink).toBeVisible();

    // Verify the link points to the correct URL
    await expect(settingsLink).toHaveAttribute("href", "/admin/settings");
  });
});
