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
        assert data["service"] == "avatar-generation-service"
        assert data["version"] == "1.0.0"
        assert "models_loaded" in data

    async def test_health_no_auth_required(self, test_client):
        """Health endpoint must work without X-Service-Key."""
        resp = await test_client.get("/health")
        assert resp.status_code == 200
