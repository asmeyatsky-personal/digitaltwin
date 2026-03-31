"""Tests for the /analyze/facial-expression endpoint."""

import pytest
from unittest.mock import patch, MagicMock
import numpy as np

from conftest import create_test_image_bytes


class TestFacialExpressionAnalysis:
    async def test_analyze_returns_emotion_data(
        self, test_client, service_api_headers
    ):
        """The /analyze/facial-expression endpoint returns structured emotion data."""
        mock_emotions = {
            "angry": 0.01,
            "disgust": 0.01,
            "fear": 0.02,
            "happy": 0.85,
            "sad": 0.03,
            "surprise": 0.05,
            "neutral": 0.03,
        }

        with patch("main.detect_emotions_deepface") as mock_detect:
            mock_detect.return_value = (mock_emotions, "happy")

            image_bytes = create_test_image_bytes()
            resp = await test_client.post(
                "/analyze/facial-expression",
                files={"file": ("test.png", image_bytes, "image/png")},
                headers=service_api_headers,
            )

        assert resp.status_code == 200
        data = resp.json()
        assert "face_detected" in data
        assert data["dominant_emotion"] == "happy"
        assert data["confidence"] == pytest.approx(0.85)
        assert "emotions" in data
        assert "happy" in data["emotions"]

    async def test_analyze_with_no_face_detected(
        self, test_client, service_api_headers
    ):
        """When no face is detected, face_detected should be False
        but emotion analysis should still run on the full image."""
        mock_emotions = {
            "angry": 0.0,
            "disgust": 0.0,
            "fear": 0.0,
            "happy": 0.0,
            "sad": 0.0,
            "surprise": 0.0,
            "neutral": 1.0,
        }

        with patch("main.detect_emotions_deepface") as mock_detect:
            mock_detect.return_value = (mock_emotions, "neutral")

            image_bytes = create_test_image_bytes()
            resp = await test_client.post(
                "/analyze/facial-expression",
                files={"file": ("test.png", image_bytes, "image/png")},
                headers=service_api_headers,
            )

        assert resp.status_code == 200
        data = resp.json()
        assert data["face_detected"] is False
        assert data["dominant_emotion"] == "neutral"

    async def test_emotion_endpoint_returns_comprehensive_analysis(
        self, test_client, service_api_headers
    ):
        """/analyze/emotion returns sentiment, arousal, and valence."""
        mock_emotions = {
            "angry": 0.05,
            "disgust": 0.02,
            "fear": 0.03,
            "happy": 0.70,
            "sad": 0.05,
            "surprise": 0.10,
            "neutral": 0.05,
        }

        with patch("main.detect_emotions_deepface") as mock_detect:
            mock_detect.return_value = (mock_emotions, "happy")

            image_bytes = create_test_image_bytes()
            resp = await test_client.post(
                "/analyze/emotion",
                files={"file": ("test.png", image_bytes, "image/png")},
                headers=service_api_headers,
            )

        assert resp.status_code == 200
        data = resp.json()
        assert data["primary_emotion"] == "happy"
        assert "emotion_scores" in data
        assert "sentiment" in data
        assert data["sentiment"] in ("Positive", "Negative", "Neutral")
        assert "sentiment_score" in data
        assert "arousal_level" in data
        assert "valence_level" in data
        assert 0.0 <= data["intensity"] <= 1.0
