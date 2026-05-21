import shutil

from fastapi import APIRouter

from app.models import HealthResponse

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
    )
