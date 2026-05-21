import logging

from fastapi import FastAPI, HTTPException
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


@app.get("/health")
async def health():
    return {"status": "ok", "service": "pipeline-3d"}
