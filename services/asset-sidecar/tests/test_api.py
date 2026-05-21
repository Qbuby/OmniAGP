import asyncio

import pytest
from httpx import ASGITransport, AsyncClient

from app.main import app


@pytest.fixture
def anyio_backend():
    return "asyncio"


@pytest.fixture
async def client():
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c


@pytest.mark.anyio
async def test_health(client: AsyncClient):
    resp = await client.get("/health")
    assert resp.status_code == 200
    data = resp.json()
    assert data["status"] == "ok"
    assert data["version"] == "0.1.0"


@pytest.mark.anyio
async def test_generate_2d_and_poll(client: AsyncClient):
    resp = await client.post("/generate/2d", json={"prompt": "a red dragon sprite"})
    assert resp.status_code == 200
    data = resp.json()
    assert data["status"] == "queued"
    assert data["asset_type"] == "sprite_2d"
    job_id = data["job_id"]

    await asyncio.sleep(3)

    resp = await client.get(f"/status/{job_id}")
    assert resp.status_code == 200
    status_data = resp.json()
    assert status_data["status"] == "completed"

    resp = await client.get(f"/result/{job_id}")
    assert resp.status_code == 200
    result_data = resp.json()
    assert result_data["file_path"] is not None


@pytest.mark.anyio
async def test_generate_3d(client: AsyncClient):
    resp = await client.post("/generate/3d", json={"prompt": "low-poly tree"})
    assert resp.status_code == 200
    data = resp.json()
    assert data["asset_type"] == "model_3d"


@pytest.mark.anyio
async def test_generate_audio(client: AsyncClient):
    resp = await client.post("/generate/audio", json={"prompt": "sword clash sound effect"})
    assert resp.status_code == 200
    data = resp.json()
    assert data["asset_type"] == "audio"


@pytest.mark.anyio
async def test_status_not_found(client: AsyncClient):
    resp = await client.get("/status/nonexistent-id")
    assert resp.status_code == 404


@pytest.mark.anyio
async def test_result_not_found(client: AsyncClient):
    resp = await client.get("/result/nonexistent-id")
    assert resp.status_code == 404


@pytest.mark.anyio
async def test_generate_invalid_prompt(client: AsyncClient):
    resp = await client.post("/generate/2d", json={"prompt": ""})
    assert resp.status_code == 422
