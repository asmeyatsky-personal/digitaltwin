using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using DigitalTwin.Core.DTOs;
using DigitalTwin.Core.Interfaces;
using DigitalTwin.API.Middleware;
using System.ComponentModel.DataAnnotations;

namespace DigitalTwin.API.Controllers
{
    /// <summary>
    /// Conversation Controller for Emotional Companion AI
    ///
    /// Architectural Intent:
    /// - Provides REST API for conversational interactions
    /// - Delegates all business logic to IConversationService
    /// - Handles only HTTP concerns (auth, model binding, responses)
    /// </summary>
    [ApiController]
    [Route("api/[controller]")]
    [Authorize]
    [RateLimit(30, "conversation")]
    public class ConversationController : ControllerBase
    {
        private readonly IConversationService _conversationService;
        private readonly ILogger<ConversationController> _logger;

        public ConversationController(
            IConversationService conversationService,
            ILogger<ConversationController> logger)
        {
            _conversationService = conversationService;
            _logger = logger;
        }

        /// <summary>
        /// Start a new conversation session
        /// </summary>
        [HttpPost("start")]
        public async Task<ActionResult<ConversationStartResponse>> StartConversation(
            [FromBody] ConversationStartRequest request)
        {
            try
            {
                var userId = GetUserId();
                if (userId == null)
                {
                    return Unauthorized(ApiResponse.Fail("User not authenticated"));
                }

                var result = await _conversationService.StartConversationAsync(userId.Value, request.Message);

                if (!result.Success)
                {
                    return StatusCode(500, ApiResponse.Fail(result.ErrorMessage ?? "Failed to start conversation"));
                }

                return Ok(ApiResponse<ConversationStartResponse>.Ok(new ConversationStartResponse
                {
                    SessionId = result.SessionId ?? Guid.NewGuid(),
                    Response = result.Response,
                    EmotionalTone = result.DetectedEmotion.ToString(),
                    Timestamp = result.ResponseTime ?? DateTime.UtcNow
                }));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error starting conversation");
                return StatusCode(500, ApiResponse.Fail("Internal server error"));
            }
        }

        /// <summary>
        /// Send a message in the conversation
        /// </summary>
        [HttpPost("message")]
        public async Task<ActionResult<ConversationMessageResponse>> SendMessage(
            [FromBody] ConversationMessageRequest request)
        {
            try
            {
                var userId = GetUserId();
                if (userId == null)
                {
                    return Unauthorized(ApiResponse.Fail("User not authenticated"));
                }

                var result = await _conversationService.ProcessMessageAsync(
                    userId.Value, request.ConversationId, request.Message);

                if (!result.Success)
                {
                    return StatusCode(500, ApiResponse.Fail(result.ErrorMessage ?? "Failed to process message"));
                }

                return Ok(ApiResponse<ConversationMessageResponse>.Ok(new ConversationMessageResponse
                {
                    Response = result.Response,
                    DetectedEmotion = result.DetectedEmotion.ToString(),
                    AIEmotionalTone = result.DetectedEmotion.ToString(),
                    ResponseTime = result.ResponseTime ?? DateTime.UtcNow,
                    ConversationId = request.ConversationId
                }));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error sending message");
                return StatusCode(500, ApiResponse.Fail("Internal server error"));
            }
        }

        /// <summary>
        /// Get conversation history
        /// </summary>
        [HttpGet("history/{conversationId}")]
        public async Task<ActionResult<ConversationHistoryResponse>> GetConversationHistory(
            Guid conversationId,
            [FromQuery] int page = 1,
            [FromQuery] int pageSize = 50)
        {
            try
            {
                var userId = GetUserId();
                if (userId == null)
                {
                    return Unauthorized(ApiResponse.Fail("User not authenticated"));
                }

                var result = await _conversationService.GetConversationHistoryAsync(
                    userId.Value, conversationId, page, pageSize);

                var messages = result.Messages.Select(m => new ConversationMessageDto
                {
                    Id = m.Id,
                    Content = m.Content,
                    Response = m.Response,
                    UserEmotion = m.UserEmotion,
                    AIEmotion = m.AIEmotion,
                    Timestamp = m.Timestamp,
                    MessageType = m.MessageType
                }).ToList();

                return Ok(ApiResponse<ConversationHistoryResponse>.Ok(new ConversationHistoryResponse
                {
                    Messages = messages,
                    TotalCount = result.TotalCount,
                    Page = result.Page,
                    PageSize = result.PageSize
                }));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error getting conversation history");
                return StatusCode(500, ApiResponse.Fail("Internal server error"));
            }
        }

