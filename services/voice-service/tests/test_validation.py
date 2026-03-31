"""Tests for request validation and authentication."""

import pytest
from httpx import ASGITransport, AsyncClient

from conftest import create_test_audio_bytes


class TestAuthValidation:
    async def test_401_without_api_key(self, test_client):
        """Requests without X-Service-Key should be rejected."""
        resp = await test_client.get("/voices")
        assert resp.status_code == 401

    async def test_401_with_wrong_api_key(self, test_client):
        """Requests with an incorrect key should be rejected."""
        resp = await test_client.get(
            "/voices",
            headers={"X-Service-Key": "wrong-key"},
        )
        assert resp.status_code == 401

    async def test_503_when_service_not_configured(self):
        """When SERVICE_API_KEY is empty, non-health endpoints return 503."""
        import main

        original_key = main.SERVICE_API_KEY
        main.SERVICE_API_KEY = ""

        try:
            transport = ASGITransport(app=main.app)
            async with AsyncClient(
                transport=transport, base_url="http://test"
            ) as client:
                resp = await client.get("/voices")
                assert resp.status_code == 503
        finally:
            main.SERVICE_API_KEY = original_key


class TestFileValidation:
    async def test_stt_missing_file_returns_422(
        self, test_client, service_api_headers
    ):
        """Posting to /voice/stt without a file should return 422."""
        resp = await test_client.post(
            "/voice/stt",
            headers=service_api_headers,
        )
        assert resp.status_code == 422

    async def test_clone_missing_files_returns_422(
        self, test_client, service_api_headers
    ):
        """Posting to /voice/clone without required fields returns 422."""
        resp = await test_client.post(
            "/voice/clone",
            headers=service_api_headers,
        )
        assert resp.status_code == 422

    async def test_user_voice_not_found(
        self, test_client, service_api_headers
    ):
        """Getting voice for a nonexistent user returns 404."""
        resp = await test_client.get(
            "/voice/user/nonexistent-user",
            headers=service_api_headers,
        )
        assert resp.status_code == 404
