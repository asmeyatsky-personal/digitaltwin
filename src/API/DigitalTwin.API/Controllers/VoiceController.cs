using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using DigitalTwin.Core.DTOs;
using DigitalTwin.API.Middleware;

namespace DigitalTwin.API.Controllers
{
    [ApiController]
    [Route("api/[controller]")]
    [Authorize]
    [RateLimit(15, "voice")]
    public class VoiceController : ControllerBase
    {
        private readonly IHttpClientFactory _httpClientFactory;
        private readonly ILogger<VoiceController> _logger;
        private readonly string _serviceKey;

        public VoiceController(
            IHttpClientFactory httpClientFactory,
            ILogger<VoiceController> logger)
        {
            _httpClientFactory = httpClientFactory;
            _logger = logger;
            _serviceKey = Environment.GetEnvironmentVariable("Services__ServiceKey") ?? "dev-service-key";
        }

        [HttpGet("list")]
        public async Task<IActionResult> ListVoices()
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Voice");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var response = await client.GetAsync("/voices");
                var content = await response.Content.ReadAsStringAsync();

                return Content(content, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying voice list request");
                return StatusCode(500, new { success = false, message = "Failed to fetch voices" });
            }
        }

        [HttpGet("user/{userId}")]
        public async Task<IActionResult> GetUserVoice(string userId)
        {
            try
            {
                var authenticatedUserId = User.FindFirst(System.Security.Claims.ClaimTypes.NameIdentifier)?.Value;
                var targetUserId = userId == "me" ? authenticatedUserId : userId;

                var client = _httpClientFactory.CreateClient("Voice");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var response = await client.GetAsync($"/voice/user/{targetUserId}");
                var content = await response.Content.ReadAsStringAsync();

                return Content(content, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying user voice request");
                return StatusCode(500, new { success = false, message = "Failed to fetch user voice" });
            }
        }

        [HttpPost("tts")]
        public async Task<IActionResult> TextToSpeech([FromBody] TTSProxyRequest request)
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Voice");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                var response = await client.PostAsJsonAsync("/tts", new
                {
                    text = request.Text,
                    voice_id = request.VoiceId
                });

                if (!response.IsSuccessStatusCode)
                {
                    return StatusCode((int)response.StatusCode, new { success = false, message = "TTS generation failed" });
                }

                var audioStream = await response.Content.ReadAsStreamAsync();
                var contentType = response.Content.Headers.ContentType?.ToString() ?? "audio/mpeg";

                return File(audioStream, contentType);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying TTS request");
                return StatusCode(500, new { success = false, message = "Failed to generate speech" });
            }
        }

        [HttpPost("analyze-emotion")]
        public async Task<IActionResult> AnalyzeVoiceEmotion()
        {
            try
            {
                var file = Request.Form.Files.FirstOrDefault();
                if (file == null)
                    return BadRequest(new { success = false, message = "No audio file provided" });

                var client = _httpClientFactory.CreateClient("Voice");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                using var content = new MultipartFormDataContent();
                var stream = file.OpenReadStream();
                var streamContent = new StreamContent(stream);
                streamContent.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue(file.ContentType ?? "audio/wav");
                content.Add(streamContent, "file", file.FileName ?? "audio.wav");

                var response = await client.PostAsync("/voice/analyze-emotion", content);
                var responseContent = await response.Content.ReadAsStringAsync();

                if (!response.IsSuccessStatusCode)
                    return StatusCode((int)response.StatusCode, ApiResponse.Fail("Voice emotion analysis failed"));

                // Wrap upstream response in AD-2 envelope
                var emotionData = System.Text.Json.JsonSerializer.Deserialize<object>(responseContent);
                return Ok(ApiResponse<object>.Ok(emotionData));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying voice emotion analysis request");
                return StatusCode(500, ApiResponse.Fail("Failed to analyze voice emotion"));
            }
        }

        [HttpPost("stt")]
        public async Task<IActionResult> SpeechToText()
        {
            try
            {
                var file = Request.Form.Files.FirstOrDefault();
                if (file == null)
                    return BadRequest(ApiResponse.Fail("No audio file provided"));

                var client = _httpClientFactory.CreateClient("Voice");
                client.DefaultRequestHeaders.Add("X-Service-Key", _serviceKey);

                using var content = new MultipartFormDataContent();
                var stream = file.OpenReadStream();
                var streamContent = new StreamContent(stream);
                streamContent.Headers.ContentType = new System.Net.Http.Headers.MediaTypeHeaderValue(file.ContentType ?? "audio/wav");
                content.Add(streamContent, "file", file.FileName ?? "audio.wav");

                var response = await client.PostAsync("/voice/stt", content);
                var responseContent = await response.Content.ReadAsStringAsync();

                if (!response.IsSuccessStatusCode)
                    return StatusCode((int)response.StatusCode, ApiResponse.Fail("Speech-to-text failed"));

                var sttData = System.Text.Json.JsonSerializer.Deserialize<object>(responseContent);
                return Ok(ApiResponse<object>.Ok(sttData));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying STT request");
                return StatusCode(500, ApiResponse.Fail("Failed to transcribe audio"));
            }
        }

        [HttpPost("clone")]
        public async Task<IActionResult> CloneVoice()
        {
            try
            {
                var client = _httpClientFactory.CreateClient("Voice");
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

                var response = await client.PostAsync("/voice/clone", content);
                var responseContent = await response.Content.ReadAsStringAsync();

                return Content(responseContent, "application/json");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error proxying voice clone request");
                return StatusCode(500, new { success = false, message = "Failed to clone voice" });
            }
        }
    }

    public class TTSProxyRequest
    {
        public string Text { get; set; } = string.Empty;
        public string? VoiceId { get; set; }
    }
}
