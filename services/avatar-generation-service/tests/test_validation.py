"""Tests for request validation and authentication."""

import pytest
from httpx import ASGITransport, AsyncClient

from conftest import create_test_image_bytes


class TestAuthValidation:
    async def test_401_without_api_key(self, test_client):
        """Requests without X-Service-Key should be rejected."""
        image_bytes = create_test_image_bytes()
        resp = await test_client.post(
            "/avatar/generate",
            files={"file": ("test.png", image_bytes, "image/png")},
        )
        assert resp.status_code == 401

    async def test_401_with_wrong_api_key(self, test_client):
        """Requests with an incorrect key should be rejected."""
        image_bytes = create_test_image_bytes()
        resp = await test_client.post(
            "/avatar/generate",
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
                    "/avatar/generate",
                    files={"file": ("test.png", image_bytes, "image/png")},
                )
                assert resp.status_code == 503
        finally:
            main.SERVICE_API_KEY = original_key


class TestFileValidation:
    async def test_missing_file_returns_422(
        self, test_client, service_api_headers
    ):
        """Posting to /avatar/generate without a file returns 422."""
        resp = await test_client.post(
            "/avatar/generate",
            headers=service_api_headers,
        )
        assert resp.status_code == 422

    async def test_extract_landmarks_missing_file_returns_422(
        self, test_client, service_api_headers
    ):
        """Posting to /avatar/extract-landmarks without a file returns 422."""
        resp = await test_client.post(
            "/avatar/extract-landmarks",
            headers=service_api_headers,
        )
        assert resp.status_code == 422

    async def test_download_nonexistent_avatar_returns_404(
        self, test_client, service_api_headers
    ):
        """Downloading a nonexistent avatar returns 404."""
        resp = await test_client.get(
            "/avatar/nonexistent-id/download",
            headers=service_api_headers,
        )
        assert resp.status_code == 404

    async def test_delete_nonexistent_avatar_returns_404(
        self, test_client, service_api_headers
    ):
        """Deleting a nonexistent avatar returns 404."""
        resp = await test_client.delete(
            "/avatar/nonexistent-id",
            headers=service_api_headers,
        )
        assert resp.status_code == 404
