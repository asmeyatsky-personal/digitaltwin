using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.Logging;
using Microsoft.IdentityModel.Tokens;
using System;
using System.IdentityModel.Tokens.Jwt;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using DigitalTwin.Core.Security;
using DigitalTwin.Core.DTOs;
using System.Net;

namespace DigitalTwin.Infrastructure.Middleware
{
    /// <summary>
    /// Middleware for handling JWT authentication and authorization
    /// </summary>
    public class JwtAuthenticationMiddleware
    {
        private readonly RequestDelegate _next;
        private readonly JwtConfiguration _jwtConfig;
        private readonly ILogger<JwtAuthenticationMiddleware> _logger;
        private readonly ISecurityEventLogger _securityEventLogger;

        public JwtAuthenticationMiddleware(
            RequestDelegate next,
            IOptions<JwtConfiguration> jwtConfig,
            ILogger<JwtAuthenticationMiddleware> logger,
            ISecurityEventLogger securityEventLogger)
        {
            _next = next;
            _jwtConfig = jwtConfig.Value;
            _logger = logger;
            _securityEventLogger = securityEventLogger;
        }

        public async Task InvokeAsync(HttpContext context)
        {
            var endpoint = context.GetEndpoint();
            if (endpoint?.Metadata?.GetMetadata<AllowAnonymousAttribute>() != null)
            {
                await _next(context);
                return;
            }

            var token = ExtractTokenFromRequest(context);
            if (string.IsNullOrEmpty(token))
            {
                await HandleUnauthorizedAsync(context, "No token provided");
                return;
            }

            try
            {
                var claimsPrincipal = ValidateToken(token);
                if (claimsPrincipal == null)
                {
                    await HandleUnauthorizedAsync(context, "Invalid token");
                    return;
                }

                context.User = claimsPrincipal;

                await LogSecurityEventAsync(context, SecurityEventType.TokenValidated, "Token validated successfully");
            }
            catch (SecurityTokenExpiredException)
            {
                await HandleUnauthorizedAsync(context, "Token has expired");
                await LogSecurityEventAsync(context, SecurityEventType.TokenExpired, "Token expired");
            }
            catch (SecurityTokenInvalidSignatureException)
            {
                await HandleUnauthorizedAsync(context, "Invalid token signature");
                await LogSecurityEventAsync(context, SecurityEventType.InvalidToken, "Invalid token signature");
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error validating JWT token");
                await HandleUnauthorizedAsync(context, "Token validation failed");
                await LogSecurityEventAsync(context, SecurityEventType.InvalidToken, "Token validation failed");
            }

            await _next(context);
        }

        private string? ExtractTokenFromRequest(HttpContext context)
        {
            var authHeader = context.Request.Headers["Authorization"].FirstOrDefault();
            if (authHeader != null && authHeader.StartsWith("Bearer "))
            {
                return authHeader.Substring("Bearer ".Length).Trim();
            }

            return context.Request.Query["access_token"].FirstOrDefault();
        }

        private System.Security.Claims.ClaimsPrincipal? ValidateToken(string token)
        {
            try
            {
                var tokenHandler = new JwtSecurityTokenHandler();
                var key = Encoding.ASCII.GetBytes(_jwtConfig.SecretKey);

                var validationParameters = new TokenValidationParameters
                {
                    ValidateIssuerSigningKey = true,
                    IssuerSigningKey = new SymmetricSecurityKey(key),
                    ValidateIssuer = true,
                    ValidIssuer = _jwtConfig.Issuer,
                    ValidateAudience = true,
                    ValidAudience = _jwtConfig.Audience,
                    ValidateLifetime = true,
                    ClockSkew = TimeSpan.Zero
                };

                var principal = tokenHandler.ValidateToken(token, validationParameters, out SecurityToken validatedToken);
                
                return principal;
            }
            catch
            {
                return null;
            }
        }

        private async Task HandleUnauthorizedAsync(HttpContext context, string message)
        {
            context.Response.StatusCode = (int)HttpStatusCode.Unauthorized;
            context.Response.ContentType = "application/json";
            
            var response = new
            {
                error = "Unauthorized",
                message = message,
                timestamp = DateTime.UtcNow
            };

            await context.Response.WriteAsync(System.Text.Json.JsonSerializer.Serialize(response));
        }

        private async Task LogSecurityEventAsync(HttpContext context, SecurityEventType eventType, string description)
        {
            var userId = context.User?.FindFirst(System.Security.Claims.ClaimTypes.NameIdentifier)?.Value;
            var ipAddress = context.Connection.RemoteIpAddress?.ToString();
            var userAgent = context.Request.Headers["User-Agent"].ToString();
            var requestPath = context.Request.Path;

            await _securityEventLogger.LogSecurityEventAsync(new Core.Models.SecurityEvent
            {
                EventType = eventType,
                UserId = userId,
                Description = description,
                IpAddress = ipAddress,
                UserAgent = userAgent,
                RequestPath = requestPath,
                Timestamp = DateTime.UtcNow
            });
        }
    }

    /// <summary>
    /// Middleware for handling CORS and security headers
    /// </summary>
    public class SecurityHeadersMiddleware
    {
        private readonly RequestDelegate _next;

        public SecurityHeadersMiddleware(RequestDelegate next)
        {
            _next = next;
        }

        public async Task InvokeAsync(HttpContext context)
        {
            // Add comprehensive security headers
            context.Response.Headers.Add("X-Content-Type-Options", "nosniff");
            context.Response.Headers.Add("X-Frame-Options", "DENY");
            context.Response.Headers.Add("X-XSS-Protection", "1; mode=block");
            context.Response.Headers.Add("Strict-Transport-Security", "max-age=31536000; includeSubDomains; preload");
            context.Response.Headers.Add("Referrer-Policy", "strict-origin-when-cross-origin");
            context.Response.Headers.Add("Permissions-Policy", 
                "geolocation=(), " +
                "microphone=(), " +
                "camera=(), " +
                "magnetometer=(), " +
                "gyroscope=(), " +
                "speaker=(), " +
                "notifications=(), " +
                "push=(), " +
                "midi=(), " +
                "vibrate=(), " +
                "payment=(), " +
                "encrypted-media=()");
            context.Response.Headers.Add("Cross-Origin-Embedder-Policy", "require-corp");
            context.Response.Headers.Add("Cross-Origin-Opener-Policy", "same-origin");
            context.Response.Headers.Add("Cross-Origin-Resource-Policy", "same-origin");
            context.Response.Headers.Add("Content-Security-Policy", 
                "default-src 'self'; " +
                "script-src 'self' 'nonce-{nonce}'; " +
                "style-src 'self' 'nonce-{nonce}'; " +
                "img-src 'self' data: https:; " +
                "font-src 'self'; " +
                "connect-src 'self'; " +
                "frame-ancestors 'none'; " +
                "base-uri 'self'; " +
                "form-action 'self'; " +
                "upgrade-insecure-requests");

            await _next(context);
        }
    }

    /// <summary>
    /// Attribute to allow anonymous access to specific endpoints
    /// </summary>
    public class AllowAnonymousAttribute : Attribute
    {
    }

    /// <summary>
    /// Attribute to specify required roles for authorization
    /// </summary>
    public class RequireRoleAttribute : Attribute
    {
        public string[] Roles { get; }

        public RequireRoleAttribute(params string[] roles)
        {
            Roles = roles;
        }
    }
}