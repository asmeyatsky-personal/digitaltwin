using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.Filters;
using System;
using System.Linq;
using System.Security.Claims;

namespace DigitalTwin.Infrastructure.Filters
{
    /// <summary>
    /// Authorization filter to prevent insecure direct object references
    /// </summary>
    public class AuthorizeUserOwnershipAttribute : Attribute, IActionFilter
    {
        private readonly string _userIdParameterName;

        public AuthorizeUserOwnershipAttribute(string userIdParameterName = "id")
        {
            _userIdParameterName = userIdParameterName;
        }

        public void OnActionExecuting(ActionExecutingContext context)
        {
            // Get current user ID from JWT claims
            var currentUserId = context.HttpContext.User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
            var userRole = context.HttpContext.User.FindFirst(ClaimTypes.Role)?.Value;

            // Admins can access any user data
            if (userRole == "Admin" || userRole == "SystemAdmin")
            {
                return;
            }

            // Check if trying to access another user's data
            if (context.ActionArguments.TryGetValue(_userIdParameterName, out var userIdValue))
            {
                var requestedUserId = userIdValue?.ToString();

                if (string.IsNullOrEmpty(requestedUserId) || requestedUserId != currentUserId)
                {
                    var errorResponse = new
                    {
                        success = false,
                        message = "Access denied",
                        errors = new[] { "You can only access your own data" }
                    };

                    context.Result = new ForbidResult();
                    return;
                }
            }
        }

        public void OnActionExecuted(ActionExecutedContext context)
        {
            // No implementation needed
        }
    }

    /// <summary>
    /// Permission-based authorization filter
    /// </summary>
    public class RequirePermissionAttribute : Attribute, IActionFilter
    {
        private readonly string _requiredPermission;

        public RequirePermissionAttribute(string requiredPermission)
        {
            _requiredPermission = requiredPermission;
        }

        public void OnActionExecuting(ActionExecutingContext context)
        {
            var userPermissions = context.HttpContext.User.Claims
                .Where(c => c.Type == "permission")
                .Select(c => c.Value)
                .ToList();

            // Check if user has the required permission
            if (!userPermissions.Contains(_requiredPermission))
            {
                var errorResponse = new
                {
                    success = false,
                    message = "Access denied",
                    errors = new[] { $"Permission '{_requiredPermission}' is required" }
                };

                context.Result = new ForbidResult();
                return;
            }
        }

        public void OnActionExecuted(ActionExecutedContext context)
        {
            // No implementation needed
        }
    }

    /// <summary>
    /// Rate limiting filter
    /// </summary>
    public class RateLimitAttribute : Attribute, IActionFilter
    {
        private readonly int _requestsPerMinute;
        private readonly string _identifier;

        public int RequestsPerMinute => _requestsPerMinute;
        public string Identifier => _identifier;

        public RateLimitAttribute(int requestsPerMinute = 100, string identifier = "default")
        {
            _requestsPerMinute = requestsPerMinute;
            _identifier = identifier;
        }

        public void OnActionExecuting(ActionExecutingContext context)
        {
            // This is a simplified in-memory rate limiter
            // In production, use Redis-based distributed rate limiting
            var clientId = GetClientId(context);
            var key = $"rate_limit_{_identifier}_{clientId}";
            
            // Implementation would go here with Redis
            // For now, just log the request
            context.HttpContext.Items["RateLimitKey"] = key;
        }

        public void OnActionExecuted(ActionExecutedContext context)
        {
            // No implementation needed
        }

        private string GetClientId(ActionExecutingContext context)
        {
            // Try to get user ID from claims
            var userId = context.HttpContext.User.FindFirst(ClaimTypes.NameIdentifier)?.Value;
            if (!string.IsNullOrEmpty(userId))
            {
                return userId;
            }

            // Fall back to IP address
            return context.HttpContext.Connection.RemoteIpAddress?.ToString() ?? "unknown";
        }
    }
}