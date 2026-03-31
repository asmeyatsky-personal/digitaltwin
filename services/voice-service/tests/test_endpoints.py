"""Tests for voice service endpoints: /voices, /tts, /voice/stt."""

import pytest
from unittest.mock import patch, MagicMock, AsyncMock

from conftest import create_test_audio_bytes


class TestListVoices:
    async def test_voices_returns_default_list_without_elevenlabs(
        self, test_client, service_api_headers
    ):
        """/voices returns default voices when ELEVENLABS_API_KEY is not set."""
        import main

        original = main.ELEVENLABS_API_KEY
        main.ELEVENLABS_API_KEY = ""
        try:
            resp = await test_client.get(
                "/voices", headers=service_api_headers
            )
            assert resp.status_code == 200
            data = resp.json()
            assert isinstance(data, list)
            assert len(data) == 5  # 5 default voices
            # Each voice should have the expected shape
            for voice in data:
                assert "voice_id" in voice
                assert "name" in voice
                assert "category" in voice
        finally:
            main.ELEVENLABS_API_KEY = original

    async def test_voices_calls_elevenlabs_when_configured(
        self, test_client, service_api_headers
    ):
        """/voices calls ElevenLabs API when the key is configured."""
        import main

        original = main.ELEVENLABS_API_KEY
        main.ELEVENLABS_API_KEY = "fake-elevenlabs-key"

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.raise_for_status = MagicMock()
        mock_response.json.return_value = {
            "voices": [
                {
                    "voice_id": "v1",
                    "name": "TestVoice",
                    "category": "premade",
                    "description": "A test voice",
                    "preview_url": None,
                    "labels": {},
                }
            ]
        }

        with patch("httpx.AsyncClient.get", new_callable=AsyncMock) as mock_get:
            mock_get.return_value = mock_response
            resp = await test_client.get(
                "/voices", headers=service_api_headers
            )

        main.ELEVENLABS_API_KEY = original

        assert resp.status_code == 200
        data = resp.json()
        assert len(data) == 1
        assert data[0]["voice_id"] == "v1"
        assert data[0]["name"] == "TestVoice"


class TestTTS:
    async def test_tts_returns_503_without_elevenlabs_key(
        self, test_client, service_api_headers
    ):
        """/tts returns 503 when ElevenLabs is not configured."""
        import main

        original = main.ELEVENLABS_API_KEY
        main.ELEVENLABS_API_KEY = ""
        try:
            resp = await test_client.post(
                "/tts",
                json={"text": "Hello world"},
                headers=service_api_headers,
            )
            assert resp.status_code == 503
        finally:
            main.ELEVENLABS_API_KEY = original

    async def test_tts_success_with_mock(
        self, test_client, service_api_headers
    ):
        """/tts returns audio stream when ElevenLabs is available."""
        import main

        original = main.ELEVENLABS_API_KEY
        main.ELEVENLABS_API_KEY = "fake-elevenlabs-key"

        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_response.raise_for_status = MagicMock()
        mock_response.content = b"fake-audio-content"

        with patch("httpx.AsyncClient.post", new_callable=AsyncMock) as mock_post:
            mock_post.return_value = mock_response
            resp = await test_client.post(
                "/tts",
                json={"text": "Hello world"},
                headers=service_api_headers,
            )

        main.ELEVENLABS_API_KEY = original

        assert resp.status_code == 200
        assert resp.headers["content-type"] == "audio/mpeg"


class TestSTT:
    async def test_stt_returns_503_without_openai_key(
        self, test_client, service_api_headers
    ):
        """/voice/stt returns 503 when OPENAI_API_KEY is not set."""
        audio_bytes = create_test_audio_bytes()
        resp = await test_client.post(
            "/voice/stt",
            files={"file": ("test.wav", audio_bytes, "audio/wav")},
            headers=service_api_headers,
        )
        assert resp.status_code == 503

    async def test_stt_success_with_mock(
        self, test_client, service_api_headers, monkeypatch
    ):
        """/voice/stt transcribes audio using mocked OpenAI Whisper."""
        monkeypatch.setenv("OPENAI_API_KEY", "fake-openai-key")

        mock_transcript = MagicMock()
        mock_transcript.text = "Hello, this is a test."
        mock_transcript.language = "en"

        mock_client_instance = MagicMock()
        mock_client_instance.audio.transcriptions.create.return_value = (
            mock_transcript
        )

        with patch("main.openai_module.OpenAI", return_value=mock_client_instance):
            audio_bytes = create_test_audio_bytes()
            resp = await test_client.post(
                "/voice/stt",
                files={"file": ("test.wav", audio_bytes, "audio/wav")},
                headers=service_api_headers,
            )

        assert resp.status_code == 200
        data = resp.json()
        assert data["text"] == "Hello, this is a test."
        assert data["language"] == "en"
