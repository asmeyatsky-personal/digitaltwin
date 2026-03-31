using Xunit;
using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using FluentAssertions;
using DigitalTwin.API.Controllers;
using DigitalTwin.IntegrationTests.Fixtures;

namespace DigitalTwin.IntegrationTests.Tests;

public class SubscriptionTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public SubscriptionTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task GetTiers_ReturnsAvailableTiers()
    {
        // Act -- tiers endpoint is [AllowAnonymous]
        var unauthenticatedClient = _factory.CreateClient();
        var response = await unauthenticatedClient.GetAsync("/api/subscription/tiers");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetArrayLength().Should().BeGreaterThanOrEqualTo(3);

        // Verify tier structure
        var firstTier = data[0];
        firstTier.TryGetProperty("tier", out _).Should().BeTrue();
        firstTier.TryGetProperty("name", out _).Should().BeTrue();
        firstTier.TryGetProperty("price", out _).Should().BeTrue();
        firstTier.TryGetProperty("features", out _).Should().BeTrue();
    }

    [Fact]
    public async Task GetCurrentSubscription_ReturnsUserSubscription()
    {
        // Act
        var response = await _client.GetAsync("/api/subscription/current");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("tier", out _).Should().BeTrue();
        data.TryGetProperty("status", out _).Should().BeTrue();
    }

    [Fact]
    public async Task CreateCheckout_WithInvalidTier_ReturnsBadRequest()
    {
        // Arrange
        var request = new CheckoutRequest
        {
            Tier = "nonexistent_tier",
            Platform = "web"
        };

        // Act
        var response = await _client.PostAsync("/api/subscription/checkout",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.BadRequest);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeFalse();
    }

    [Fact]
    public async Task Subscription_ProtectedEndpoints_RequireAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var currentResponse = await unauthenticatedClient.GetAsync("/api/subscription/current");
        currentResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var checkoutResponse = await unauthenticatedClient.PostAsync("/api/subscription/checkout",
            new StringContent(JsonSerializer.Serialize(new CheckoutRequest
            {
                Tier = "plus",
                Platform = "web"
            }), Encoding.UTF8, "application/json"));
        checkoutResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var cancelResponse = await unauthenticatedClient.PostAsync("/api/subscription/cancel", null);
        cancelResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
