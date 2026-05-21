from __future__ import annotations

from contextlib import asynccontextmanager
from typing import Optional

from fastapi import FastAPI
from pydantic import BaseModel

from .director import AssetDirector, AssetDirectorConfig


class DesignDocRequest(BaseModel):
    design_doc: dict


class DirectorStatusResponse(BaseModel):
    total_tasks: int
    succeeded: int
    failed: int
    failures: list[dict]
    asset_registry: dict


_director: Optional[AssetDirector] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global _director
    _director = AssetDirector(AssetDirectorConfig())
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
