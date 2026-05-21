import logging
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from contextlib import asynccontextmanager

from ..models import GenerateRequest, GenerateResponse
from ..pipeline import Pipeline3D
from ..config import settings

logger = logging.getLogger(__name__)

pipeline: Pipeline3D = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global pipeline
    pipeline = Pipeline3D()
    logger.info("3D Pipeline initialized")
    yield
    await pipeline.close()
    logger.info("3D Pipeline shut down")


app = FastAPI(
    title="OmniAGP 3D Asset Pipeline",
    version="0.1.0",
    lifespan=lifespan,
)


@app.post("/generate/3d", response_model=GenerateResponse)
async def generate_3d(request: GenerateRequest) -> GenerateResponse:
    logger.info(f"Received 3D generation request: {request.prompt[:80]}...")
    result = await pipeline.generate(request)
    if result.status == "error":
        raise HTTPException(status_code=500, detail=result.error)
    return result


@app.get("/assets/{task_id}")
async def download_asset(task_id: str):
    if not task_id.isalnum():
        raise HTTPException(status_code=400, detail="Invalid task_id")
    glb_path = Path(settings.output_dir) / task_id / f"{task_id}.glb"
    if not glb_path.exists():
        raise HTTPException(status_code=404, detail="Asset not found")
    return FileResponse(
        path=str(glb_path),
        media_type="model/gltf-binary",
        filename=f"{task_id}.glb",
    )


@app.get("/health")
async def health():
    return {"status": "ok", "service": "pipeline-3d"}
