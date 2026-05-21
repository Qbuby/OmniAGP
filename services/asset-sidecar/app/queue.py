from __future__ import annotations

import asyncio
import logging
import uuid
from collections.abc import Callable, Coroutine
from typing import Any

import httpx

from app.models import AssetType, JobStatus

logger = logging.getLogger(__name__)


class Job:
    __slots__ = ("id", "asset_type", "prompt", "params", "status", "progress", "result_path", "error")

    def __init__(self, asset_type: AssetType, prompt: str, params: dict | None = None):
        self.id = str(uuid.uuid4())
        self.asset_type = asset_type
        self.prompt = prompt
        self.params = params or {}
        self.status = JobStatus.QUEUED
        self.progress: float | None = None
        self.result_path: str | None = None
        self.error: str | None = None


WorkerFn = Callable[[Job], Coroutine[Any, Any, None]]


class TaskQueue:
    def __init__(self, max_concurrent: int = 4):
        self._jobs: dict[str, Job] = {}
        self._semaphore = asyncio.Semaphore(max_concurrent)
        self._worker: WorkerFn | None = None
        self._orchestrator_url: str | None = None

    def set_worker(self, worker: WorkerFn) -> None:
        self._worker = worker

    def set_orchestrator_url(self, url: str) -> None:
        self._orchestrator_url = url

    async def submit(self, asset_type: AssetType, prompt: str, params: dict | None = None) -> Job:
        job = Job(asset_type=asset_type, prompt=prompt, params=params)
        self._jobs[job.id] = job
        asyncio.create_task(self._run(job))
        return job

    def get(self, job_id: str) -> Job | None:
        return self._jobs.get(job_id)

    @property
    def active_count(self) -> int:
        return sum(1 for j in self._jobs.values() if j.status == JobStatus.RUNNING)

    async def _run(self, job: Job) -> None:
        async with self._semaphore:
            job.status = JobStatus.RUNNING
            try:
                if self._worker:
                    await self._worker(job)
                else:
                    await self._mock_worker(job)
                if job.status == JobStatus.RUNNING:
                    job.status = JobStatus.COMPLETED
            except Exception as exc:
                job.status = JobStatus.FAILED
                job.error = str(exc)
            await self._notify_orchestrator(job)

    async def _notify_orchestrator(self, job: Job) -> None:
        if not self._orchestrator_url:
            return
        payload = {
            "job_id": job.id,
            "asset_type": job.asset_type.value,
            "status": job.status.value,
            "result_path": job.result_path,
            "error": job.error,
        }
        try:
            async with httpx.AsyncClient(timeout=10.0) as client:
                await client.post(
                    f"{self._orchestrator_url}/api/v1/sidecar/callback",
                    json=payload,
                )
        except Exception as exc:
            logger.warning("Failed to notify orchestrator: %s", exc)

    @staticmethod
    async def _mock_worker(job: Job) -> None:
        await asyncio.sleep(2)
        job.progress = 1.0
        job.result_path = f"/tmp/asset-sidecar/output/{job.id}.png"


task_queue = TaskQueue()
