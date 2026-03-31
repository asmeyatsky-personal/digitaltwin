using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using DigitalTwin.Core.Interfaces;
using DigitalTwin.Core.Entities;
using DigitalTwin.Core.DTOs;

namespace DigitalTwin.Core.Repositories
{
    public class InMemoryWebhookRepository : IWebhookRepository
    {
        private readonly List<WebhookSubscription> _webhooks = new();

        public Task<WebhookSubscription> GetByIdAsync(Guid id) =>
            Task.FromResult(_webhooks.Find(w => w.Id == id));

        public Task<List<WebhookSubscription>> GetAllAsync() =>
            Task.FromResult(new List<WebhookSubscription>(_webhooks));

        public Task<List<WebhookSubscription>> GetByBuildingIdAsync(Guid buildingId) =>
            Task.FromResult(_webhooks.FindAll(w => w.BuildingId == buildingId));

        public Task<WebhookSubscription> AddAsync(WebhookSubscription webhook)
        {
            webhook.Id = Guid.NewGuid();
            webhook.CreatedAt = DateTime.UtcNow;
            _webhooks.Add(webhook);
            return Task.FromResult(webhook);
        }

        public Task<WebhookSubscription> UpdateAsync(WebhookSubscription webhook)
        {
            var idx = _webhooks.FindIndex(w => w.Id == webhook.Id);
            if (idx >= 0) _webhooks[idx] = webhook;
            return Task.FromResult(webhook);
        }

        public Task<bool> DeleteAsync(Guid id)
        {
            var removed = _webhooks.RemoveAll(w => w.Id == id);
            return Task.FromResult(removed > 0);
        }
    }

    public class InMemoryWebhookDeliveryRepository : IWebhookDeliveryRepository
    {
        private readonly List<WebhookDelivery> _deliveries = new();

        public Task<WebhookDelivery> GetByIdAsync(Guid id) =>
            Task.FromResult(_deliveries.Find(d => d.Id == id));

        public Task<List<WebhookDelivery>> GetByWebhookIdAsync(Guid webhookId) =>
            Task.FromResult(_deliveries.FindAll(d => d.WebhookId == webhookId));

        public Task<List<WebhookDelivery>> GetFailedDeliveriesAsync() =>
            Task.FromResult(_deliveries.FindAll(d => d.Status == WebhookDeliveryStatus.Failed));

        public Task<List<WebhookDelivery>> GetRecentDeliveriesAsync(DateTime since) =>
            Task.FromResult(_deliveries.FindAll(d => d.AttemptedAt >= since));

        public Task<PaginatedResult<WebhookDelivery>> GetWithPaginationAsync(int page, int pageSize, Guid? webhookId = null, WebhookDeliveryStatus? status = null, DateTime? startDate = null, DateTime? endDate = null)
        {
            var query = _deliveries.AsEnumerable();
            if (webhookId.HasValue) query = query.Where(d => d.WebhookId == webhookId.Value);
            if (status.HasValue) query = query.Where(d => d.Status == status.Value);
            if (startDate.HasValue) query = query.Where(d => d.AttemptedAt >= startDate.Value);
            if (endDate.HasValue) query = query.Where(d => d.AttemptedAt <= endDate.Value);

            var list = query.ToList();
            var paged = list.Skip((page - 1) * pageSize).Take(pageSize).ToList();
            return Task.FromResult(new PaginatedResult<WebhookDelivery>
            {
                Items = paged,
                TotalCount = list.Count,
                Page = page,
                PageSize = pageSize
            });
        }

        public Task<WebhookDelivery> AddAsync(WebhookDelivery delivery)
        {
            delivery.Id = Guid.NewGuid();
            delivery.AttemptedAt = DateTime.UtcNow;
            _deliveries.Add(delivery);
            return Task.FromResult(delivery);
        }

        public Task<WebhookDelivery> UpdateAsync(WebhookDelivery delivery)
        {
            var idx = _deliveries.FindIndex(d => d.Id == delivery.Id);
            if (idx >= 0) _deliveries[idx] = delivery;
            return Task.FromResult(delivery);
        }

        public Task<bool> DeleteAsync(Guid id)
        {
            var removed = _deliveries.RemoveAll(d => d.Id == id);
            return Task.FromResult(removed > 0);
        }
    }

