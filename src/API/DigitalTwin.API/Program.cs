using Microsoft.AspNetCore.Authentication.JwtBearer;
using Prometheus;
using Microsoft.EntityFrameworkCore;
using Microsoft.IdentityModel.Tokens;
using DigitalTwin.Core.Data;
using DigitalTwin.Core.Security;
using DigitalTwin.Core.Interfaces;
using DigitalTwin.Core.Services;
using DigitalTwin.Core.Plugins;
using DigitalTwin.API.Middleware;
using DigitalTwin.API.Hubs;
using DigitalTwin.API.Services;
using System.Security.Cryptography;
using OpenTelemetry.Resources;
using OpenTelemetry.Trace;
using Microsoft.Extensions.Http.Resilience;
using Microsoft.Extensions.Diagnostics.HealthChecks;
using Microsoft.AspNetCore.Diagnostics.HealthChecks;
using Polly;

namespace DigitalTwin.API
{
    public class Program
    {
        public static void Main(string[] args)
        {
            var builder = WebApplication.CreateBuilder(args);

            builder.Services.AddControllers(options =>
            {
                options.Filters.Add<DigitalTwin.API.Filters.ModelValidationFilter>();
            });
            builder.Services.AddEndpointsApiExplorer();
            builder.Services.AddSwaggerGen(c =>
            {
                c.SwaggerDoc("v1", new()
                {
                    Title = "Digital Twin Emotional Companion API",
                    Version = "v1",
                    Description = "RESTful API for emotional companion AI system with real-time emotion detection and conversation capabilities"
                });

                var xmlFile = $"{System.Reflection.Assembly.GetExecutingAssembly().GetName().Name}.xml";
                var xmlPath = Path.Combine(AppContext.BaseDirectory, xmlFile);
                if (File.Exists(xmlPath))
                {
                    c.IncludeXmlComments(xmlPath);
                }

                c.AddSecurityDefinition("Bearer", new()
                {
                    Description = "JWT Authorization header using the Bearer scheme. Example: \"Authorization: Bearer {token}\"",
                    Name = "Authorization",
                    In = Microsoft.OpenApi.Models.ParameterLocation.Header,
                    Type = Microsoft.OpenApi.Models.SecuritySchemeType.ApiKey,
                    Scheme = "Bearer"
                });

                c.AddSecurityRequirement(new()
                {
                    {
                        new()
                        {
                            Reference = new() { Type = Microsoft.OpenApi.Models.ReferenceType.SecurityScheme, Id = "Bearer" }
                        },
                        Array.Empty<string>()
                    }
                });
            });

            // Database — use standardized env var with fallback
            var connectionString = Environment.GetEnvironmentVariable("ConnectionStrings__DefaultConnection")
                ?? builder.Configuration.GetConnectionString("DefaultConnection");
            if (string.IsNullOrEmpty(connectionString))
            {
                if (builder.Environment.IsDevelopment())
                    connectionString = "Host=localhost;Database=digitaltwin;Username=postgres;Password=password";
                else
                    throw new InvalidOperationException("ConnectionStrings__DefaultConnection must be set in production");
            }

            builder.Services.AddDbContext<DigitalTwinDbContext>(options =>
                options.UseNpgsql(connectionString));

            // JWT Authentication — RS256 asymmetric signing (AD-1 security fix)
            var jwtIssuer = Environment.GetEnvironmentVariable("JwtConfiguration__Issuer")
                ?? builder.Configuration["JwtConfiguration:Issuer"]
                ?? "DigitalTwin";
            var jwtAudience = Environment.GetEnvironmentVariable("JwtConfiguration__Audience")
                ?? builder.Configuration["JwtConfiguration:Audience"]
                ?? "DigitalTwin";

            // Load RSA key for RS256 — private key path for signing, public for validation
            var rsaPrivateKeyPath = Environment.GetEnvironmentVariable("JwtConfiguration__PrivateKeyPath")
                ?? builder.Configuration["JwtConfiguration:PrivateKeyPath"];
            var rsaPublicKeyPath = Environment.GetEnvironmentVariable("JwtConfiguration__PublicKeyPath")
                ?? builder.Configuration["JwtConfiguration:PublicKeyPath"];

            SecurityKey jwtSigningKey;
            var rsa = RSA.Create();

            if (!string.IsNullOrEmpty(rsaPrivateKeyPath) && File.Exists(rsaPrivateKeyPath))
            {
                var privateKeyPem = File.ReadAllText(rsaPrivateKeyPath);
                rsa.ImportFromPem(privateKeyPem);
                jwtSigningKey = new RsaSecurityKey(rsa);
            }
            else if (!string.IsNullOrEmpty(rsaPublicKeyPath) && File.Exists(rsaPublicKeyPath))
            {
                var publicKeyPem = File.ReadAllText(rsaPublicKeyPath);
                rsa.ImportFromPem(publicKeyPem);
                jwtSigningKey = new RsaSecurityKey(rsa);
            }
            else if (builder.Environment.IsDevelopment())
            {
                // Development fallback: generate ephemeral RSA key pair
                jwtSigningKey = new RsaSecurityKey(rsa);
            }
            else
            {
                throw new InvalidOperationException(
                    "JwtConfiguration__PrivateKeyPath or JwtConfiguration__PublicKeyPath must be set in production");
            }

            // Store RSA instance for token generation in JwtAuthenticationService
            builder.Services.AddSingleton(new JwtSigningCredentials(
                new SigningCredentials(jwtSigningKey, SecurityAlgorithms.RsaSha256),
                jwtIssuer,
                jwtAudience));

            builder.Services.AddAuthentication(JwtBearerDefaults.AuthenticationScheme)
                .AddJwtBearer(options =>
                {
                    options.TokenValidationParameters = new TokenValidationParameters
                    {
                        ValidateIssuer = true,
                        ValidateAudience = true,
                        ValidateLifetime = true,
                        ValidateIssuerSigningKey = true,
                        ValidIssuer = jwtIssuer,
                        ValidAudience = jwtAudience,
                        IssuerSigningKey = jwtSigningKey
                    };

                    // Allow SignalR to receive JWT via query string
                    options.Events = new Microsoft.AspNetCore.Authentication.JwtBearer.JwtBearerEvents
                    {
                        OnMessageReceived = context =>
                        {
                            var accessToken = context.Request.Query["access_token"];
                            var path = context.HttpContext.Request.Path;
                            if (!string.IsNullOrEmpty(accessToken) && path.StartsWithSegments("/hubs"))
                            {
                                context.Token = accessToken;
                            }
                            return Task.CompletedTask;
                        }
                    };
                });

            builder.Services.AddAuthorization();

            // SignalR for real-time communication (shared experiences, emotion streaming)
            builder.Services.AddSignalR();

            // Distributed cache — Redis in production, in-memory for development
            var redisConnection = Environment.GetEnvironmentVariable("Redis__ConnectionString");
            if (!string.IsNullOrEmpty(redisConnection))
            {
                builder.Services.AddStackExchangeRedisCache(options =>
                {
                    options.Configuration = redisConnection;
                    options.InstanceName = "digitaltwin:";
                });
            }
            else
            {
                builder.Services.AddDistributedMemoryCache();
            }

            // HttpClient with named clients, resilience pipelines (Polly), and timeouts
            builder.Services.AddHttpClient("DeepFace", client =>
            {
                var baseUrl = Environment.GetEnvironmentVariable("Services__DeepFace__BaseUrl") ?? "http://localhost:8001";
                client.BaseAddress = new Uri(baseUrl);
                client.Timeout = Timeout.InfiniteTimeSpan; // Polly manages timeouts
            })
            .AddStandardResilienceHandler(options =>
            {
                options.TotalRequestTimeout.Timeout = TimeSpan.FromSeconds(30);
                options.Retry.MaxRetryAttempts = 3;
                options.Retry.Delay = TimeSpan.FromMilliseconds(500);
                options.Retry.BackoffType = DelayBackoffType.Exponential;
                options.Retry.UseJitter = true;
                options.CircuitBreaker.SamplingDuration = TimeSpan.FromSeconds(30);
                options.CircuitBreaker.FailureRatio = 0.5;
                options.CircuitBreaker.MinimumThroughput = 5;
                options.CircuitBreaker.BreakDuration = TimeSpan.FromSeconds(30);
                options.AttemptTimeout.Timeout = TimeSpan.FromSeconds(10);
            });
            builder.Services.AddHttpClient("LLM", client =>
            {
                var baseUrl = Environment.GetEnvironmentVariable("Services__LLM__BaseUrl") ?? "http://localhost:8004";
                client.BaseAddress = new Uri(baseUrl);
                client.Timeout = Timeout.InfiniteTimeSpan;
            })
            .AddStandardResilienceHandler(options =>
            {
                options.TotalRequestTimeout.Timeout = TimeSpan.FromSeconds(45);
                options.Retry.MaxRetryAttempts = 3;
                options.Retry.Delay = TimeSpan.FromMilliseconds(500);
                options.Retry.BackoffType = DelayBackoffType.Exponential;
                options.Retry.UseJitter = true;
                options.CircuitBreaker.SamplingDuration = TimeSpan.FromSeconds(30);
                options.CircuitBreaker.FailureRatio = 0.5;
                options.CircuitBreaker.MinimumThroughput = 5;
                options.CircuitBreaker.BreakDuration = TimeSpan.FromSeconds(30);
                options.AttemptTimeout.Timeout = TimeSpan.FromSeconds(15);
            });
            builder.Services.AddHttpClient("Avatar", client =>
            {
                var baseUrl = Environment.GetEnvironmentVariable("Services__Avatar__BaseUrl") ?? "http://localhost:8002";
                client.BaseAddress = new Uri(baseUrl);
                client.Timeout = Timeout.InfiniteTimeSpan;
            })
            .AddStandardResilienceHandler(options =>
            {
                options.TotalRequestTimeout.Timeout = TimeSpan.FromSeconds(90);
                options.Retry.MaxRetryAttempts = 3;
                options.Retry.Delay = TimeSpan.FromSeconds(1);
                options.Retry.BackoffType = DelayBackoffType.Exponential;
                options.Retry.UseJitter = true;
                options.CircuitBreaker.SamplingDuration = TimeSpan.FromSeconds(30);
                options.CircuitBreaker.FailureRatio = 0.5;
                options.CircuitBreaker.MinimumThroughput = 5;
                options.CircuitBreaker.BreakDuration = TimeSpan.FromSeconds(30);
                options.AttemptTimeout.Timeout = TimeSpan.FromSeconds(30);
            });
            builder.Services.AddHttpClient("Voice", client =>
            {
                var baseUrl = Environment.GetEnvironmentVariable("Services__Voice__BaseUrl") ?? "http://localhost:8003";
                client.BaseAddress = new Uri(baseUrl);
                client.Timeout = Timeout.InfiniteTimeSpan;
            })
            .AddStandardResilienceHandler(options =>
            {
                options.TotalRequestTimeout.Timeout = TimeSpan.FromSeconds(60);
                options.Retry.MaxRetryAttempts = 3;
                options.Retry.Delay = TimeSpan.FromMilliseconds(500);
                options.Retry.BackoffType = DelayBackoffType.Exponential;
                options.Retry.UseJitter = true;
                options.CircuitBreaker.SamplingDuration = TimeSpan.FromSeconds(30);
                options.CircuitBreaker.FailureRatio = 0.5;
                options.CircuitBreaker.MinimumThroughput = 5;
                options.CircuitBreaker.BreakDuration = TimeSpan.FromSeconds(30);
                options.AttemptTimeout.Timeout = TimeSpan.FromSeconds(20);
            });
            builder.Services.AddHttpClient("ExpoPush", client =>
            {
                client.BaseAddress = new Uri("https://exp.host/--/api/v2/push/");
                client.Timeout = Timeout.InfiniteTimeSpan;
            })
            .AddStandardResilienceHandler(options =>
            {
                options.TotalRequestTimeout.Timeout = TimeSpan.FromSeconds(30);
                options.Retry.MaxRetryAttempts = 2;
                options.Retry.Delay = TimeSpan.FromSeconds(1);
                options.Retry.BackoffType = DelayBackoffType.Exponential;
                options.Retry.UseJitter = true;
                options.CircuitBreaker.SamplingDuration = TimeSpan.FromSeconds(30);
                options.CircuitBreaker.FailureRatio = 0.5;
                options.CircuitBreaker.MinimumThroughput = 5;
                options.CircuitBreaker.BreakDuration = TimeSpan.FromSeconds(60);
                options.AttemptTimeout.Timeout = TimeSpan.FromSeconds(10);
            });
            builder.Services.AddHttpClient();

            // Stripe configuration
            var stripeSecretKey = Environment.GetEnvironmentVariable("Stripe__SecretKey");
            if (!string.IsNullOrEmpty(stripeSecretKey))
            {
                Stripe.StripeConfiguration.ApiKey = stripeSecretKey;
            }

            // Core services
            builder.Services.AddSingleton<API.Services.JwtAuthenticationService>();
            builder.Services.AddScoped<PasswordHasher>();
            builder.Services.AddScoped<AuthenticationService>();
            builder.Services.AddScoped<RoleBasedAccessControlService>();
            builder.Services.AddScoped<SecurityEventLogger>();
            builder.Services.AddScoped<IAITwinService, AITwinService>();
            builder.Services.AddScoped<IAnalyticsService, AnalyticsService>();
            builder.Services.AddScoped<IPredictiveAnalyticsService, PredictiveAnalyticsService>();
            builder.Services.AddScoped<IAlertService, AlertService>();
            builder.Services.AddScoped<IReportService, ReportService>();
            builder.Services.AddScoped<IExportService, ExportService>();
            builder.Services.AddScoped<IWebhookService, WebhookService>();
            builder.Services.AddScoped<IConversationService, ConversationService>();
            builder.Services.AddScoped<IEmotionalStateService, EmotionalStateService>();
            builder.Services.AddScoped<IEmbeddingService, EmbeddingService>();
            builder.Services.AddScoped<IEmotionFusionService, EmotionFusionService>();
            builder.Services.AddScoped<IUsageLimitService, UsageLimitService>();
            builder.Services.AddScoped<IProactiveCheckInService, ProactiveCheckInService>();
            builder.Services.AddScoped<IPushNotificationService, PushNotificationService>();

            // Encryption — optional, enabled when Encryption__Key is set (AD-4 compliant)
            var encryptionKey = Environment.GetEnvironmentVariable("Encryption__Key");
            if (!string.IsNullOrEmpty(encryptionKey))
            {
                builder.Services.AddSingleton<IEncryptionService, EncryptionService>();
            }

            // Plugin system
            builder.Services.AddScoped<ICompanionPlugin, SafetyPlugin>();
            builder.Services.AddScoped<ICompanionPlugin, MoodTrackingPlugin>();
            builder.Services.AddScoped<ICompanionPlugin, PersonalityPlugin>();
            builder.Services.AddScoped<IPluginManager, PluginManager>();

            // Biometric, coaching, shared experience services
            builder.Services.AddScoped<IBiometricService, BiometricService>();
            builder.Services.AddScoped<ICoachingService, CoachingService>();
            builder.Services.AddScoped<ISharedExperienceService, SharedExperienceService>();
            builder.Services.AddScoped<IPersonalHistoryService, PersonalHistoryService>();
            builder.Services.AddScoped<IFamilyService, FamilyService>();
            builder.Services.AddScoped<IAchievementService, AchievementService>();
            builder.Services.AddScoped<ICommunityService, CommunityService>();
            builder.Services.AddScoped<IModerationService, ModerationService>();
            builder.Services.AddScoped<ICreativeService, CreativeService>();
            builder.Services.AddScoped<ITherapyService, TherapyService>();
            builder.Services.AddScoped<ILearningService, LearningService>();

            // Rate limiting — Redis-backed in production, in-memory fallback
            if (!string.IsNullOrEmpty(redisConnection))
            {
                builder.Services.AddSingleton<IRateLimitService, RedisRateLimitService>();
            }
            else
            {
                builder.Services.AddSingleton<IRateLimitService, InMemoryRateLimitService>();
            }

            // Event bus — RabbitMQ in production, in-memory fallback for dev
            var rabbitMqConnection = Environment.GetEnvironmentVariable("RabbitMQ__ConnectionString");
            if (!string.IsNullOrEmpty(rabbitMqConnection))
            {
                builder.Services.AddSingleton<IEventBus, RabbitMqEventBus>();
            }
            else
            {
                builder.Services.AddSingleton<IEventBus, InMemoryEventBus>();
            }

            // CORS — restrict in production
            var allowedOrigins = Environment.GetEnvironmentVariable("CORS__AllowedOrigins")?.Split(',')
                ?? new[] { "http://localhost:3000", "http://localhost:8081", "http://localhost:19006" };

            builder.Services.AddCors(options =>
            {
                options.AddPolicy("Default", policy =>
                {
                    policy.WithOrigins(allowedOrigins)
                          .WithMethods("GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS")
                          .WithHeaders("Content-Type", "Authorization", "X-Service-Key")
                          .AllowCredentials();
                });
            });

            // OpenTelemetry distributed tracing — exports to Jaeger via OTLP
            var otlpEndpoint = Environment.GetEnvironmentVariable("OTEL__ExporterOtlpEndpoint")
                ?? "http://localhost:4317";

            builder.Services.AddOpenTelemetry()
                .ConfigureResource(resource => resource
                    .AddService(
                        serviceName: "digitaltwin-api",
                        serviceVersion: "1.0.0"))
                .WithTracing(tracing => tracing
                    .AddAspNetCoreInstrumentation(opts =>
                    {
                        opts.RecordException = true;
                        opts.Filter = ctx => !ctx.Request.Path.StartsWithSegments("/health")
                                          && !ctx.Request.Path.StartsWithSegments("/metrics");
                    })
                    .AddHttpClientInstrumentation(opts => opts.RecordException = true)
                    .AddEntityFrameworkCoreInstrumentation(opts => opts.SetDbStatementForText = true)
                    .AddSource("DigitalTwin.API")
                    .AddOtlpExporter(opts =>
                    {
                        opts.Endpoint = new Uri(otlpEndpoint);
                    }));

            // Structured JSON logging to console (for ELK ingestion via Docker log driver)
            builder.Logging.ClearProviders();
            builder.Logging.AddJsonConsole(options =>
            {
                options.IncludeScopes = true;
                options.TimestampFormat = "yyyy-MM-ddTHH:mm:ss.fffZ";
                options.JsonWriterOptions = new System.Text.Json.JsonWriterOptions { Indented = false };
            });

            // Aggregated health checks — checks all dependencies
            builder.Services.AddHealthChecks()
                .AddNpgSql(connectionString, name: "postgres", tags: new[] { "db" })
                .AddUrlGroup(new Uri(
                    (Environment.GetEnvironmentVariable("Services__DeepFace__BaseUrl") ?? "http://localhost:8001") + "/health"),
                    name: "deepface", tags: new[] { "microservice" })
                .AddUrlGroup(new Uri(
                    (Environment.GetEnvironmentVariable("Services__LLM__BaseUrl") ?? "http://localhost:8004") + "/health"),
                    name: "llm", tags: new[] { "microservice" })
                .AddUrlGroup(new Uri(
                    (Environment.GetEnvironmentVariable("Services__Avatar__BaseUrl") ?? "http://localhost:8002") + "/health"),
                    name: "avatar", tags: new[] { "microservice" })
                .AddUrlGroup(new Uri(
                    (Environment.GetEnvironmentVariable("Services__Voice__BaseUrl") ?? "http://localhost:8003") + "/health"),
                    name: "voice", tags: new[] { "microservice" });

            if (!string.IsNullOrEmpty(redisConnection))
            {
                builder.Services.AddHealthChecks()
                    .AddRedis(redisConnection, name: "redis", tags: new[] { "cache" });
            }

            // Graceful shutdown configuration
            builder.Host.ConfigureHostOptions(options =>
            {
                options.ShutdownTimeout = TimeSpan.FromSeconds(30);
            });

            var app = builder.Build();

            // Global exception handling — must be first in pipeline
            app.UseMiddleware<ExceptionHandlingMiddleware>();

            // Swagger — available in all environments
            app.UseSwagger();
            if (app.Environment.IsDevelopment())
            {
                app.UseSwaggerUI(c =>
                {
                    c.SwaggerEndpoint("/swagger/v1/swagger.json", "Digital Twin Emotional Companion API v1");
                    c.RoutePrefix = string.Empty;
                });
            }
            else
            {
                app.UseSwaggerUI(c =>
                {
                    c.SwaggerEndpoint("/swagger/v1/swagger.json", "Digital Twin Emotional Companion API v1");
                    c.RoutePrefix = "api-docs";
                });
            }

            app.UseHttpsRedirection();
            app.UseCors("Default");
            app.UseAuthentication();
            app.UseAuthorization();
            app.UseMiddleware<DigitalTwin.API.Middleware.RateLimitingMiddleware>();
            app.UseUsageLimitMiddleware();

            app.UseHttpMetrics();

            app.MapControllers();
            app.MapMetrics();
            app.MapHub<CompanionHub>("/hubs/companion");

            // Aggregated health check — readiness probe (checks all dependencies)
            app.MapHealthChecks("/health", new HealthCheckOptions
            {
                ResponseWriter = async (context, report) =>
                {
                    context.Response.ContentType = "application/json";
                    var result = new
                    {
                        status = report.Status.ToString().ToLower(),
                        timestamp = DateTime.UtcNow,
                        service = "Digital Twin Emotional Companion API",
                        version = "1.0.0",
                        totalDuration = report.TotalDuration.TotalMilliseconds,
                        checks = report.Entries.Select(e => new
                        {
                            name = e.Key,
                            status = e.Value.Status.ToString().ToLower(),
                            duration = e.Value.Duration.TotalMilliseconds,
                            error = e.Value.Exception?.Message
                        })
                    };
                    await context.Response.WriteAsJsonAsync(result);
                }
            });

            // Lightweight liveness probe — no dependency checks (for K8s livenessProbe)
            app.MapGet("/health/live", () => Results.Ok(new { status = "alive", timestamp = DateTime.UtcNow }));

            app.MapGet("/", () => new
            {
                name = "Digital Twin Emotional Companion API",
                version = "1.0.0",
                description = "RESTful API for emotional companion AI system",
                endpoints = new
                {
                    swagger = "/swagger",
                    health = "/health",
                    conversation = "/api/conversation"
                }
            });

            app.Run();
        }
    }
}
