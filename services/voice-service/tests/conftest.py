"""Shared fixtures for voice-service tests."""

import os
import sys
from unittest.mock import MagicMock, AsyncMock, patch

import pytest
from httpx import ASGITransport, AsyncClient

# Ensure the service root is importable
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))


@pytest.fixture(autouse=True)
def _clean_env(monkeypatch):
    """Ensure no real API keys leak into tests."""
    for key in ("SERVICE_API_KEY", "ELEVENLABS_API_KEY", "OPENAI_API_KEY"):
        monkeypatch.delenv(key, raising=False)


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
    """AsyncClient wired to the FastAPI app."""
    import main

    main.SERVICE_API_KEY = service_api_key

    transport = ASGITransport(app=main.app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        yield client

    # Restore
    main.SERVICE_API_KEY = ""


def create_test_audio_bytes() -> bytes:
    """Create minimal WAV audio bytes for upload tests."""
    import struct

    # Minimal WAV header + 1 second of silence at 8kHz mono 16-bit
    sample_rate = 8000
    num_samples = sample_rate  # 1 second
    bits_per_sample = 16
    num_channels = 1
    byte_rate = sample_rate * num_channels * bits_per_sample // 8
    block_align = num_channels * bits_per_sample // 8
    data_size = num_samples * block_align

    header = struct.pack(
        "<4sI4s4sIHHIIHH4sI",
        b"RIFF",
        36 + data_size,
        b"WAVE",
        b"fmt ",
        16,  # chunk size
        1,  # PCM format
        num_channels,
        sample_rate,
        byte_rate,
        block_align,
        bits_per_sample,
        b"data",
        data_size,
    )
    # Silence
    audio_data = b"\x00\x00" * num_samples
    return header + audio_data