    public class InMemoryBuildingRepository : IBuildingRepository
    {
        private readonly List<Building> _buildings = new();

        public Task<Building> GetByIdAsync(Guid id) => Task.FromResult(_buildings.Find(b => b.Id == id));
        public Task<List<Building>> GetAllAsync() => Task.FromResult(new List<Building>(_buildings));
        public Task<List<Building>> GetByUserIdAsync(string userId) => Task.FromResult(new List<Building>());
        public Task<Building> AddAsync(Building building) { building.Id = Guid.NewGuid(); _buildings.Add(building); return Task.FromResult(building); }
        public Task<Building> UpdateAsync(Building building) { var i = _buildings.FindIndex(b => b.Id == building.Id); if (i >= 0) _buildings[i] = building; return Task.FromResult(building); }
        public Task<bool> DeleteAsync(Guid id) => Task.FromResult(_buildings.RemoveAll(b => b.Id == id) > 0);
    }

    public class InMemorySensorRepository : ISensorRepository
    {
        private readonly List<Sensor> _sensors = new();
        private readonly List<SensorReading> _readings = new();

        public Task<Sensor> GetByIdAsync(Guid id) => Task.FromResult(_sensors.Find(s => s.Id == id));
        public Task<List<Sensor>> GetAllAsync() => Task.FromResult(new List<Sensor>(_sensors));
        public Task<List<Sensor>> GetByBuildingIdAsync(Guid buildingId) => Task.FromResult(new List<Sensor>());
        public Task<List<Sensor>> GetByRoomIdAsync(Guid roomId) => Task.FromResult(_sensors.FindAll(s => s.RoomId == roomId));
        public Task<Sensor> AddAsync(Sensor sensor) { sensor.Id = Guid.NewGuid(); _sensors.Add(sensor); return Task.FromResult(sensor); }
        public Task<Sensor> UpdateAsync(Sensor sensor) { var i = _sensors.FindIndex(s => s.Id == sensor.Id); if (i >= 0) _sensors[i] = sensor; return Task.FromResult(sensor); }
        public Task<bool> DeleteAsync(Guid id) => Task.FromResult(_sensors.RemoveAll(s => s.Id == id) > 0);
        
        public Task<List<SensorReading>> GetReadingsAsync(Guid sensorId, DateTime? start = null, DateTime? end = null)
        {
            var query = _readings.Where(r => r.SensorId == sensorId);
            if (start.HasValue) query = query.Where(r => r.Timestamp >= start.Value);
            if (end.HasValue) query = query.Where(r => r.Timestamp <= end.Value);
            return Task.FromResult(query.ToList());
        }
        
        public Task<IEnumerable<Sensor>> GetByBuildingAndTypeAsync(Guid buildingId, SensorType type) =>
            Task.FromResult(Enumerable.Empty<Sensor>());
        
        public Task<IEnumerable<Sensor>> GetByBuildingAsync(Guid buildingId) =>
            Task.FromResult(Enumerable.Empty<Sensor>());
    }

    public class InMemoryAITwinRepository : IAITwinRepository
    {
        private readonly List<AITwinProfile> _aitwins = new();

        public Task<AITwinProfile> GetByIdAsync(Guid id) => Task.FromResult(_aitwins.Find(a => a.Id == id));
        public Task<List<AITwinProfile>> GetAllAsync() => Task.FromResult(new List<AITwinProfile>(_aitwins));
        public Task<List<AITwinProfile>> GetByUserIdAsync(string userId) => Task.FromResult(_aitwins.FindAll(a => a.UserId == userId));
        public Task<List<AITwinProfile>> GetByBuildingIdAsync(Guid buildingId) => Task.FromResult(_aitwins.FindAll(a => a.BuildingId == buildingId));
        public Task<int> GetCountByUserIdAsync(string userId) => Task.FromResult(_aitwins.Count(a => a.UserId == userId));
        public Task<AITwinProfile> AddAsync(AITwinProfile profile) { profile.Id = Guid.NewGuid(); _aitwins.Add(profile); return Task.FromResult(profile); }
        public Task<AITwinProfile> UpdateAsync(AITwinProfile profile) { var i = _aitwins.FindIndex(a => a.Id == profile.Id); if (i >= 0) _aitwins[i] = profile; return Task.FromResult(profile); }
        public Task<bool> DeleteAsync(Guid id) => Task.FromResult(_aitwins.RemoveAll(a => a.Id == id) > 0);
    }

