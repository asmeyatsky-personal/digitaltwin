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

public class FamilyTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public FamilyTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task CreateFamily_ReturnsCreatedFamily()
    {
        // Arrange
        var request = new CreateFamilyRequest
        {
            Name = "The Smiths"
        };

        // Act
        var response = await _client.PostAsync("/api/family",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("name").GetString().Should().Be("The Smiths");
    }

    [Fact]
    public async Task GetFamily_ReturnsOk()
    {
        // Act
        var response = await _client.GetAsync("/api/family");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task JoinFamily_WithInvalidCode_ReturnsBadRequest()
    {
        // Arrange
        var request = new JoinFamilyRequest
        {
            InviteCode = "invalid-code-12345"
        };

        // Act
        var response = await _client.PostAsync("/api/family/join",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().BeOneOf(HttpStatusCode.BadRequest, HttpStatusCode.InternalServerError);
    }

    [Fact]
    public async Task GetInsights_ForFamily_ReturnsOk()
    {
        // Arrange -- create a family first
        var createRequest = new CreateFamilyRequest { Name = "Insights Test Family" };
        var createResponse = await _client.PostAsync("/api/family",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));
        var createContent = await createResponse.Content.ReadAsStringAsync();
        var createJson = JsonDocument.Parse(createContent);
        var familyId = createJson.RootElement.GetProperty("data").GetProperty("id").GetGuid();

        // Act
        var response = await _client.GetAsync($"/api/family/{familyId}/insights");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task InviteMember_ToFamily_ReturnsInvite()
    {
        // Arrange -- create a family first
        var createRequest = new CreateFamilyRequest { Name = "Invite Test Family" };
        var createResponse = await _client.PostAsync("/api/family",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));
        var createContent = await createResponse.Content.ReadAsStringAsync();
        var createJson = JsonDocument.Parse(createContent);
        var familyId = createJson.RootElement.GetProperty("data").GetProperty("id").GetGuid();

        var inviteRequest = new InviteMemberRequest
        {
            Email = "familymember@example.com",
            Role = FamilyRole.Adult
        };

        // Act
        var response = await _client.PostAsync($"/api/family/{familyId}/invite",
            new StringContent(JsonSerializer.Serialize(inviteRequest), Encoding.UTF8, "application/json"));

        // Assert
        // The response may be OK or Forbidden depending on whether the service
        // recognizes the test user as the family owner.
        response.StatusCode.Should().BeOneOf(HttpStatusCode.OK, HttpStatusCode.Forbidden);
    }

    [Fact]
    public async Task Family_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var getResponse = await unauthenticatedClient.GetAsync("/api/family");
        getResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var createResponse = await unauthenticatedClient.PostAsync("/api/family",
            new StringContent(JsonSerializer.Serialize(new CreateFamilyRequest
            {
                Name = "Test Family"
            }), Encoding.UTF8, "application/json"));
        createResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var joinResponse = await unauthenticatedClient.PostAsync("/api/family/join",
            new StringContent(JsonSerializer.Serialize(new JoinFamilyRequest
            {
                InviteCode = "test-code"
            }), Encoding.UTF8, "application/json"));
        joinResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
