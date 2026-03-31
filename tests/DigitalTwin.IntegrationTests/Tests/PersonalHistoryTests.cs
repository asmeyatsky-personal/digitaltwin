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

public class PersonalHistoryTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public PersonalHistoryTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task AddLifeEvent_ReturnsCreatedEvent()
    {
        // Arrange
        var request = new AddLifeEventRequest
        {
            Title = "Started new job",
            Description = "Began working as a software engineer",
            EventDate = DateTime.UtcNow.AddDays(-30),
            Category = LifeEventCategory.Career,
            EmotionalImpact = Emotion.Excited,
            IsRecurring = false
        };

        // Act
        var response = await _client.PostAsync("/api/personal-history/events",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("title").GetString().Should().Be("Started new job");
        data.GetProperty("description").GetString().Should().Be("Began working as a software engineer");
    }

    [Fact]
    public async Task GetTimeline_ReturnsEvents()
    {
        // Arrange -- add an event first
        var addRequest = new AddLifeEventRequest
        {
            Title = "Timeline test event",
            Description = "Event for timeline test",
            EventDate = DateTime.UtcNow,
            Category = LifeEventCategory.Milestone,
            EmotionalImpact = Emotion.Happy,
            IsRecurring = false
        };
        await _client.PostAsync("/api/personal-history/events",
            new StringContent(JsonSerializer.Serialize(addRequest), Encoding.UTF8, "application/json"));

        // Act
        var response = await _client.GetAsync("/api/personal-history/timeline");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task GetUpcomingEvents_ReturnsEvents()
    {
        // Arrange -- add a future event
        var addRequest = new AddLifeEventRequest
        {
            Title = "Birthday celebration",
            Description = "Annual birthday event",
            EventDate = DateTime.UtcNow.AddDays(15),
            Category = LifeEventCategory.Milestone,
            EmotionalImpact = Emotion.Happy,
            IsRecurring = true
        };
        await _client.PostAsync("/api/personal-history/events",
            new StringContent(JsonSerializer.Serialize(addRequest), Encoding.UTF8, "application/json"));

        // Act
        var response = await _client.GetAsync("/api/personal-history/upcoming?daysAhead=30");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task UpdatePersonalContext_ReturnsUpdatedContext()
    {
        // Arrange
        var request = new UpdatePersonalContextRequest
        {
            CulturalBackground = "East Asian",
            CommunicationPreferences = "{\"style\":\"direct\",\"language\":\"en\"}",
            ImportantPeople = "[\"Mom\",\"Dad\",\"Best friend\"]",
            Values = "[\"family\",\"growth\",\"honesty\"]"
        };

        // Act
        var response = await _client.PutAsync("/api/personal-history/context",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("culturalBackground").GetString().Should().Be("East Asian");
    }

    [Fact]
    public async Task PersonalHistory_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var addEventResponse = await unauthenticatedClient.PostAsync("/api/personal-history/events",
            new StringContent(JsonSerializer.Serialize(new AddLifeEventRequest
            {
                Title = "Test",
                Description = "Test",
                EventDate = DateTime.UtcNow,
                Category = LifeEventCategory.Milestone,
                EmotionalImpact = Emotion.Neutral
            }), Encoding.UTF8, "application/json"));
        addEventResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var timelineResponse = await unauthenticatedClient.GetAsync("/api/personal-history/timeline");
        timelineResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var contextResponse = await unauthenticatedClient.GetAsync("/api/personal-history/context");
        contextResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
