using System.Security.Claims;
using DigitalTwin.API.Services;

namespace DigitalTwin.API.Middleware
{
    public class RateLimitingMiddleware
    {
        private readonly RequestDelegate _next;
        private readonly IRateLimitService _rateLimitService;
        private readonly ILogger<RateLimitingMiddleware> _logger;

        public RateLimitingMiddleware(
            RequestDelegate next,
            IRateLimitService rateLimitService,
            ILogger<RateLimitingMiddleware> logger)
        {
            _next = next;
            _rateLimitService = rateLimitService;
            _logger = logger;
        }

        public async Task InvokeAsync(HttpContext context)
        {
            var clientId = GetClientId(context);
            var endpoint = context.GetEndpoint();
            var limit = GetRateLimit(endpoint);
            var key = $"rate_limit_{limit.Category}_{clientId}";

            var isAllowed = await _rateLimitService.IsAllowedAsync(key, limit.Requests, limit.Window);

            if (!isAllowed)
            {
                _logger.LogWarning("Rate limit exceeded for client {ClientId} on {Path}", clientId, context.Request.Path);

                context.Response.StatusCode = 429;
                context.Response.Headers.Append("Retry-After", ((int)limit.Window.TotalSeconds).ToString());
                context.Response.Headers.Append("X-RateLimit-Limit", limit.Requests.ToString());
                context.Response.Headers.Append("X-RateLimit-Remaining", "0");

                await context.Response.WriteAsync("Rate limit exceeded. Please try again later.");
                return;
            }

            var currentCount = await _rateLimitService.GetCurrentCountAsync(key);
            var resetTime = await _rateLimitService.GetResetTimeAsync(key);

            context.Response.Headers.Append("X-RateLimit-Limit", limit.Requests.ToString());
            context.Response.Headers.Append("X-RateLimit-Remaining", (limit.Requests - currentCount).ToString());

            if (resetTime.HasValue)
            {
                context.Response.Headers.Append("X-RateLimit-Reset", ((long)resetTime.Value.TotalSeconds).ToString());
            }

            await _next(context);
        }

        private string GetClientId(HttpContext context)
        {
            var userId = context.User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
            if (!string.IsNullOrEmpty(userId))
                return $"user:{userId}";

            return $"ip:{context.Connection.RemoteIpAddress?.ToString() ?? "unknown"}";
        }

        private RateLimitConfig GetRateLimit(Endpoint? endpoint)
        {
            var attr = endpoint?.Metadata.GetMetadata<RateLimitAttribute>();
            if (attr != null)
            {
                return new RateLimitConfig
                {
                    Requests = attr.RequestsPerMinute,
                    Window = TimeSpan.FromMinutes(1),
                    Category = attr.Identifier
                };
            }

            return new RateLimitConfig
            {
                Requests = 100,
                Window = TimeSpan.FromMinutes(1),
                Category = "default"
            };
        }
    }

    internal class RateLimitConfig
    {
        public int Requests { get; set; }
        public TimeSpan Window { get; set; }
        public string Category { get; set; } = "default";
    }

    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Method)]
    public class RateLimitAttribute : Attribute
    {
        public int RequestsPerMinute { get; }
        public string Identifier { get; }

        public RateLimitAttribute(int requestsPerMinute = 100, string identifier = "default")
        {
            RequestsPerMinute = requestsPerMinute;
            Identifier = identifier;
        }
    }
}
