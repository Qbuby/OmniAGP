from contextlib import asynccontextmanager

from fastapi import FastAPI

from app.config import settings
from app.queue import task_queue
from app.routes import generate, health, jobs


@asynccontextmanager
async def lifespan(app: FastAPI):
    task_queue._semaphore = __import__("asyncio").Semaphore(settings.max_concurrent_jobs)
    yield


app = FastAPI(
    title="OmniAGP Asset Sidecar",
    description="Python sidecar service for AI asset generation (2D/3D/Audio)",
    version="0.1.0",
    lifespan=lifespan,
)

app.include_router(health.router)
app.include_router(generate.router)
app.include_router(jobs.router)