        /// <summary>
        /// End conversation session
        /// </summary>
        [HttpPost("end")]
        public async Task<ActionResult> EndConversation(
            [FromBody] ConversationEndRequest request)
        {
            try
            {
                var userId = GetUserId();
                if (userId == null)
                {
                    return Unauthorized(ApiResponse.Fail("User not authenticated"));
                }

                var success = await _conversationService.EndConversationAsync(userId.Value);

                if (!success)
                {
                    return NotFound(ApiResponse.Fail("No active conversation found"));
                }

                return Ok(ApiResponse.Ok("Conversation ended successfully"));
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error ending conversation");
                return StatusCode(500, ApiResponse.Fail("Internal server error"));
            }
        }

        private Guid? GetUserId()
        {
            var userIdClaim = User.FindFirst(System.Security.Claims.ClaimTypes.NameIdentifier)?.Value;
            if (string.IsNullOrEmpty(userIdClaim))
            {
                return null;
            }

            if (Guid.TryParse(userIdClaim, out var userId))
            {
                return userId;
            }

            // Generate a deterministic Guid from a non-Guid user identifier
            var padded = userIdClaim.PadRight(32, '0').Substring(0, 32);
            var formatted = padded.Insert(8, "-").Insert(13, "-").Insert(18, "-").Insert(23, "-");
            return Guid.Parse(formatted);
        }
    }

    // DTOs
    public class ConversationStartRequest
    {
        [Required]
        [StringLength(5000, MinimumLength = 1)]
        public string Message { get; set; } = string.Empty;
    }

    public class ConversationStartResponse
    {
        public Guid SessionId { get; set; }
        public string Response { get; set; } = string.Empty;
        public string EmotionalTone { get; set; } = string.Empty;
        public DateTime Timestamp { get; set; }
    }

    public class ConversationMessageRequest
    {
        [Required]
        public Guid ConversationId { get; set; }

        [Required]
        [StringLength(5000, MinimumLength = 1)]
        public string Message { get; set; } = string.Empty;
    }

    public class ConversationMessageResponse
    {
        public string Response { get; set; } = string.Empty;
        public string DetectedEmotion { get; set; } = string.Empty;
        public string AIEmotionalTone { get; set; } = string.Empty;
        public DateTime ResponseTime { get; set; }
        public Guid ConversationId { get; set; }
    }

    public class ConversationHistoryResponse
    {
        public List<ConversationMessageDto> Messages { get; set; } = new();
        public int TotalCount { get; set; }
        public int Page { get; set; }
        public int PageSize { get; set; }
    }

    public class ConversationMessageDto
    {
        public Guid Id { get; set; }
        public string Content { get; set; } = string.Empty;
        public string? Response { get; set; }
        public string UserEmotion { get; set; } = string.Empty;
        public string? AIEmotion { get; set; }
        public DateTime Timestamp { get; set; }
        public string MessageType { get; set; } = string.Empty;
    }

    public class ConversationEndRequest
    {
        public Guid ConversationId { get; set; }
        public TimeSpan? SessionDuration { get; set; }
    }

    public class EmotionDetectionResult
    {
        public string Emotion { get; set; } = string.Empty;
        public double Confidence { get; set; }
    }

    public class LLMResponse
    {
        public string Response { get; set; } = string.Empty;
        public double Confidence { get; set; }
    }
}
