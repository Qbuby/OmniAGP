from enum import Enum
from typing import Optional
from pydantic import BaseModel, Field


class AssetType(str, Enum):
    CHARACTER = "character"
    PROP = "prop"


class GenerationBackend(str, Enum):
    TRIPOSR = "triposr"
    HUNYUAN3D = "hunyuan3d"


class GenerateRequest(BaseModel):
    prompt: str = Field(..., min_length=1, max_length=1000)
    asset_type: AssetType = AssetType.PROP
    backend: Optional[GenerationBackend] = None
    negative_prompt: str = ""
    seed: Optional[int] = None


class MeshMetrics(BaseModel):
    vertex_count: int
    face_count: int
    is_manifold: bool
    file_size_mb: float
    has_valid_normals: bool
    has_uv: bool
    within_budget: bool


class GenerateResponse(BaseModel):
    task_id: str
    status: str
    glb_path: Optional[str] = None
    metrics: Optional[MeshMetrics] = None
    error: Optional[str] = None
    generation_time_seconds: Optional[float] = None
