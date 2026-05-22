from __future__ import annotations

import os
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from .director import AssetDirector, AssetDirectorConfig, PipelineEndpoints


class DesignDocRequest(BaseModel):
    design_doc: dict


class SingleAudioRequest(BaseModel):
    prompt: str
    audio_type: str
    duration_sec: Optional[float] = None
    output_dir: Optional[str] = None


class DirectorStatusResponse(BaseModel):
    total_tasks: int
    succeeded: int
    failed: int
    failures: list[dict]
    asset_registry: dict


_director: Optional[AssetDirector] = None


def _build_config() -> AssetDirectorConfig:
    endpoints = PipelineEndpoints(
        audio_url=os.environ.get("AUDIO_PIPELINE_URL", "http://localhost:8090"),
        image_url=os.environ.get("IMAGE_PIPELINE_URL", "http://localhost:8091"),
    )
    return AssetDirectorConfig(
        endpoints=endpoints,
        max_retries=int(os.environ.get("MAX_RETRIES", "2")),
        max_parallel_audio=int(os.environ.get("MAX_PARALLEL_AUDIO", "1")),
        max_parallel_image=int(os.environ.get("MAX_PARALLEL_IMAGE", "2")),
        timeout_sec=float(os.environ.get("TIMEOUT_SEC", "300")),
    )


@asynccontextmanager
async def lifespan(app: FastAPI):
    global _director
    _director = AssetDirector(_build_config())
    yield
    if _director:
        await _director.close()
    _director = None


app = FastAPI(title="OmniAGP AssetDirector", version="0.1.0", lifespan=lifespan)


@app.get("/health")
async def health():
    return {"status": "ok", "service": "asset-director"}


@app.post("/execute", response_model=DirectorStatusResponse)
async def execute_design_doc(req: DesignDocRequest):
    result = await _director.run_from_design_doc(req.design_doc)
    return DirectorStatusResponse(**result)


@app.post("/generate/audio")
async def generate_single_audio(req: SingleAudioRequest):
    if req.audio_type not in ("bgm", "sfx"):
        raise HTTPException(status_code=400, detail="audio_type must be 'bgm' or 'sfx'")

    from .director import AssetCategory, AssetTask

    task = AssetTask(
        id="single_audio_001",
        category=AssetCategory.BGM if req.audio_type == "bgm" else AssetCategory.SFX,
        prompt=req.prompt,
        duration_sec=req.duration_sec,
        output_path=req.output_dir,
    )
    result = await _director._execute_task(task)
    if not result.success:
        raise HTTPException(status_code=500, detail=result.error or "audio generation failed")
    return result.metadata
