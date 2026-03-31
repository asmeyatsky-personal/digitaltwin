import { test, expect } from "@playwright/test";

const MOCK_GROUPS = {
  data: {
    groups: [
      {
        id: "g1",
        name: "Anxiety Support",
        description: "A safe space to discuss anxiety",
        category: "Support",
        memberCount: 42,
        createdAt: "2025-01-15T00:00:00Z",
      },
      {
        id: "g2",
        name: "Mindfulness Practice",
        description: "Daily mindfulness exercises",
        category: "Mindfulness",
        memberCount: 128,
        createdAt: "2025-02-01T00:00:00Z",
      },
      {
        id: "g3",
        name: "Book Club",
        description: "Monthly book discussions",
        category: "Interest",
        memberCount: 35,
        createdAt: "2025-03-10T00:00:00Z",
      },
    ],
    totalCount: 3,
  },
};

/**
 * Helper to inject auth tokens into localStorage.
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

test.describe("Community Page", () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);

    // Mock the community groups API
    await page.route("**/api/community/groups*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(MOCK_GROUPS),
      })
    );
  });

  test("community page loads with heading", async ({ page }) => {
    await page.goto("/community");

    await expect(
      page.getByRole("heading", { name: "Community" })
    ).toBeVisible();
    await expect(
      page.getByText("Browse community groups and posts")
    ).toBeVisible();
  });

  test("groups are displayed in a table", async ({ page }) => {
    await page.goto("/community");

    // Wait for groups to load
    await expect(page.getByText("Anxiety Support")).toBeVisible({
      timeout: 5000,
    });
    await expect(page.getByText("Mindfulness Practice")).toBeVisible();
    await expect(page.getByText("Book Club")).toBeVisible();

    // Verify table headers
    await expect(page.getByText("Name")).toBeVisible();
    await expect(page.getByText("Category")).toBeVisible();
    await expect(page.getByText("Members")).toBeVisible();
  });

  test("can search for groups", async ({ page }) => {
    await page.goto("/community");

    // Wait for initial load
    await expect(page.getByText("Anxiety Support")).toBeVisible({
      timeout: 5000,
    });

    // Mock search results
    await page.route("**/api/community/groups*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          data: {
            groups: [MOCK_GROUPS.data.groups[0]],
            totalCount: 1,
          },
        }),
      })
    );

    const searchInput = page.getByPlaceholder("Search groups...");
    await searchInput.fill("Anxiety");
    await page.getByRole("button", { name: "Search" }).click();

    // After search, only the matching group should appear
    await expect(page.getByText("Anxiety Support")).toBeVisible();
  });
});