    public class InMemoryAlertRuleRepository : IAlertRuleRepository
    {
        private readonly List<AlertRule> _rules = new();

        public Task<AlertRule> GetByIdAsync(Guid id) => Task.FromResult(_rules.Find(r => r.Id == id));
        public Task<List<AlertRule>> GetAllAsync() => Task.FromResult(new List<AlertRule>(_rules));
        public Task<List<AlertRule>> GetByBuildingIdAsync(Guid buildingId) => Task.FromResult(_rules.FindAll(r => r.BuildingId == buildingId));
        public Task<List<AlertRule>> GetBySensorIdAsync(Guid sensorId) => Task.FromResult(_rules.FindAll(r => r.Id == sensorId));
        public Task<AlertRule> AddAsync(AlertRule rule) { rule.Id = Guid.NewGuid(); rule.CreatedAt = DateTime.UtcNow; _rules.Add(rule); return Task.FromResult(rule); }
        public Task<AlertRule> UpdateAsync(AlertRule rule) { var i = _rules.FindIndex(r => r.Id == rule.Id); if (i >= 0) _rules[i] = rule; return Task.FromResult(rule); }
        public Task<bool> DeleteAsync(Guid id) => Task.FromResult(_rules.RemoveAll(r => r.Id == id) > 0);
    }

    public class InMemoryAlertRepository : IAlertRepository
    {
        private readonly List<Alert> _alerts = new();
        private readonly List<AlertSubscription> _subscriptions = new();

        public Task<Alert> GetByIdAsync(Guid id) => Task.FromResult(_alerts.Find(a => a.Id == id));
        public Task<List<Alert>> GetAllAsync(DateTime? startDate = null, DateTime? endDate = null)
        {
            var query = _alerts.AsEnumerable();
            if (startDate.HasValue) query = query.Where(a => a.Timestamp >= startDate.Value);
            if (endDate.HasValue) query = query.Where(a => a.Timestamp <= endDate.Value);
            return Task.FromResult(query.ToList());
        }
        public Task<List<Alert>> GetActiveAlertsAsync() => Task.FromResult(_alerts.FindAll(a => a.Status == AlertStatus.Active));
        public Task<List<Alert>> GetByBuildingIdAsync(Guid buildingId, DateTime? startDate = null, DateTime? endDate = null) => Task.FromResult(new List<Alert>());
        public Task<PaginatedResult<Alert>> GetWithPaginationAsync(int page, int pageSize, DateTime? startDate = null, DateTime? endDate = null, AlertSeverity? severity = null, AlertStatus? status = null, string type = null)
        {
            var query = _alerts.AsEnumerable();
            if (startDate.HasValue) query = query.Where(a => a.Timestamp >= startDate.Value);
            if (endDate.HasValue) query = query.Where(a => a.Timestamp <= endDate.Value);
            if (severity.HasValue) query = query.Where(a => a.Severity == severity.Value);
            if (status.HasValue) query = query.Where(a => a.Status == status.Value);
            var list = query.ToList();
            return Task.FromResult(new PaginatedResult<Alert>
            {
                Items = list.Skip((page - 1) * pageSize).Take(pageSize).ToList(),
                TotalCount = list.Count,
                Page = page,
                PageSize = pageSize
            });
        }
        public Task<List<Alert>> GetRecentAlertsAsync(Guid ruleId, TimeSpan timeWindow) => Task.FromResult(new List<Alert>());
        public Task<Alert> AddAsync(Alert alert) { alert.Id = Guid.NewGuid(); alert.Timestamp = DateTime.UtcNow; _alerts.Add(alert); return Task.FromResult(alert); }
        public Task<Alert> UpdateAsync(Alert alert) { var i = _alerts.FindIndex(a => a.Id == alert.Id); if (i >= 0) _alerts[i] = alert; return Task.FromResult(alert); }
        public Task<bool> DeleteAsync(Guid id) => Task.FromResult(_alerts.RemoveAll(a => a.Id == id) > 0);
        public Task<List<AlertSubscription>> GetUserSubscriptionsAsync(string userId) => Task.FromResult(_subscriptions.FindAll(s => s.UserId == userId));
        public Task<List<AlertSubscription>> GetSubscriptionsByBuildingIdAsync(Guid buildingId) => Task.FromResult(_subscriptions.FindAll(s => s.BuildingId == buildingId));
        public Task<List<AlertSubscription>> GetEscalationSubscriptionsAsync(Guid buildingId, AlertSeverity severity) => Task.FromResult(new List<AlertSubscription>());
        public Task<AlertSubscription> AddSubscriptionAsync(AlertSubscription subscription) { subscription.Id = Guid.NewGuid(); subscription.SubscribedAt = DateTime.UtcNow; _subscriptions.Add(subscription); return Task.FromResult(subscription); }
        public Task<bool> DeleteSubscriptionAsync(Guid subscriptionId) => Task.FromResult(_subscriptions.RemoveAll(s => s.Id == subscriptionId) > 0);
    }

