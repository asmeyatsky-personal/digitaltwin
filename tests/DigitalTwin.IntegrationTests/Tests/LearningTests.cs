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

public class LearningTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public LearningTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task GetPaths_ReturnsLearningPaths()
    {
        // Act
        var response = await _client.GetAsync("/api/learning/paths");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();
        root.GetProperty("data").TryGetProperty("paths", out _).Should().BeTrue();
    }

    [Fact]
    public async Task GetPaths_FilteredByCategory_ReturnsOk()
    {
        // Act
        var response = await _client.GetAsync("/api/learning/paths?category=Mindfulness");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task GetProgress_ReturnsUserProgress()
    {
        // Act
        var response = await _client.GetAsync("/api/learning/progress");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();
        root.GetProperty("data").TryGetProperty("progress", out _).Should().BeTrue();
    }

    [Fact]
    public async Task GetSuggested_ReturnsSuggestedPath()
    {
        // Act
        var response = await _client.GetAsync("/api/learning/suggested");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task Learning_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var pathsResponse = await unauthenticatedClient.GetAsync("/api/learning/paths");
        pathsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var progressResponse = await unauthenticatedClient.GetAsync("/api/learning/progress");
        progressResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var suggestedResponse = await unauthenticatedClient.GetAsync("/api/learning/suggested");
        suggestedResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
