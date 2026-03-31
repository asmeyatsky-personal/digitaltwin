using Xunit;
using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using FluentAssertions;
using DigitalTwin.API.Controllers;
using DigitalTwin.Core.Entities;
using DigitalTwin.IntegrationTests.Fixtures;

namespace DigitalTwin.IntegrationTests.Tests;

public class ModerationTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public ModerationTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task ReportContent_ReturnsCreatedReport()
    {
        // Arrange
        var request = new ReportContentRequest
        {
            ContentType = ContentType.Post,
            ContentId = Guid.NewGuid(),
            Reason = ReportReason.Spam,
            Description = "This looks like spam content"
        };

        // Act
        var response = await _client.PostAsync("/api/moderation/report",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task GetPendingReports_AsRegularUser_ReturnsForbidden()
    {
        // Act -- the default test user has "User" role, not "admin" or "moderator"
        var response = await _client.GetAsync("/api/moderation/reports");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Forbidden);
    }

    [Fact]
    public async Task GetModerationStats_AsAdmin_ReturnsStats()
    {
        // Arrange -- create a client with admin role
        var adminClient = _factory.CreateAuthenticatedClient("admin-user-id", "admin", new[] { "admin" });

        // Act
        var response = await adminClient.GetAsync("/api/moderation/stats");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task Moderation_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var reportResponse = await unauthenticatedClient.PostAsync("/api/moderation/report",
            new StringContent(JsonSerializer.Serialize(new ReportContentRequest
            {
                ContentType = ContentType.Post,
                ContentId = Guid.NewGuid(),
                Reason = ReportReason.Spam
            }), Encoding.UTF8, "application/json"));
        reportResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var reportsResponse = await unauthenticatedClient.GetAsync("/api/moderation/reports");
        reportsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var statsResponse = await unauthenticatedClient.GetAsync("/api/moderation/stats");
        statsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
