using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using DigitalTwin.API.Middleware;

namespace DigitalTwin.API.Controllers
{
    [ApiController]
    [Route("api/[controller]")]
    [Authorize]
    [RateLimit(20, "emotion")]
    public class EmotionController : ControllerBase
    {
        private readonly IHttpClientFactory _httpClientFactory;
        private readonly ILogger<EmotionController> _logger;
        private readonly string _serviceKey;

        public EmotionController(
            IHttpClientFactory httpClientFactory,
            ILogger<EmotionController> logger)
        {
            _httpClientFactory = httpClientFactory;
            _logger = logger;
            _serviceKey = Environment.GetEnvironmentVariable("Services__ServiceKey") ?? "dev-service-key";
        }

        [HttpPost("analyze")]
        public async Task<IActionResult> AnalyzeEmotion()
        {
            try
            {
                var client = _httpClientFactory.CreateClient("DeepFace");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                using var content = new MultipartFormDataContent();

                foreach (var file in Request.Form.Files)
                {
                    var stream = file.OpenReadStream();
                    var streamContent = new StreamContent(stream);
                    streamContent.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue(file.ContentType);
                    content.Add(streamContent, file.Name, file.FileName);
                }

                var response = await client.PostAsync("/analyze/facial-expression", content);
                var responseContent = await response.Content.ReadAsStringAsync();

                if (!response.IsSuccessStatusCode)
                {
                    _logger.LogWarning("Emotion analysis failed with status {Status}", response.StatusCode);
                    return StatusCode((int)response.StatusCode, new { success = false, message = "Emotion analysis failed" });
                }

                return Content(responseContent, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying emotion analysis request");
                return StatusCode(500, new { success = false, message = "Failed to analyze emotion" });
            }
        }
    }
}
