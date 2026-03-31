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

public class CommunityTests : IClassFixture<TestWebApplicationFactory>
{
    private readonly TestWebApplicationFactory _factory;
    private readonly HttpClient _client;

    public CommunityTests(TestWebApplicationFactory factory)
    {
        _factory = factory;
        _client = factory.CreateAuthenticatedClient();
    }

    [Fact]
    public async Task CreateGroup_ReturnsCreatedGroup()
    {
        // Arrange
        var request = new CreateGroupRequest
        {
            Name = "Mindfulness Circle",
            Description = "A group for daily mindfulness practice",
            Category = GroupCategory.Mindfulness
        };

        // Act
        var response = await _client.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(request), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.GetProperty("name").GetString().Should().Be("Mindfulness Circle");
        data.GetProperty("description").GetString().Should().Be("A group for daily mindfulness practice");
    }

    [Fact]
    public async Task GetGroups_ReturnsPagedResults()
    {
        // Arrange -- create a group so there is data
        var createRequest = new CreateGroupRequest
        {
            Name = "Wellness Warriors",
            Description = "Wellness discussion group",
            Category = GroupCategory.Wellness
        };
        await _client.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));

        // Act
        var response = await _client.GetAsync("/api/community/groups?page=1&pageSize=10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        var root = json.RootElement;

        root.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = root.GetProperty("data");
        data.TryGetProperty("groups", out _).Should().BeTrue();
        data.GetProperty("page").GetInt32().Should().Be(1);
        data.GetProperty("pageSize").GetInt32().Should().Be(10);
    }

    [Fact]
    public async Task GetGroup_ById_ReturnsGroup()
    {
        // Arrange -- create a group first
        var createRequest = new CreateGroupRequest
        {
            Name = "Support Network",
            Description = "Peer support group",
            Category = GroupCategory.Support
        };
        var createResponse = await _client.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));
        var createContent = await createResponse.Content.ReadAsStringAsync();
        var createJson = JsonDocument.Parse(createContent);
        var groupId = createJson.RootElement.GetProperty("data").GetProperty("id").GetGuid();

        // Act
        var response = await _client.GetAsync($"/api/community/groups/{groupId}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
        json.RootElement.GetProperty("data").GetProperty("name").GetString().Should().Be("Support Network");
    }

    [Fact]
    public async Task JoinGroup_ReturnsOk()
    {
        // Arrange -- create a group
        var createRequest = new CreateGroupRequest
        {
            Name = "Join Test Group",
            Description = "Group for join test",
            Category = GroupCategory.Interest
        };
        var createResponse = await _client.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(createRequest), Encoding.UTF8, "application/json"));
        var createContent = await createResponse.Content.ReadAsStringAsync();
        var createJson = JsonDocument.Parse(createContent);
        var groupId = createJson.RootElement.GetProperty("data").GetProperty("id").GetGuid();

        // Act
        var response = await _client.PostAsync($"/api/community/groups/{groupId}/join", null);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
    }

    [Fact]
    public async Task CreatePost_InGroup_ReturnsPost()
    {
        // Arrange -- create a group and join it
        var createGroupRequest = new CreateGroupRequest
        {
            Name = "Post Test Group",
            Description = "Group for post test",
            Category = GroupCategory.Support
        };
        var groupResponse = await _client.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(createGroupRequest), Encoding.UTF8, "application/json"));
        var groupContent = await groupResponse.Content.ReadAsStringAsync();
        var groupJson = JsonDocument.Parse(groupContent);
        var groupId = groupJson.RootElement.GetProperty("data").GetProperty("id").GetGuid();

        await _client.PostAsync($"/api/community/groups/{groupId}/join", null);

        var postRequest = new CreatePostRequest
        {
            Title = "My first post",
            Content = "Hello everyone, glad to be here!",
            IsAnonymous = false
        };

        // Act
        var response = await _client.PostAsync($"/api/community/groups/{groupId}/posts",
            new StringContent(JsonSerializer.Serialize(postRequest), Encoding.UTF8, "application/json"));

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();

        var data = json.RootElement.GetProperty("data");
        data.GetProperty("title").GetString().Should().Be("My first post");
        data.GetProperty("content").GetString().Should().Be("Hello everyone, glad to be here!");
    }

    [Fact]
    public async Task GetMyGroups_ReturnsUserGroups()
    {
        // Act
        var response = await _client.GetAsync("/api/community/my-groups");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
        json.RootElement.GetProperty("data").TryGetProperty("groups", out _).Should().BeTrue();
    }

    [Fact]
    public async Task GetSuggestedGroups_ReturnsGroups()
    {
        // Act
        var response = await _client.GetAsync("/api/community/suggested");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);

        var content = await response.Content.ReadAsStringAsync();
        var json = JsonDocument.Parse(content);
        json.RootElement.GetProperty("success").GetBoolean().Should().BeTrue();
        json.RootElement.GetProperty("data").TryGetProperty("groups", out _).Should().BeTrue();
    }

    [Fact]
    public async Task Community_RequiresAuthentication()
    {
        var unauthenticatedClient = _factory.CreateClient();

        var getGroupsResponse = await unauthenticatedClient.GetAsync("/api/community/groups");
        getGroupsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var createGroupResponse = await unauthenticatedClient.PostAsync("/api/community/groups",
            new StringContent(JsonSerializer.Serialize(new CreateGroupRequest
            {
                Name = "Test",
                Description = "Test",
                Category = GroupCategory.Support
            }), Encoding.UTF8, "application/json"));
        createGroupResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);

        var myGroupsResponse = await unauthenticatedClient.GetAsync("/api/community/my-groups");
        myGroupsResponse.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }
}
