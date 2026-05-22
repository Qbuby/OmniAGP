"""Tests for the 2D asset generation pipeline."""

import io
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.testclient import TestClient
from PIL import Image

from main import (
    AssetCategory,
    Generate2DRequest,
    StylePreset,
    _build_workflow,
    _remove_background,
    _save_asset,
    _slice_tileset,
    _validate_assets,
    app,
)

client = TestClient(app)


def _make_test_image(width=128, height=128, mode="RGBA") -> Image.Image:
    return Image.new(mode, (width, height), (255, 0, 0, 255))


class TestHealthEndpoint:
    @patch("main._check_comfyui", new_callable=AsyncMock)
    def test_health_ok(self, mock_check):
        mock_check.return_value = (True, 8192.0)
        resp = client.get("/health")
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "ok"
        assert data["comfyui_connected"] is True
        assert data["vram_free_mb"] == 8192.0

    @patch("main._check_comfyui", new_callable=AsyncMock)
    def test_health_degraded(self, mock_check):
        mock_check.return_value = (False, None)
        resp = client.get("/health")
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "degraded"
        assert data["comfyui_connected"] is False


class TestBuildWorkflow:
    def test_pixel_style_uses_lora(self):
        req = Generate2DRequest(prompt="a sword", style=StylePreset.pixel)
        workflow = _build_workflow(req)
        assert "10" in workflow
        assert workflow["10"]["class_type"] == "LoraLoader"
        assert "pixel-art-xl" in workflow["10"]["inputs"]["lora_name"]

    def test_anime_style_no_lora(self):
        req = Generate2DRequest(prompt="a cat", style=StylePreset.anime)
        workflow = _build_workflow(req)
        assert "2" in workflow
        text = workflow["2"]["inputs"]["text"]
        assert "anime" in text.lower() or "cel" in text.lower()

    def test_realistic_style(self):
        req = Generate2DRequest(prompt="a tree", style=StylePreset.realistic)
        workflow = _build_workflow(req)
        text = workflow["2"]["inputs"]["text"]
        assert "realistic" in text.lower() or "photorealistic" in text.lower()

    def test_custom_dimensions(self):
        req = Generate2DRequest(prompt="test", width=512, height=768)
        workflow = _build_workflow(req)
        assert workflow["4"]["inputs"]["width"] == 512
        assert workflow["4"]["inputs"]["height"] == 768

    def test_custom_steps_and_cfg(self):
        req = Generate2DRequest(prompt="test", steps=40, cfg_scale=12.0)
        workflow = _build_workflow(req)
        assert workflow["5"]["inputs"]["steps"] == 40
        assert workflow["5"]["inputs"]["cfg"] == 12.0

    def test_fixed_seed(self):
        req = Generate2DRequest(prompt="test", seed=42)
        workflow = _build_workflow(req)
        assert workflow["5"]["inputs"]["seed"] == 42

    def test_random_seed_when_negative(self):
        req = Generate2DRequest(prompt="test", seed=-1)
        workflow = _build_workflow(req)
        assert workflow["5"]["inputs"]["seed"] >= 0

    def test_reference_image_adds_ipadapter(self):
        req = Generate2DRequest(prompt="test", reference_image_b64="abc123")
        workflow = _build_workflow(req)
        assert "20" in workflow
        assert workflow["20"]["class_type"] == "IPAdapterApply"
        assert "21" in workflow
        assert workflow["21"]["inputs"]["image"] == "abc123"


class TestSliceTileset:
    def test_basic_slicing(self):
        img = _make_test_image(256, 256)
        tiles = _slice_tileset(img, 64)
        assert len(tiles) == 16
        for tile in tiles:
            assert tile.size == (64, 64)

    def test_non_divisible_size(self):
        img = _make_test_image(100, 100)
        tiles = _slice_tileset(img, 32)
        assert len(tiles) == 9

    def test_tile_larger_than_image(self):
        img = _make_test_image(32, 32)
        tiles = _slice_tileset(img, 64)
        assert len(tiles) == 0


class TestRemoveBackground:
    @patch("main.rembg_remove")
    def test_returns_rgba(self, mock_rembg):
        img = _make_test_image(64, 64, mode="RGB")
        out_bytes = io.BytesIO()
        img.save(out_bytes, format="PNG")
        mock_rembg.return_value = out_bytes.getvalue()

        result = _remove_background(img)
        assert result.mode == "RGBA"


