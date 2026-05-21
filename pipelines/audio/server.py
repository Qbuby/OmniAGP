from __future__ import annotations

import asyncio
from contextlib import asynccontextmanager
from enum import Enum
from typing import Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from .config import AudioPipelineConfig
from .generator import AudioGenerator, AudioType


class AudioRequest(BaseModel):
    prompt: str
    audio_type: str  # "bgm" or "sfx"
    duration_sec: Optional[float] = None
    output_dir: Optional[str] = None


class AudioResponse(BaseModel):
    file_path: str
    audio_type: str
    duration_sec: float
    sample_rate: int
    loop_point_samples: Optional[int] = None
    validation: dict


_generator: Optional[AudioGenerator] = None
_semaphore: Optional[asyncio.Semaphore] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global _generator, _semaphore
    config = AudioPipelineConfig()
    _generator = AudioGenerator(config)
    _semaphore = asyncio.Semaphore(config.max_concurrent_jobs)
    yield
    _generator = None


app = FastAPI(title="OmniAGP Audio Pipeline", version="0.1.0", lifespan=lifespan)


@app.get("/health")
async def health():
    return {"status": "ok", "service": "audio-pipeline"}


@app.post("/generate", response_model=AudioResponse)
async def generate_audio(req: AudioRequest):
    if req.audio_type not in ("bgm", "sfx"):
        raise HTTPException(status_code=400, detail="audio_type must be 'bgm' or 'sfx'")

    audio_type = AudioType(req.audio_type)

    async with _semaphore:
        loop = asyncio.get_event_loop()
        result = await loop.run_in_executor(
            None,
            _generator.generate,
            req.prompt,
            audio_type,
            req.duration_sec,
            req.output_dir,
        )

    return AudioResponse(**result)


@app.get("/models")
async def list_models():
    return {
        "musicgen": _generator.config.musicgen_model,
        "audiogen": _generator.config.audiogen_model,
        "device": _generator.config.device,
    }
