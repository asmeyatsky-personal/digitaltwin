using Xunit;
using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using FluentAssertions;
using DigitalTwin.API.Controllers;
using DigitalTwin.Core.Entities;
using DigitalTwin.Core.Enums;
using DigitalTwin.IntegrationTests.Fixtures;

namespace DigitalTwin.IntegrationTests.Tests;

public class CreativeTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public CreativeTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task CreateWork_ReturnsCreatedWork()
    {
        // Arrange
        var request = new CreateWorkRequest
        {
            Type = CreativeWorkType.Poem,
            Title = "Morning Light",
            Content = "The sun rises gently over the hills...",
            Mood = Emotion.Calm
        };

        // Act
        var response = await _client.PostAsync("/api/creative/works",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("title").GetString().Should().Be("Morning Light");
        data.GetProperty("content").GetString().Should().Be("The sun rises gently over the hills...");
    }

    [Fact]
    public async Task GetWorks_ReturnsPagedResults()
    {
        // Arrange -- create a work first
        var createRequest = new CreateWorkRequest
        {
            Type = CreativeWorkType.Story,
            Title = "Test Story",
            Content = "Once upon a time...",
            Mood = Emotion.Happy
        };
        await _client.PostAsync("/api/creative/works",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));

        // Act
        var response = await _client.GetAsync("/api/creative/works?page=1&pageSize=10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("works", out _).Should().BeTrue();
        data.GetProperty("page").GetInt32().Should().Be(1);
        data.GetProperty("pageSize").GetInt32().Should().Be(10);
    }

    [Fact]
    public async Task GeneratePrompt_ReturnsPromptAndType()
    {
        // Arrange
        var request = new GeneratePromptRequest
        {
            Type = CreativeWorkType.Reflection
        };

        // Act
        var response = await _client.PostAsync("/api/creative/prompt",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("prompt", out _).Should().BeTrue();
    }

    [Fact]
    public async Task GetSharedWorks_ReturnsPaginatedList()
    {
        // Act
        var response = await _client.GetAsync("/api/creative/shared?page=1&pageSize=10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("works", out _).Should().BeTrue();
        data.GetProperty("page").GetInt32().Should().Be(1);
    }

    [Fact]
    public async Task StartCollaborativeStory_ReturnsStory()
    {
        // Arrange
        var request = new StartStoryRequest
        {
            RoomId = Guid.NewGuid(),
            Title = "The Collaborative Journey"
        };

        // Act
        var response = await _client.PostAsync("/api/creative/stories",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("title").GetString().Should().Be("The Collaborative Journey");
    }

    [Fact]
    public async Task Creative_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var getWorksResponse = await unauthenticatedClient.GetAsync("/api/creative/works");
        getWorksResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var createWorkResponse = await unauthenticatedClient.PostAsync("/api/creative/works",
            new StringContent(JsonSerializer.Serialize(new CreateWorkRequest
            {
                Type = CreativeWorkType.Poem,
                Title = "Test",
                Content = "Test content",
                Mood = Emotion.Neutral
            }), Encoding.UTF8, "application/json"));
        createWorkResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var sharedResponse = await unauthenticatedClient.GetAsync("/api/creative/shared");
        sharedResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
