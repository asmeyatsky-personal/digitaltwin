using Microsoft.Extensions.Caching.Distributed;
using System.Collections.Concurrent;
using System.Text.Json;

namespace DigitalTwin.API.Services
{
    public interface IRateLimitService
    {
        Task<bool> IsAllowedAsync(string key, int limit, TimeSpan window);
        Task<int> GetCurrentCountAsync(string key);
        Task<TimeSpan?> GetResetTimeAsync(string key);
    }

    public class RedisRateLimitService : IRateLimitService
    {
        private readonly IDistributedCache _cache;

        public RedisRateLimitService(IDistributedCache cache)
        {
            _cache = cache;
        }

        public async Task<bool> IsAllowedAsync(string key, int limit, TimeSpan window)
        {
            var currentCount = await GetCurrentCountAsync(key);

            if (currentCount >= limit)
                return false;

            var newCount = currentCount + 1;
            var data = new RateLimitData
            {
                Count = newCount,
                ResetTime = DateTime.UtcNow.Add(window)
            };

            await _cache.SetStringAsync(key, JsonSerializer.Serialize(data), new DistributedCacheEntryOptions
            {
                AbsoluteExpirationRelativeToNow = window
            });

            return true;
        }

        public async Task<int> GetCurrentCountAsync(string key)
        {
            var json = await _cache.GetStringAsync(key);
            if (string.IsNullOrEmpty(json))
                return 0;

            try
            {
                return JsonSerializer.Deserialize<RateLimitData>(json)?.Count ?? 0;
            }
            catch
            {
                return 0;
            }
        }

        public async Task<TimeSpan?> GetResetTimeAsync(string key)
        {
            var json = await _cache.GetStringAsync(key);
            if (string.IsNullOrEmpty(json))
                return null;

            try
            {
                var data = JsonSerializer.Deserialize<RateLimitData>(json);
                return data?.ResetTime - DateTime.UtcNow;
            }
            catch
            {
                return null;
            }
        }
    }

    public class InMemoryRateLimitService : IRateLimitService
    {
        private readonly ConcurrentDictionary<string, RateLimitData> _cache = new();
        private readonly object _lock = new();

        public Task<bool> IsAllowedAsync(string key, int limit, TimeSpan window)
        {
            lock (_lock)
            {
                var now = DateTime.UtcNow;

                if (_cache.TryGetValue(key, out var data))
                {
                    if (now >= data.ResetTime)
                    {
                        data = new RateLimitData { Count = 1, ResetTime = now.Add(window) };
                        _cache[key] = data;
                        return Task.FromResult(true);
                    }

                    if (data.Count >= limit)
                        return Task.FromResult(false);

                    data.Count++;
                    return Task.FromResult(true);
                }

                _cache[key] = new RateLimitData { Count = 1, ResetTime = now.Add(window) };
                return Task.FromResult(true);
            }
        }

        public Task<int> GetCurrentCountAsync(string key)
        {
            lock (_lock)
            {
                return Task.FromResult(_cache.TryGetValue(key, out var data) ? data.Count : 0);
            }
        }

        public Task<TimeSpan?> GetResetTimeAsync(string key)
        {
            lock (_lock)
            {
                if (_cache.TryGetValue(key, out var data))
                {
                    var remaining = data.ResetTime - DateTime.UtcNow;
                    return Task.FromResult<TimeSpan?>(remaining > TimeSpan.Zero ? remaining : null);
                }
                return Task.FromResult<TimeSpan?>(null);
            }
        }
    }

    internal class RateLimitData
    {
        public int Count { get; set; }
        public DateTime ResetTime { get; set; }
    }
}