class TestSaveAsset:
    def test_save_to_file(self, tmp_path, monkeypatch):
        monkeypatch.setattr("main.OUTPUT_DIR", tmp_path)
        img = _make_test_image(64, 64)
        out = _save_asset(img, "test-id", "file")
        assert out.file_path is not None
        assert out.width == 64
        assert out.height == 64
        assert out.has_alpha is True
        assert out.file_size_bytes > 0

    def test_save_as_base64(self, tmp_path, monkeypatch):
        monkeypatch.setattr("main.OUTPUT_DIR", tmp_path)
        img = _make_test_image(64, 64)
        out = _save_asset(img, "test-id", "base64")
        assert out.data_b64 is not None
        assert out.file_path is None
        assert out.width == 64
        assert out.height == 64


class TestValidateAssets:
    def test_no_errors_when_valid(self):
        from main import AssetOutput
        assets = [AssetOutput(file_path="/tmp/test.png", width=128, height=128, has_alpha=True, file_size_bytes=1000)]
        errors = _validate_assets(assets, expect_alpha=True)
        assert errors == []

    def test_missing_alpha(self):
        from main import AssetOutput
        assets = [AssetOutput(file_path="/tmp/test.png", width=128, height=128, has_alpha=False, file_size_bytes=1000)]
        errors = _validate_assets(assets, expect_alpha=True)
        assert len(errors) == 1
        assert "RGBA" in errors[0]

    def test_too_small(self):
        from main import AssetOutput
        assets = [AssetOutput(file_path="/tmp/test.png", width=8, height=8, has_alpha=True, file_size_bytes=100)]
        errors = _validate_assets(assets, expect_alpha=True)
        assert len(errors) == 1
        assert "too small" in errors[0]


class TestGenerateEndpoint:
    @patch("main._run_comfyui_workflow", new_callable=AsyncMock)
    def test_generate_success(self, mock_workflow, tmp_path, monkeypatch):
        monkeypatch.setattr("main.OUTPUT_DIR", tmp_path)
        mock_workflow.return_value = _make_test_image(512, 512)

        with patch("main._remove_background", side_effect=lambda img: img):
            resp = client.post("/generate", json={
                "prompt": "a pixel art sword",
                "style": "pixel",
                "category": "sprite",
                "remove_background": True,
            })

        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "success"
        assert len(data["assets"]) == 1
        assert data["assets"][0]["has_alpha"] is True
        assert data["generation_time_ms"] >= 0

    @patch("main._run_comfyui_workflow", new_callable=AsyncMock)
    def test_generate_tileset(self, mock_workflow, tmp_path, monkeypatch):
        monkeypatch.setattr("main.OUTPUT_DIR", tmp_path)
        mock_workflow.return_value = _make_test_image(256, 256)

        with patch("main._remove_background", side_effect=lambda img: img):
            resp = client.post("/generate", json={
                "prompt": "grass tileset",
                "style": "pixel",
                "category": "tileset",
                "tile_size": 64,
                "remove_background": True,
            })

        assert resp.status_code == 200
        data = resp.json()
        assert len(data["assets"]) == 16

    @patch("main._run_comfyui_workflow", new_callable=AsyncMock)
    def test_generate_comfyui_failure(self, mock_workflow):
        mock_workflow.side_effect = RuntimeError("ComfyUI down")
        resp = client.post("/generate", json={"prompt": "test"})
        assert resp.status_code == 502

    def test_generate_invalid_prompt(self):
        resp = client.post("/generate", json={"prompt": ""})
        assert resp.status_code == 422

    @patch("main._run_comfyui_workflow", new_callable=AsyncMock)
    def test_generate_base64_output(self, mock_workflow, tmp_path, monkeypatch):
        monkeypatch.setattr("main.OUTPUT_DIR", tmp_path)
        mock_workflow.return_value = _make_test_image(128, 128)

        with patch("main._remove_background", side_effect=lambda img: img):
            resp = client.post("/generate", json={
                "prompt": "an icon",
                "output_format": "base64",
                "remove_background": True,
            })

        assert resp.status_code == 200
        data = resp.json()
        assert data["assets"][0]["data_b64"] is not None
        assert data["assets"][0]["file_path"] is None
