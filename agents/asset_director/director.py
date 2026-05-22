from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

import httpx
from pydantic import BaseModel


class AssetCategory(str, Enum):
    SPRITE_2D = "sprite_2d"
    TEXTURE = "texture"
    BGM = "bgm"
    SFX = "sfx"


class AssetTask(BaseModel):
    id: str
    category: AssetCategory
    prompt: str
    output_path: Optional[str] = None
    duration_sec: Optional[float] = None
    priority: int = 0


class AssetTaskResult(BaseModel):
    task: AssetTask
    success: bool
    file_path: Optional[str] = None
    error: Optional[str] = None
    metadata: dict = {}


class PipelineEndpoints(BaseModel):
    audio_url: str = "http://localhost:8090"
    image_url: str = "http://localhost:8091"


class AssetDirectorConfig(BaseModel):
    endpoints: PipelineEndpoints = PipelineEndpoints()
    max_retries: int = 2
    retry_delay_sec: float = 5.0
    max_parallel_audio: int = 1
    max_parallel_image: int = 2
    timeout_sec: float = 300.0


class AssetDirector:
    def __init__(self, config: AssetDirectorConfig | None = None):
        self.config = config or AssetDirectorConfig()
        self._client = httpx.AsyncClient(timeout=self.config.timeout_sec)

    async def close(self):
        await self._client.aclose()

    def parse_design_doc(self, design_doc: dict) -> list[AssetTask]:
        tasks = []
        task_id = 0

        scenes = design_doc.get("scenes", [])
        for scene in scenes:
            scene_name = scene.get("name", "unknown")

            if bgm := scene.get("bgm"):
                task_id += 1
                tasks.append(AssetTask(
                    id=f"audio_{task_id:03d}",
                    category=AssetCategory.BGM,
                    prompt=bgm if isinstance(bgm, str) else bgm.get("description", ""),
                    duration_sec=bgm.get("duration_sec", 30.0) if isinstance(bgm, dict) else 30.0,
                    output_path=f"assets/audio/bgm/{scene_name}",
                ))

            for sfx in scene.get("sfx", []):
                task_id += 1
                tasks.append(AssetTask(
                    id=f"audio_{task_id:03d}",
                    category=AssetCategory.SFX,
                    prompt=sfx if isinstance(sfx, str) else sfx.get("description", ""),
                    duration_sec=sfx.get("duration_sec", 5.0) if isinstance(sfx, dict) else 5.0,
                    output_path=f"assets/audio/sfx/{scene_name}",
                ))

            for sprite in scene.get("sprites", []):
                task_id += 1
                tasks.append(AssetTask(
                    id=f"sprite_{task_id:03d}",
                    category=AssetCategory.SPRITE_2D,
                    prompt=sprite if isinstance(sprite, str) else sprite.get("description", ""),
                    output_path=f"assets/sprites/{scene_name}",
                ))

            for texture in scene.get("textures", []):
                task_id += 1
                tasks.append(AssetTask(
                    id=f"texture_{task_id:03d}",
                    category=AssetCategory.TEXTURE,
                    prompt=texture if isinstance(texture, str) else texture.get("description", ""),
                    output_path=f"assets/textures/{scene_name}",
                ))

        global_assets = design_doc.get("assets_needed", {})
        for audio_item in global_assets.get("audio", []):
            task_id += 1
            cat = AssetCategory.BGM if audio_item.get("type") == "bgm" else AssetCategory.SFX
            tasks.append(AssetTask(
                id=f"audio_{task_id:03d}",
                category=cat,
                prompt=audio_item.get("description", ""),
                duration_sec=audio_item.get("duration_sec"),
                output_path=f"assets/audio/{cat.value}",
            ))

        for sprite_item in global_assets.get("sprites", []):
            task_id += 1
            tasks.append(AssetTask(
                id=f"sprite_{task_id:03d}",
                category=AssetCategory.SPRITE_2D,
                prompt=sprite_item if isinstance(sprite_item, str) else sprite_item.get("description", ""),
                output_path="assets/sprites",
            ))

        return tasks

    async def execute_all(self, tasks: list[AssetTask]) -> list[AssetTaskResult]:
        audio_tasks = [t for t in tasks if t.category in (AssetCategory.BGM, AssetCategory.SFX)]
        image_tasks = [t for t in tasks if t.category in (AssetCategory.SPRITE_2D, AssetCategory.TEXTURE)]

        audio_sem = asyncio.Semaphore(self.config.max_parallel_audio)
        image_sem = asyncio.Semaphore(self.config.max_parallel_image)

        async def run_with_sem(sem, task):
            async with sem:
                return await self._execute_task(task)

        coros = []
        for t in audio_tasks:
            coros.append(run_with_sem(audio_sem, t))
        for t in image_tasks:
            coros.append(run_with_sem(image_sem, t))

        results = await asyncio.gather(*coros, return_exceptions=True)

        final_results = []
        all_tasks = audio_tasks + image_tasks
        for task, result in zip(all_tasks, results):
            if isinstance(result, Exception):
                final_results.append(AssetTaskResult(
                    task=task, success=False, error=str(result)
                ))
            else:
                final_results.append(result)

        return final_results

    async def _execute_task(self, task: AssetTask) -> AssetTaskResult:
        last_error = None
        for attempt in range(self.config.max_retries + 1):
            try:
                if task.category in (AssetCategory.BGM, AssetCategory.SFX):
                    return await self._call_audio_pipeline(task)
                else:
                    return await self._call_image_pipeline(task)
            except Exception as e:
                last_error = e
                if attempt < self.config.max_retries:
                    await asyncio.sleep(self.config.retry_delay_sec)

        return AssetTaskResult(
            task=task, success=False, error=f"failed after {self.config.max_retries + 1} attempts: {last_error}"
        )

    async def _call_audio_pipeline(self, task: AssetTask) -> AssetTaskResult:
        url = f"{self.config.endpoints.audio_url}/generate"
        payload = {
            "prompt": task.prompt,
            "audio_type": task.category.value,
            "duration_sec": task.duration_sec,
            "output_dir": task.output_path,
        }
        resp = await self._client.post(url, json=payload)
        resp.raise_for_status()
        data = resp.json()

        return AssetTaskResult(
            task=task,
            success=data.get("validation", {}).get("valid", True),
            file_path=data.get("file_path"),
            metadata=data,
        )

    async def _call_image_pipeline(self, task: AssetTask) -> AssetTaskResult:
        url = f"{self.config.endpoints.image_url}/generate"
        payload = {
            "prompt": task.prompt,
            "asset_type": task.category.value,
            "output_dir": task.output_path,
        }
        try:
            resp = await self._client.post(url, json=payload)
            resp.raise_for_status()
            data = resp.json()
            return AssetTaskResult(
                task=task,
                success=True,
                file_path=data.get("file_path"),
                metadata=data,
            )
        except httpx.ConnectError:
            return AssetTaskResult(
                task=task,
                success=False,
                error="image pipeline not available (degraded mode)",
            )

    async def run_from_design_doc(self, design_doc: dict) -> dict:
        tasks = self.parse_design_doc(design_doc)
        results = await self.execute_all(tasks)

        succeeded = [r for r in results if r.success]
        failed = [r for r in results if not r.success]

        registry = {}
        for r in succeeded:
            if r.file_path:
                registry[r.task.id] = {
                    "category": r.task.category.value,
                    "prompt": r.task.prompt,
                    "file_path": r.file_path,
                    "metadata": r.metadata,
                }

        return {
            "total_tasks": len(tasks),
            "succeeded": len(succeeded),
            "failed": len(failed),
            "failures": [{"id": r.task.id, "error": r.error} for r in failed],
            "asset_registry": registry,
        }
