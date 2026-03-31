"""Tests for request validation and authentication."""

import pytest
from unittest.mock import MagicMock

from httpx import ASGITransport, AsyncClient

from conftest import create_test_image_bytes


class TestAuthValidation:
    async def test_401_without_api_key(self, test_client):
        """Requests without X-Service-Key should be rejected."""
        image_bytes = create_test_image_bytes()
        resp = await test_client.post(
            "/analyze/facial-expression",
            files={"file": ("test.png", image_bytes, "image/png")},
            # no headers
        )
        assert resp.status_code == 401

    async def test_401_with_wrong_api_key(self, test_client):
        """Requests with an incorrect key should be rejected."""
        image_bytes = create_test_image_bytes()
        resp = await test_client.post(
            "/analyze/facial-expression",
            files={"file": ("test.png", image_bytes, "image/png")},
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
                image_bytes = create_test_image_bytes()
                resp = await client.post(
                    "/analyze/facial-expression",
                    files={"file": ("test.png", image_bytes, "image/png")},
                )
                assert resp.status_code == 503
        finally:
            main.SERVICE_API_KEY = original_key


class TestFileValidation:
    async def test_missing_file_returns_422(
        self, test_client, service_api_headers
    ):
        """Posting without a file should return 422 Unprocessable Entity."""
        resp = await test_client.post(
            "/analyze/facial-expression",
            headers=service_api_headers,
        )
        assert resp.status_code == 422

    async def test_text_emotion_detection_with_empty_text(
        self, test_client, service_api_headers
    ):
        """Empty text should return neutral with 0.5 confidence."""
        resp = await test_client.post(
            "/detect-emotion",
            json={"text": ""},
            headers=service_api_headers,
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["emotion"] == "neutral"
        assert data["confidence"] == 0.5

    async def test_text_emotion_detection_happy(
        self, test_client, service_api_headers
    ):
        """Text with happy keywords should detect happy emotion."""
        resp = await test_client.post(
            "/detect-emotion",
            json={"text": "I feel so happy and excited today!"},
            headers=service_api_headers,
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["emotion"] == "happy"
        assert data["confidence"] > 0.0
