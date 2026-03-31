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

public class TherapyTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public TherapyTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task GetTherapists_ReturnsPagedResults()
    {
        // Act
        var response = await _client.GetAsync("/api/therapy/therapists?page=1&pageSize=10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("therapists", out _).Should().BeTrue();
        data.GetProperty("page").GetInt32().Should().Be(1);
        data.GetProperty("pageSize").GetInt32().Should().Be(10);
    }

    [Fact]
    public async Task GetSessions_ReturnsUserSessions()
    {
        // Act
        var response = await _client.GetAsync("/api/therapy/sessions?page=1&pageSize=10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("sessions", out _).Should().BeTrue();
        data.GetProperty("page").GetInt32().Should().Be(1);
    }

    [Fact]
    public async Task GetScreeningHistory_ReturnsHistory()
    {
        // Act
        var response = await _client.GetAsync("/api/therapy/screening/history");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task GetReferrals_ReturnsUserReferrals()
    {
        // Act
        var response = await _client.GetAsync("/api/therapy/referrals");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task Therapy_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var therapistsResponse = await unauthenticatedClient.GetAsync("/api/therapy/therapists");
        therapistsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var sessionsResponse = await unauthenticatedClient.GetAsync("/api/therapy/sessions");
        sessionsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var bookResponse = await unauthenticatedClient.PostAsync("/api/therapy/sessions",
            new StringContent(JsonSerializer.Serialize(new BookSessionRequest
            {
                TherapistId = Guid.NewGuid(),
                ScheduledAt = DateTime.UtcNow.AddDays(7)
            }), Encoding.UTF8, "application/json"));
        bookResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var screeningResponse = await unauthenticatedClient.GetAsync("/api/therapy/screening/history");
        screeningResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
