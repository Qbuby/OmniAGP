from __future__ import annotations

import enum
from typing import Optional

from pydantic import BaseModel, Field


class AssetType(str, enum.Enum):
    SPRITE_2D = "sprite_2d"
    TEXTURE = "texture"
    MODEL_3D = "model_3d"
    AUDIO = "audio"
    MUSIC = "music"


class JobStatus(str, enum.Enum):
    QUEUED = "queued"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


class GenerateRequest(BaseModel):
    prompt: str = Field(..., min_length=1, max_length=2000)
    output_format: Optional[str] = None
    params: Optional[dict] = None


class JobResponse(BaseModel):
    job_id: str
    status: JobStatus
    asset_type: AssetType


class JobStatusResponse(BaseModel):
    job_id: str
    status: JobStatus
    asset_type: AssetType
    progress: Optional[float] = None
    error: Optional[str] = None


class JobResultResponse(BaseModel):
    job_id: str
    status: JobStatus
    asset_type: AssetType
    file_path: Optional[str] = None
    file_url: Optional[str] = None
    error: Optional[str] = None


class HealthResponse(BaseModel):
    status: str
    version: str
    gpu_available: bool
