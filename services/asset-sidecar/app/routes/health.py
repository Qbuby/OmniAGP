import shutil

from fastapi import APIRouter

from app.config import settings
from app.models import HealthResponse
from app.queue import task_queue

router = APIRouter(tags=["health"])

VERSION = "0.1.0"


def _gpu_available() -> bool:
    try:
        import torch
        return torch.cuda.is_available()
    except ImportError:
        return shutil.which("nvidia-smi") is not None


@router.get("/health", response_model=HealthResponse)
async def health():
    return HealthResponse(
        status="ok",
        version=VERSION,
        gpu_available=_gpu_available(),
        active_jobs=task_queue.active_count,
        max_concurrent_jobs=settings.max_concurrent_jobs,
    )
