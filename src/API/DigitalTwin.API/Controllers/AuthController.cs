using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Identity;
using Microsoft.Extensions.Logging;
using DigitalTwin.API.Services;
using DigitalTwin.API.Middleware;
using System.ComponentModel.DataAnnotations;

namespace DigitalTwin.API.Controllers
{
    /// <summary>
    /// Authentication Controller for JWT token management
    ///
    /// Architectural Intent:
    /// - Handles user authentication and authorization
    /// - Manages JWT token generation and validation
    /// - Supports user registration and login
    /// - Provides token refresh functionality
    ///
    /// Key Features:
    /// 1. User registration with validation
    /// 2. User login with JWT token response
    /// 3. Token refresh mechanism
    /// 4. Password reset functionality
    /// 5. User profile management
    /// </summary>
    [ApiController]
    [Route("api/[controller]")]
    [RateLimit(5, "auth")]
    public class AuthController : ControllerBase
    {
        private readonly JwtAuthenticationService _jwtService;
        private readonly ILogger<AuthController> _logger;

        public AuthController(
            JwtAuthenticationService jwtService,
            ILogger<AuthController> logger)
        {
            _jwtService = jwtService;
            _logger = logger;
        }

        /// <summary>
        /// User login endpoint
        /// </summary>
        [HttpPost("login")]
        public async Task<ActionResult<LoginResponse>> Login(
            [FromBody] LoginRequest request)
        {
            try
            {
                _logger.LogInformation("Login attempt for user: {Username}", request.Username);

                // Validate credentials (placeholder - in real implementation, verify against database)
                if (!ValidateCredentials(request.Username, request.Password))
                {
                    _logger.LogWarning("Login failed for user: {Username}", request.Username);
                    return Unauthorized(new LoginResponse
                    {
                        Success = false,
                        Message = "Invalid username or password"
                    });
                }

                // Get user roles (placeholder - in real implementation, fetch from database)
                var roles = GetUserRoles(request.Username);

                // Generate JWT token
                var token = _jwtService.GenerateToken(
                    GetUserId(request.Username), 
                    request.Username, 
                    roles);

                // Generate refresh token
                var refreshToken = _jwtService.GenerateRefreshToken();

                _logger.LogInformation("Login successful for user: {Username}", request.Username);

                return Ok(new LoginResponse
                {
                    Success = true,
                    Token = token,
                    RefreshToken = refreshToken,
                    ExpiresIn = 3600, // 1 hour in seconds
                    User = new UserInfo
                    {
                        UserId = GetUserId(request.Username),
                        Username = request.Username,
                        Roles = roles
                    }
                });
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during login for user: {Username}", request.Username);
                return StatusCode(500, new LoginResponse
                {
                    Success = false,
                    Message = "Internal server error during login"
                });
            }
        }

        /// <summary>
        /// Register new user
        /// </summary>
        [HttpPost("register")]
        public async Task<ActionResult<RegisterResponse>> Register(
            [FromBody] RegisterRequest request)
        {
            try
            {
                _logger.LogInformation("Registration attempt for user: {Username}", request.Username);

                // Validate input
                if (!ModelState.IsValid)
                {
                    return BadRequest(new RegisterResponse
                    {
                        Success = false,
                        Message = "Invalid input data",
                        Errors = ModelState.Values.SelectMany(v => v.Errors).Select(e => e.ErrorMessage).ToList()
                    });
                }

                // Check if user already exists (placeholder - in real implementation, check database)
                if (UserExists(request.Username, request.Email))
                {
                    _logger.LogWarning("Registration failed - user exists: {Username}", request.Username);
                    return Conflict(new RegisterResponse
                    {
                        Success = false,
                        Message = "Username or email already exists"
                    });
                }

                // Create new user (placeholder - in real implementation, save to database)
                var userId = CreateUser(request);
                if (string.IsNullOrEmpty(userId))
                {
                    _logger.LogError("Failed to create user: {Username}", request.Username);
                    return StatusCode(500, new RegisterResponse
                    {
                        Success = false,
                        Message = "Failed to create user account"
                    });
                }

                // Generate JWT token for new user
                var roles = new List<string> { "User" }; // Default role
                var token = _jwtService.GenerateToken(userId, request.Username, roles);
                var refreshToken = _jwtService.GenerateRefreshToken();

                _logger.LogInformation("Registration successful for user: {Username}", request.Username);

                return Ok(new RegisterResponse
                {
                    Success = true,
                    Message = "User registered successfully",
                    Token = token,
                    RefreshToken = refreshToken,
                    User = new UserInfo
                    {
                        UserId = userId,
                        Username = request.Username,
                        Roles = roles
                    }
                });
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during registration for user: {Username}", request.Username);
                return StatusCode(500, new RegisterResponse
                {
                    Success = false,
                    Message = "Internal server error during registration"
                });
            }
        }

        /// <summary>
        /// Refresh JWT token
        /// </summary>
        [HttpPost("refresh")]
        public async Task<ActionResult<TokenRefreshResponse>> RefreshToken(
            [FromBody] TokenRefreshRequest request)
        {
            try
            {
                // Validate refresh token (placeholder - in real implementation, validate against database)
                if (!ValidateRefreshToken(request.RefreshToken))
                {
                    _logger.LogWarning("Invalid refresh token provided");
                    return Unauthorized(new TokenRefreshResponse
                    {
                        Success = false,
                        Message = "Invalid refresh token"
                    });
                }

                // Get user info from existing token
                var userInfo = _jwtService.ExtractUserInfoFromToken(request.Token);
                if (userInfo == null)
                {
                    _logger.LogWarning("Unable to extract user info from token during refresh");
                    return Unauthorized(new TokenRefreshResponse
                    {
                        Success = false,
                        Message = "Invalid token"
                    });
                }

                // Generate new token
                var newToken = _jwtService.GenerateToken(
                    userInfo.UserId, 
                    userInfo.Username, 
                    userInfo.Roles);
                var newRefreshToken = _jwtService.GenerateRefreshToken();

                _logger.LogInformation("Token refreshed successfully for user: {Username}", userInfo.Username);

                return Ok(new TokenRefreshResponse
                {
                    Success = true,
                    Token = newToken,
                    RefreshToken = newRefreshToken,
                    ExpiresIn = 3600
                });
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during token refresh");
                return StatusCode(500, new TokenRefreshResponse
                {
                    Success = false,
                    Message = "Internal server error during token refresh"
                });
            }
        }