    public class StubDataCollectionService : IDataCollectionService
    {
        public Task<List<SensorReading>> CollectReadingsAsync(Guid buildingId) => Task.FromResult(new List<SensorReading>());
        public Task<SensorReading> GetLatestReadingAsync(Guid sensorId) => Task.FromResult<SensorReading>(null!);
        public Task<bool> StartCollectionAsync(Guid buildingId) => Task.FromResult(true);
        public Task<bool> StopCollectionAsync(Guid buildingId) => Task.FromResult(true);
        public Task<IEnumerable<SensorReading>> GetSensorReadingsAsync(Guid sensorId, DateTime startDate, DateTime endDate) => Task.FromResult(Enumerable.Empty<SensorReading>());
    }

    public class StubLLMService : ILLMService
    {
        public Task<LLMResponse> GenerateResponseAsync(LLMRequest request) => Task.FromResult(new LLMResponse { Content = "This is a stub response.", Confidence = 0.8 });
        public Task<LLMTrainingResult> FineTuneAsync(LLMTrainingRequest request) => Task.FromResult(new LLMTrainingResult { Success = true });
        public Task<double> EvaluateResponseQualityAsync(string prompt, string response, string expectedResponse = null) => Task.FromResult(0.8);
        public Task<List<LLMModel>> GetAvailableModelsAsync() => Task.FromResult(new List<LLMModel>());
        public Task<bool> SetLLMConfigurationAsync(LLMConfiguration config) => Task.FromResult(true);
    }

    public class StubNotificationService : INotificationService
    {
        public Task<bool> SendNotificationAsync(NotificationRequest request) => Task.FromResult(true);
        public Task<BulkNotificationResult> SendBulkNotificationsAsync(List<NotificationRequest> requests) => Task.FromResult(new BulkNotificationResult { TotalRequested = requests.Count, Successful = requests.Count });
        public Task<NotificationStatus> GetNotificationStatusAsync(Guid notificationId) => Task.FromResult(NotificationStatus.Sent);
        public Task<List<NotificationRecord>> GetNotificationHistoryAsync(string userId, DateTime? startDate = null, DateTime? endDate = null) => Task.FromResult(new List<NotificationRecord>());
        public Task<List<NotificationChannelInfo>> GetSupportedChannelsAsync() => Task.FromResult(new List<NotificationChannelInfo>());
        public Task<ChannelTestResult> TestNotificationChannelAsync(AlertNotificationChannel channel, string testRecipient) => Task.FromResult(new ChannelTestResult { Success = true, Channel = channel });
    }

    public class StubWebhookEventProcessor : IWebhookEventProcessor
    {
        public string EventType => "stub";

        public Task<WebhookEvent> ProcessEventAsync(object eventData)
        {
            return Task.FromResult(new WebhookEvent
            {
                Id = Guid.NewGuid(),
                EventType = EventType,
                Timestamp = DateTime.UtcNow,
                Data = eventData
            });
        }

        public Task<bool> ValidateEventAsync(WebhookEvent webhookEvent)
        {
            return Task.FromResult(true);
        }
    }
}
