import { test, expect } from "@playwright/test";

/**
 * Helper to inject auth tokens into localStorage so the chat page
 * does not redirect to /login.
 */
async function loginViaStorage(page: import("@playwright/test").Page) {
  await page.goto("/login");
  await page.evaluate(() => {
    localStorage.setItem("token", "test-token");
    localStorage.setItem("refreshToken", "test-refresh");
    localStorage.setItem(
      "user",
      JSON.stringify({ id: "1", username: "testuser", roles: ["User"] })
    );
  });
}

test.describe("Chat Page", () => {
  test.beforeEach(async ({ page }) => {
    await loginViaStorage(page);
  });

  test("chat page loads with welcome message", async ({ page }) => {
    await page.goto("/chat");

    // The welcome heading should greet the user
    await expect(page.locator("text=Digital Twin")).toBeVisible();
    await expect(
      page.getByText("I'm your Digital Twin companion")
    ).toBeVisible();
  });

  test("can type and send a message", async ({ page }) => {
    // Mock the chat API
    await page.route("**/api/chat", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ response: "Hello! How can I help you?" }),
      })
    );

    await page.goto("/chat");

    const textarea = page.getByPlaceholder("Type a message...");
    await expect(textarea).toBeVisible();

    await textarea.fill("Hello there");
    await page.getByRole("button", { name: /send/i }).click();

    // The user message should appear in the conversation
    await expect(page.getByText("Hello there")).toBeVisible();
  });

  test("assistant response appears in conversation", async ({ page }) => {
    // Mock the chat API with a known response
    await page.route("**/api/chat", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          response: "I am doing well, thank you for asking!",
        }),
      })
    );

    await page.goto("/chat");

    const textarea = page.getByPlaceholder("Type a message...");
    await textarea.fill("How are you?");
    await page.getByRole("button", { name: /send/i }).click();

    // Both the user message and the assistant response should be visible
    await expect(page.getByText("How are you?")).toBeVisible();
    await expect(
      page.getByText("I am doing well, thank you for asking!")
    ).toBeVisible({ timeout: 5000 });
  });
});