        /// <summary>
        /// Validate current token
        /// </summary>
        [HttpGet("validate")]
        [Authorize]
        public ActionResult<TokenValidationResponse> ValidateToken()
        {
            try
            {
                var authHeader = Request.Headers["Authorization"].FirstOrDefault();
                if (string.IsNullOrEmpty(authHeader) || !authHeader.StartsWith("Bearer "))
                {
                    return BadRequest(new TokenValidationResponse
                    {
                        Valid = false,
                        Message = "No token provided"
                    });
                }

                var token = authHeader.Substring("Bearer ".Length).Trim();
                var userInfo = _jwtService.ExtractUserInfoFromToken(token);

                if (userInfo == null)
                {
                    return Ok(new TokenValidationResponse
                    {
                        Valid = false,
                        Message = "Invalid or expired token"
                    });
                }

                return Ok(new TokenValidationResponse
                {
                    Valid = true,
                    Message = "Token is valid",
                    User = userInfo
                });
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error during token validation");
                return StatusCode(500, new TokenValidationResponse
                {
                    Valid = false,
                    Message = "Internal server error during token validation"
                });
            }
        }

        // Helper methods — delegate to proper services in production
        private bool ValidateCredentials(string username, string password)
        {
            // TODO: Replace with AuthenticationService.AuthenticateUserAsync
            // This is a placeholder that rejects all logins until the database is connected
            if (string.IsNullOrEmpty(username) || string.IsNullOrEmpty(password))
                return false;

            // In development mode, allow a single test account
            // NEVER use hardcoded credentials in production
            if (Environment.GetEnvironmentVariable("ASPNETCORE_ENVIRONMENT") == "Development")
            {
                var devPassword = Environment.GetEnvironmentVariable("DEV_TEST_PASSWORD") ?? "";
                return username == "dev" && !string.IsNullOrEmpty(devPassword) && password == devPassword;
            }

            return false;
        }

        private List<string> GetUserRoles(string username)
        {
            // Placeholder role assignment
            // In real implementation, fetch from database
            return username.ToLower() switch
            {
                "admin" => new List<string> { "Admin", "User" },
                _ => new List<string> { "User" }
            };
        }

        private string GetUserId(string username)
        {
            // Placeholder user ID generation
            // In real implementation, fetch from database
            return username switch
            {
                "admin" => "admin-id-12345",
                "user" => "user-id-67890",
                "testuser" => "test-user-id-11111",
                _ => $"user-id-{Guid.NewGuid():N}"
            };
        }

        private bool UserExists(string username, string email)
        {
            // Placeholder user existence check
            // In real implementation, query database
            var existingUsers = new[] { "admin", "user", "testuser" };
            return existingUsers.Contains(username.ToLower());
        }

        private string CreateUser(RegisterRequest request)
        {
            // Placeholder user creation
            // In real implementation, save to database with password hashing
            return $"user-id-{Guid.NewGuid():N}";
        }

        private bool ValidateRefreshToken(string refreshToken)
        {
            // Placeholder refresh token validation
            // In real implementation, validate against database
            return !string.IsNullOrEmpty(refreshToken) && refreshToken.Length > 20;
        }
    }

    // DTOs
    public class LoginRequest
    {
        [Required]
        public string Username { get; set; } = string.Empty;

        [Required]
        public string Password { get; set; } = string.Empty;
    }

    public class LoginResponse
    {
        public bool Success { get; set; }
        public string Message { get; set; } = string.Empty;
        public string? Token { get; set; }
        public string? RefreshToken { get; set; }
        public int ExpiresIn { get; set; }
        public UserInfo? User { get; set; }
    }

    public class RegisterRequest
    {
        [Required]
        [StringLength(50, MinimumLength = 3)]
        public string Username { get; set; } = string.Empty;

        [Required]
        [EmailAddress]
        public string Email { get; set; } = string.Empty;

        [Required]
        [StringLength(100, MinimumLength = 6)]
        public string Password { get; set; } = string.Empty;

        [Required]
        public string FirstName { get; set; } = string.Empty;

        [Required]
        public string LastName { get; set; } = string.Empty;
    }

    public class RegisterResponse
    {
        public bool Success { get; set; }
        public string Message { get; set; } = string.Empty;
        public string? Token { get; set; }
        public string? RefreshToken { get; set; }
        public UserInfo? User { get; set; }
        public List<string>? Errors { get; set; }
    }

    public class TokenRefreshRequest
    {
        [Required]
        public string Token { get; set; } = string.Empty;

        [Required]
        public string RefreshToken { get; set; } = string.Empty;
    }

    public class TokenRefreshResponse
    {
        public bool Success { get; set; }
        public string Message { get; set; } = string.Empty;
        public string? Token { get; set; }
        public string? RefreshToken { get; set; }
        public int ExpiresIn { get; set; }
    }

    public class TokenValidationResponse
    {
        public bool Valid { get; set; }
        public string Message { get; set; } = string.Empty;
        public UserInfo? User { get; set; }
    }
}