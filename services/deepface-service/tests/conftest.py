"""Shared fixtures for deepface-service tests."""

import os
import sys
from unittest.mock import MagicMock, patch

import pytest
from httpx import ASGITransport, AsyncClient

# Ensure the service root is importable
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))


@pytest.fixture(autouse=True)
def _clean_env(monkeypatch):
    """Ensure no real API keys leak into tests."""
    monkeypatch.delenv("SERVICE_API_KEY", raising=False)


@pytest.fixture
def service_api_key(monkeypatch):
    """Set SERVICE_API_KEY and return the value."""
    key = "test-service-key"
    monkeypatch.setenv("SERVICE_API_KEY", key)
    return key


@pytest.fixture
def service_api_headers(service_api_key):
    """Headers dict with the service API key."""
    return {"X-Service-Key": service_api_key}


@pytest.fixture
async def test_client(service_api_key):
    """AsyncClient wired to the FastAPI app with mocked dependencies."""
    import main

    main.SERVICE_API_KEY = service_api_key

    # Provide a mock face cascade so the app thinks models are loaded
    mock_cascade = MagicMock()
    mock_cascade.detectMultiScale.return_value = []
    main.face_cascade = mock_cascade

    transport = ASGITransport(app=main.app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        yield client

    # Restore
    main.SERVICE_API_KEY = ""
    main.face_cascade = None


def create_test_image_bytes() -> bytes:
    """Create a minimal valid PNG image for upload tests."""
    from PIL import Image
    import io

    img = Image.new("RGB", (100, 100), color=(128, 128, 128))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()
