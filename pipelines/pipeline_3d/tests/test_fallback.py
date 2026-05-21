import pytest
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import trimesh

from src.models import AssetType, GenerateRequest, GenerationBackend
from src.pipeline import Pipeline3D


def _create_mock_glb(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    mesh = trimesh.creation.box(extents=[1.0, 1.0, 1.0])
    mesh.export(str(path), file_type="glb")


@pytest.fixture
def pipeline():
    with patch("src.pipeline.ReferenceImageGenerator") as ref_mock, \
         patch("src.pipeline.TripoSRGenerator") as tripo_mock, \
         patch("src.pipeline.Hunyuan3DGenerator") as hunyuan_mock:

        ref_instance = ref_mock.return_value
        ref_instance.generate = AsyncMock(side_effect=lambda prompt, output_path, **kw: _create_mock_glb(output_path) or output_path)
        ref_instance.close = AsyncMock()

        tripo_instance = tripo_mock.return_value
        tripo_instance.unload = MagicMock()

        hunyuan_instance = hunyuan_mock.return_value
        hunyuan_instance.close = AsyncMock()

        p = Pipeline3D()
        p.ref_image_gen = ref_instance
        p.triposr = tripo_instance
        p.hunyuan3d = hunyuan_instance
        yield p


@pytest.mark.anyio
async def test_triposr_fallback_to_hunyuan3d(pipeline, tmp_path):
    pipeline.triposr.generate = MagicMock(side_effect=RuntimeError("CUDA OOM"))
    pipeline.hunyuan3d.generate = AsyncMock(
        side_effect=lambda img, out: _create_mock_glb(out) or out
    )

    with patch("src.pipeline.settings") as mock_settings:
        mock_settings.output_dir = str(tmp_path / "output")
        mock_settings.temp_dir = str(tmp_path / "tmp")
        mock_settings.generation_timeout = 120
        mock_settings.default_backend = "triposr"
        mock_settings.hunyuan3d_api_url = "http://fake-api"
        mock_settings.max_vertices_prop = 5000
        mock_settings.max_file_size_mb = 50.0

        request = GenerateRequest(prompt="a sword", asset_type=AssetType.PROP)
        result = await pipeline.generate(request)

    assert result.status == "success"
    assert result.backend_used == "hunyuan3d"
    pipeline.hunyuan3d.generate.assert_called_once()


@pytest.mark.anyio
async def test_triposr_no_fallback_when_hunyuan_not_configured(pipeline, tmp_path):
    pipeline.triposr.generate = MagicMock(side_effect=RuntimeError("CUDA OOM"))

    with patch("src.pipeline.settings") as mock_settings:
        mock_settings.output_dir = str(tmp_path / "output")
        mock_settings.temp_dir = str(tmp_path / "tmp")
        mock_settings.generation_timeout = 120
        mock_settings.default_backend = "triposr"
        mock_settings.hunyuan3d_api_url = ""

        request = GenerateRequest(prompt="a sword", asset_type=AssetType.PROP)
        result = await pipeline.generate(request)

    assert result.status == "error"
    assert "CUDA OOM" in result.error


@pytest.mark.anyio
async def test_pipeline_timeout(pipeline, tmp_path):
    async def slow_generate(img, out):
        import asyncio
        await asyncio.sleep(10)

    pipeline.triposr.generate = MagicMock(side_effect=lambda img, out: _create_mock_glb(out))

    with patch("src.pipeline.settings") as mock_settings:
        mock_settings.output_dir = str(tmp_path / "output")
        mock_settings.temp_dir = str(tmp_path / "tmp")
        mock_settings.generation_timeout = 0.1
        mock_settings.default_backend = "triposr"
        mock_settings.hunyuan3d_api_url = ""

        ref_gen = pipeline.ref_image_gen
        ref_gen.generate = AsyncMock(side_effect=slow_generate)

        request = GenerateRequest(prompt="a dragon", asset_type=AssetType.CHARACTER)
        result = await pipeline.generate(request)

    assert result.status == "error"
    assert "timed out" in result.error
