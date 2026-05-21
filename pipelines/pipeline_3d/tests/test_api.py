import pytest
from pathlib import Path
from unittest.mock import AsyncMock, patch, MagicMock

import trimesh
from httpx import AsyncClient, ASGITransport

from src.api.routes import app
from src.models import AssetType, GenerateResponse, MeshMetrics


@pytest.fixture
def mock_glb(tmp_path: Path) -> Path:
    mesh = trimesh.creation.box(extents=[1.0, 1.0, 1.0])
    glb_path = tmp_path / "test" / "test.glb"
    glb_path.parent.mkdir(parents=True)
    mesh.export(str(glb_path), file_type="glb")
    return glb_path


@pytest.fixture
def mock_pipeline(mock_glb):
    metrics = MeshMetrics(
        vertex_count=8,
        face_count=12,
        is_manifold=True,
        file_size_mb=0.01,
        has_valid_normals=True,
        has_uv=True,
        within_budget=True,
    )
    response = GenerateResponse(
        task_id="abc123",
        status="success",
        glb_path=str(mock_glb),
        backend_used="triposr",
        metrics=metrics,
        generation_time_seconds=5.0,
    )
    with patch("src.api.routes.pipeline") as mock:
        mock.generate = AsyncMock(return_value=response)
        yield mock


@pytest.mark.anyio
async def test_generate_3d_success(mock_pipeline):
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.post("/generate/3d", json={
            "prompt": "a medieval sword",
            "asset_type": "prop",
        })

    assert resp.status_code == 200
    data = resp.json()
    assert data["status"] == "success"
    assert data["task_id"] == "abc123"
    assert data["backend_used"] == "triposr"
    assert data["metrics"]["within_budget"] is True


@pytest.mark.anyio
async def test_generate_3d_error(mock_pipeline):
    mock_pipeline.generate = AsyncMock(
        return_value=GenerateResponse(
            task_id="err123",
            status="error",
            error="TripoSR OOM",
            generation_time_seconds=2.0,
        )
    )
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.post("/generate/3d", json={
            "prompt": "a dragon character",
            "asset_type": "character",
        })

    assert resp.status_code == 500
    assert "TripoSR OOM" in resp.json()["detail"]


@pytest.mark.anyio
async def test_generate_3d_validation_error():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.post("/generate/3d", json={
            "prompt": "",
        })

    assert resp.status_code == 422


@pytest.mark.anyio
async def test_health_endpoint():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.get("/health")

    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


@pytest.mark.anyio
async def test_download_asset_not_found():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.get("/assets/nonexistent123")

    assert resp.status_code == 404


@pytest.mark.anyio
async def test_download_asset_invalid_id():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        resp = await client.get("/assets/../../etc/passwd")

    assert resp.status_code == 400
