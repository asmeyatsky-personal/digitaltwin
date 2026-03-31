using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using DigitalTwin.API.Middleware;

namespace DigitalTwin.API.Controllers
{
    [ApiController]
    [Route("api/[controller]")]
    [Authorize]
    [RateLimit(10, "avatar")]
    public class AvatarController : ControllerBase
    {
        private readonly IHttpClientFactory _httpClientFactory;
        private readonly ILogger<AvatarController> _logger;
        private readonly string _serviceKey;

        public AvatarController(
            IHttpClientFactory httpClientFactory,
            ILogger<AvatarController> logger)
        {
            _httpClientFactory = httpClientFactory;
            _logger = logger;
            _serviceKey = Environment.GetEnvironmentVariable("Services__ServiceKey") ?? "dev-service-key";
        }

        [HttpPost("generate")]
        public async Task<IActionResult> GenerateAvatar()
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Avatar");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var userId = User.FindFirst(System.Security.Claims.ClaimTypes.NameIdentifier)?.Value;

                using var content = new MultipartFormDataContent();

                foreach (var file in Request.Form.Files)
                {
                    var stream = file.OpenReadStream();
                    var streamContent = new StreamContent(stream);
                    streamContent.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue(file.ContentType);
                    content.Add(streamContent, file.Name, file.FileName);
                }

                content.Add(new StringContent(userId ?? ""), "user_id");

                foreach (var field in Request.Form)
                {
                    if (field.Key != "user_id")
                        content.Add(new StringContent(field.Value.ToString()), field.Key);
                }

                var response = await client.PostAsync("/avatar/generate", content);
                var responseContent = await response.Content.ReadAsStringAsync();

                return Content(responseContent, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying avatar generation request");
                return StatusCode(500, new { success = false, message = "Failed to generate avatar" });
            }
        }

        [HttpGet("{id}/status")]
        public async Task<IActionResult> GetAvatarStatus(string id)
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Avatar");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var response = await client.GetAsync($"/avatar/{id}/status");
                var content = await response.Content.ReadAsStringAsync();

                return Content(content, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying avatar status request");
                return StatusCode(500, new { success = false, message = "Failed to check avatar status" });
            }
        }

        [HttpGet("{id}/download")]
        public async Task<IActionResult> DownloadAvatar(string id)
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Avatar");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var response = await client.GetAsync($"/avatar/{id}/download");

                if (!response.IsSuccessStatusCode)
                {
                    return StatusCode((int)response.StatusCode, new { success = false, message = "Avatar not found" });
                }

                var stream = await response.Content.ReadAsStreamAsync();
                var contentType = response.Content.Headers.ContentType?.ToString() ?? "model/gltf-binary";

                return File(stream, contentType, $"avatar_{id}.glb");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying avatar download request");
                return StatusCode(500, new { success = false, message = "Failed to download avatar" });
            }
        }

        [HttpDelete("{id}")]
        public async Task<IActionResult> DeleteAvatar(string id)
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Avatar");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var userId = User.FindFirst(System.Security.Claims.ClaimTypes.NameIdentifier)?.Value;

                var response = await client.DeleteAsync($"/avatar/{id}?user_id={userId}");
                var content = await response.Content.ReadAsStringAsync();

                return Content(content, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying avatar delete request");
                return StatusCode(500, new { success = false, message = "Failed to delete avatar" });
            }
        }
    }
}
