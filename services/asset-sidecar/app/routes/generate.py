from fastapi import APIRouter

from app.models import AssetType, GenerateRequest, JobResponse, JobStatus
from app.queue import task_queue

router = APIRouter(prefix="/generate", tags=["generate"])


@router.post("/2d", response_model=JobResponse)
async def generate_2d(req: GenerateRequest):
    job = await task_queue.submit(AssetType.SPRITE_2D, req.prompt, req.params)
    return JobResponse(job_id=job.id, status=job.status, asset_type=job.asset_type)


@router.post("/3d", response_model=JobResponse)
async def generate_3d(req: GenerateRequest):
    job = await task_queue.submit(AssetType.MODEL_3D, req.prompt, req.params)
    return JobResponse(job_id=job.id, status=job.status, asset_type=job.asset_type)


@router.post("/audio", response_model=JobResponse)
async def generate_audio(req: GenerateRequest):
    job = await task_queue.submit(AssetType.AUDIO, req.prompt, req.params)
    return JobResponse(job_id=job.id, status=job.status, asset_type=job.asset_type)
