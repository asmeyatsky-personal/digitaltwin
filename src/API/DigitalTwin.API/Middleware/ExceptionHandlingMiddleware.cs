using System.ComponentModel.DataAnnotations;

namespace DigitalTwin.API.Middleware
{
    public class ExceptionHandlingMiddleware
    {
        private readonly RequestDelegate _next;
        private readonly ILogger<ExceptionHandlingMiddleware> _logger;

        public ExceptionHandlingMiddleware(RequestDelegate next, ILogger<ExceptionHandlingMiddleware> logger)
        {
            _next = next;
            _logger = logger;
        }

        public async Task InvokeAsync(HttpContext context)
        {
            try
            {
                await _next(context);
            }
            catch (ValidationException ex)
            {
                context.Response.StatusCode = 400;
                context.Response.ContentType = "application/json";
                await context.Response.WriteAsJsonAsync(new
                {
                    success = false,
                    error = "VALIDATION_ERROR",
                    message = ex.Message,
                    timestamp = DateTime.UtcNow
                });
            }
            catch (UnauthorizedAccessException)
            {
                context.Response.StatusCode = 403;
                context.Response.ContentType = "application/json";
                await context.Response.WriteAsJsonAsync(new
                {
                    success = false,
                    error = "FORBIDDEN",
                    message = "Access denied",
                    timestamp = DateTime.UtcNow
                });
            }
            catch (KeyNotFoundException ex)
            {
                context.Response.StatusCode = 404;
                context.Response.ContentType = "application/json";
                await context.Response.WriteAsJsonAsync(new
                {
                    success = false,
                    error = "NOT_FOUND",
                    message = ex.Message,
                    timestamp = DateTime.UtcNow
                });
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Unhandled exception for {Method} {Path}",
                    context.Request.Method, context.Request.Path);

                context.Response.StatusCode = 500;
                context.Response.ContentType = "application/json";

                var isDev = context.RequestServices.GetRequiredService<IHostEnvironment>().IsDevelopment();
                await context.Response.WriteAsJsonAsync(new
                {
                    success = false,
                    error = "INTERNAL_ERROR",
                    message = isDev ? ex.Message : "An internal server error occurred",
                    timestamp = DateTime.UtcNow
                });
            }
        }
    }
}
