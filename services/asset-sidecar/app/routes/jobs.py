from fastapi import APIRouter, HTTPException

from app.models import JobResultResponse, JobStatusResponse
from app.queue import task_queue

router = APIRouter(tags=["jobs"])


@router.get("/status/{job_id}", response_model=JobStatusResponse)
async def get_status(job_id: str):
    job = task_queue.get(job_id)
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    return JobStatusResponse(
        job_id=job.id,
        status=job.status,
        asset_type=job.asset_type,
        progress=job.progress,
        error=job.error,
    )


@router.get("/result/{job_id}", response_model=JobResultResponse)
async def get_result(job_id: str):
    job = task_queue.get(job_id)
    if not job:
        raise HTTPException(status_code=404, detail="Job not found")
    return JobResultResponse(
        job_id=job.id,
        status=job.status,
        asset_type=job.asset_type,
        file_path=job.result_path,
        error=job.error,
    )
