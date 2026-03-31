"""Tests for the /avatar/generate endpoint with mocked mediapipe."""

import os
import pytest
from unittest.mock import patch, MagicMock
import numpy as np

from conftest import create_test_image_bytes


def _build_mock_mediapipe():
    """Build a mock mediapipe module with face mesh and face detection."""
    # Build mock face landmarks
    mock_landmark = MagicMock()
    mock_landmark.x = 0.5
    mock_landmark.y = 0.5
    mock_landmark.z = 0.01

    mock_face_landmarks = MagicMock()
    mock_face_landmarks.landmark = [mock_landmark] * 478  # MediaPipe has 478 landmarks

    mock_mesh_results = MagicMock()
    mock_mesh_results.multi_face_landmarks = [mock_face_landmarks]

    mock_face_mesh_instance = MagicMock()
    mock_face_mesh_instance.process.return_value = mock_mesh_results

    # Detection
    mock_detection = MagicMock()
    mock_detection_results = MagicMock()
    mock_detection_results.detections = [mock_detection]

    mock_face_detector_instance = MagicMock()
    mock_face_detector_instance.process.return_value = mock_detection_results

    return mock_face_mesh_instance, mock_face_detector_instance


class TestAvatarGeneration:
    async def test_generate_avatar_success(
        self, test_client, service_api_headers, tmp_path
    ):
        """Avatar generation returns expected response when mediapipe works."""
        import main

        mock_mesh, mock_detector = _build_mock_mediapipe()

        # Patch get_mediapipe to return our mocks
        with patch.object(main, "get_mediapipe", return_value=(mock_mesh, mock_detector)):
            # Also patch file writing helpers to use tmp_path
            with patch.object(main, "AVATAR_STORAGE_PATH", str(tmp_path)):
                image_bytes = create_test_image_bytes()
                resp = await test_client.post(
                    "/avatar/generate",
                    files={"file": ("test.png", image_bytes, "image/png")},
                    data={"user_id": "user-1", "avatar_style": "realistic"},
                    headers=service_api_headers,
                )

        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "completed"
        assert data["user_id"] == "user-1"
        assert "avatar_id" in data
        assert "avatar_url" in data
        assert "thumbnail_url" in data
        assert data["face_landmarks_count"] == 478
        assert data["mesh_vertices"] > 0

    async def test_generate_avatar_no_face_detected(
        self, test_client, service_api_headers
    ):
        """When no face is detected, the endpoint returns 400."""
        import main

        mock_mesh = MagicMock()
        mock_detector = MagicMock()
        mock_detector.process.return_value = MagicMock(detections=None)

        with patch.object(main, "get_mediapipe", return_value=(mock_mesh, mock_detector)):
            image_bytes = create_test_image_bytes()
            resp = await test_client.post(
                "/avatar/generate",
                files={"file": ("test.png", image_bytes, "image/png")},
                data={"user_id": "user-1"},
                headers=service_api_headers,
            )

        assert resp.status_code == 400
        assert "No face detected" in resp.json()["detail"]

    async def test_generate_avatar_no_landmarks(
        self, test_client, service_api_headers
    ):
        """When face is detected but landmarks fail, endpoint returns 400."""
        import main

        mock_detection = MagicMock()
        mock_detection_results = MagicMock(detections=[mock_detection])
        mock_detector = MagicMock()
        mock_detector.process.return_value = mock_detection_results

        mock_mesh = MagicMock()
        mock_mesh.process.return_value = MagicMock(multi_face_landmarks=None)

        with patch.object(main, "get_mediapipe", return_value=(mock_mesh, mock_detector)):
            image_bytes = create_test_image_bytes()
            resp = await test_client.post(
                "/avatar/generate",
                files={"file": ("test.png", image_bytes, "image/png")},
                data={"user_id": "user-1"},
                headers=service_api_headers,
            )

        assert resp.status_code == 400
        assert "landmarks" in resp.json()["detail"].lower()


class TestAvatarStatus:
    async def test_status_not_found(self, test_client, service_api_headers):
        """Querying a nonexistent avatar ID returns 404."""
        resp = await test_client.get(
            "/avatar/nonexistent-id/status",
            headers=service_api_headers,
        )
        assert resp.status_code == 404
