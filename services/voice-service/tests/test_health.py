"""Tests for the /health endpoint."""

import pytest


class TestHealthEndpoint:
    async def test_health_returns_200(self, test_client):
        resp = await test_client.get("/health")
        assert resp.status_code == 200

    async def test_health_response_shape(self, test_client):
        resp = await test_client.get("/health")
        data = resp.json()
        assert data["status"] == "healthy"
        assert data["service"] == "voice-service"
        assert data["version"] == "1.0.0"
        assert "elevenlabs_configured" in data

    async def test_health_no_auth_required(self, test_client):
        """Health endpoint must work without X-Service-Key."""
        resp = await test_client.get("/health")
        assert resp.status_code == 200

    async def test_health_elevenlabs_not_configured(self, test_client):
        """Without ELEVENLABS_API_KEY the health check should report it."""
        import main

        original = main.ELEVENLABS_API_KEY
        main.ELEVENLABS_API_KEY = ""
        try:
            resp = await test_client.get("/health")
            data = resp.json()
            assert data["elevenlabs_configured"] is False
        finally:
            main.ELEVENLABS_API_KEY = original
