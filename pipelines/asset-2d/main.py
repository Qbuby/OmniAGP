"""2D Asset Generation Sidecar — ComfyUI + Rembg + Tileset slicing."""

import asyncio
import base64
import io
import json
import logging
import os
import time
import uuid
from enum import Enum
from pathlib import Path
from typing import Optional

import aiohttp
import numpy as np
from fastapi import FastAPI, HTTPException
from PIL import Image
from pydantic import BaseModel, Field
from rembg import remove as rembg_remove

app = FastAPI(title="OmniAGP 2D Asset Pipeline", version="0.2.0")
logger = logging.getLogger("asset-2d")
logging.basicConfig(level=logging.INFO)

COMFYUI_URL = os.getenv("COMFYUI_URL", "http://127.0.0.1:8188")
OUTPUT_DIR = Path(os.getenv("OUTPUT_DIR", "/tmp/omni-assets"))
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
WORKFLOW_DIR = Path(os.getenv("WORKFLOW_DIR", str(Path(__file__).parent / "workflows")))

_generation_lock = asyncio.Lock()


class StylePreset(str, Enum):
    pixel = "pixel"
    anime = "anime"
    realistic = "realistic"


class AssetCategory(str, Enum):
    sprite = "sprite"
    icon = "icon"
    tileset = "tileset"


class Generate2DRequest(BaseModel):
    prompt: str = Field(..., min_length=1, max_length=2000)
    negative_prompt: str = Field(default="blurry, low quality, watermark, text, signature")
    style: StylePreset = StylePreset.pixel
    category: AssetCategory = AssetCategory.sprite
    width: int = Field(default=1024, ge=64, le=2048)
    height: int = Field(default=1024, ge=64, le=2048)
    remove_background: bool = True
    tile_size: Optional[int] = Field(default=None, ge=16, le=512)
    seed: int = Field(default=-1)
    steps: int = Field(default=25, ge=1, le=100)
    cfg_scale: float = Field(default=7.0, ge=1.0, le=30.0)
    reference_image_b64: Optional[str] = None
    output_format: str = Field(default="file", pattern="^(file|base64)$")


class AssetOutput(BaseModel):
    file_path: Optional[str] = None
    data_b64: Optional[str] = None
    width: int
    height: int
    has_alpha: bool
    file_size_bytes: int


class Generate2DResponse(BaseModel):
    request_id: str
    status: str
    generation_time_ms: int
    assets: list[AssetOutput]
    errors: list[str] = []


class HealthResponse(BaseModel):
    status: str
    comfyui_connected: bool
    vram_free_mb: Optional[float] = None


def _load_workflow_template(style: StylePreset) -> Optional[dict]:
    template_path = WORKFLOW_DIR / f"{style.value}.json"
    if not template_path.exists():
        return None
    with open(template_path) as f:
        data = json.load(f)
    return data.get("nodes", data)


@app.get("/health", response_model=HealthResponse)
async def health():
    connected, vram = await _check_comfyui()
    status = "ok" if connected else "degraded"
    return HealthResponse(status=status, comfyui_connected=connected, vram_free_mb=vram)


@app.post("/generate", response_model=Generate2DResponse)
async def generate_2d(req: Generate2DRequest):
    request_id = str(uuid.uuid4())
    start = time.time()
    errors: list[str] = []
    assets: list[AssetOutput] = []

    async with _generation_lock:
        try:
            raw_image = await _run_comfyui_workflow(req)
        except Exception as e:
            logger.error(f"ComfyUI generation failed: {e}")
            raise HTTPException(status_code=502, detail=f"ComfyUI generation failed: {e}")

        if req.remove_background:
            try:
                raw_image = _remove_background(raw_image)
            except Exception as e:
                errors.append(f"Background removal failed: {e}")

        if req.category == AssetCategory.tileset and req.tile_size:
            tiles = _slice_tileset(raw_image, req.tile_size)
            for i, tile in enumerate(tiles):
                out = _save_asset(tile, request_id, req.output_format, suffix=f"_tile_{i:03d}")
                assets.append(out)
        else:
            out = _save_asset(raw_image, request_id, req.output_format)
            assets.append(out)

    elapsed_ms = int((time.time() - start) * 1000)

    validation_errors = _validate_assets(assets, req.remove_background)
    errors.extend(validation_errors)

    return Generate2DResponse(
        request_id=request_id,
        status="success" if not errors else "partial",
        generation_time_ms=elapsed_ms,
        assets=assets,
        errors=errors,
    )


@app.post("/unload-models")
async def unload_models():
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(
                f"{COMFYUI_URL}/free",
                json={"unload_models": True, "free_memory": True},
            ) as resp:
                if resp.status == 200:
                    return {"status": "ok", "message": "Models unloaded"}
                return {"status": "error", "message": f"ComfyUI returned {resp.status}"}
    except Exception as e:
        raise HTTPException(status_code=502, detail=str(e))


async def _check_comfyui() -> tuple[bool, Optional[float]]:
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{COMFYUI_URL}/system_stats",
                timeout=aiohttp.ClientTimeout(total=5),
            ) as resp:
                if resp.status != 200:
                    return False, None
                data = await resp.json()
                devices = data.get("devices", [])
                vram_free = None
                if devices:
                    dev = devices[0]
                    vram_total = dev.get("vram_total", 0)
                    vram_used = dev.get("vram_free", 0)
                    if vram_total > 0:
                        vram_free = vram_used / (1024 * 1024)
                return True, vram_free
    except Exception:
        return False, None


async def _run_comfyui_workflow(req: Generate2DRequest) -> Image.Image:
    workflow = _build_workflow(req)

    async with aiohttp.ClientSession() as session:
        prompt_payload = {"prompt": workflow}
        async with session.post(f"{COMFYUI_URL}/prompt", json=prompt_payload) as resp:
            if resp.status != 200:
                body = await resp.text()
                raise RuntimeError(f"ComfyUI /prompt returned {resp.status}: {body}")
            data = await resp.json()
            prompt_id = data["prompt_id"]

        image_data = await _poll_for_result(session, prompt_id)

    return Image.open(io.BytesIO(image_data)).convert("RGBA")


async def _poll_for_result(session: aiohttp.ClientSession, prompt_id: str) -> bytes:
    for _ in range(120):
        await asyncio.sleep(0.5)
        async with session.get(f"{COMFYUI_URL}/history/{prompt_id}") as resp:
            if resp.status != 200:
                continue
            history = await resp.json()
            if prompt_id not in history:
                continue
            entry = history[prompt_id]
            if entry.get("status", {}).get("status_str") == "error":
                msgs = entry.get("status", {}).get("messages", [])
                raise RuntimeError(f"ComfyUI workflow error: {msgs}")
            outputs = entry.get("outputs", {})
            for node_id, node_output in outputs.items():
                images = node_output.get("images", [])
                if images:
                    img_info = images[0]
                    filename = img_info["filename"]
                    subfolder = img_info.get("subfolder", "")
                    img_type = img_info.get("type", "output")
                    params = {"filename": filename, "subfolder": subfolder, "type": img_type}
                    async with session.get(f"{COMFYUI_URL}/view", params=params) as img_resp:
                        if img_resp.status == 200:
                            return await img_resp.read()
    raise TimeoutError("ComfyUI generation timed out after 60s")


def _build_workflow(req: Generate2DRequest) -> dict:
    seed = req.seed if req.seed >= 0 else int.from_bytes(os.urandom(4), "big")

    template = _load_workflow_template(req.style)
    if template:
        return _apply_template(template, req, seed)

    return _build_workflow_inline(req, seed)


def _apply_template(template: dict, req: Generate2DRequest, seed: int) -> dict:
    workflow = json.loads(json.dumps(template))

    replacements = {
        "{{prompt}}": req.prompt,
        "{{negative_prompt}}": req.negative_prompt,
        "{{width}}": req.width,
        "{{height}}": req.height,
        "{{seed}}": seed,
    }

    def replace_values(obj):
        if isinstance(obj, dict):
            return {k: replace_values(v) for k, v in obj.items()}
        if isinstance(obj, list):
            return [replace_values(item) for item in obj]
        if isinstance(obj, str):
            for placeholder, value in replacements.items():
                if obj == placeholder:
                    return value
                if placeholder in obj:
                    obj = obj.replace(placeholder, str(value))
            return obj
        return obj

    workflow = replace_values(workflow)

    if req.steps and "5" in workflow:
        workflow["5"]["inputs"]["steps"] = req.steps
    if req.cfg_scale and "5" in workflow:
        workflow["5"]["inputs"]["cfg"] = req.cfg_scale

    if req.reference_image_b64:
        model_source = ["10", 0] if "10" in workflow else ["1", 0]
        workflow["20"] = {
            "class_type": "IPAdapterApply",
            "inputs": {
                "model": model_source,
                "image": ["21", 0],
                "weight": 0.6,
                "noise": 0.0,
            },
        }
        workflow["21"] = {
            "class_type": "LoadImageFromBase64",
            "inputs": {"image": req.reference_image_b64},
        }
        workflow["5"]["inputs"]["model"] = ["20", 0]

    return workflow


def _build_workflow_inline(req: Generate2DRequest, seed: int) -> dict:
    checkpoint = "sd_xl_base_1.0.safetensors"
    lora_node = None

    style_suffix = ""
    if req.style == StylePreset.pixel:
        style_suffix = ", pixel art style, 16-bit, retro game sprite"
        lora_node = {
            "class_type": "LoraLoader",
            "inputs": {
                "model": ["1", 0],
                "clip": ["1", 1],
                "lora_name": "pixel-art-xl-v1.1.safetensors",
                "strength_model": 0.85,
                "strength_clip": 0.85,
            },
        }
    elif req.style == StylePreset.anime:
        style_suffix = ", anime style, cel shading, clean lines"
    elif req.style == StylePreset.realistic:
        style_suffix = ", photorealistic, detailed, high quality"

    full_prompt = req.prompt + style_suffix

    workflow = {
        "1": {
            "class_type": "CheckpointLoaderSimple",
            "inputs": {"ckpt_name": checkpoint},
        },
        "2": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "text": full_prompt,
                "clip": ["1", 1] if not lora_node else ["10", 1],
            },
        },
        "3": {
            "class_type": "CLIPTextEncode",
            "inputs": {
                "text": req.negative_prompt,
                "clip": ["1", 1] if not lora_node else ["10", 1],
            },
        },
        "4": {
            "class_type": "EmptyLatentImage",
            "inputs": {
                "width": req.width,
                "height": req.height,
                "batch_size": 1,
            },
        },
        "5": {
            "class_type": "KSampler",
            "inputs": {
                "model": ["1", 0] if not lora_node else ["10", 0],
                "positive": ["2", 0],
                "negative": ["3", 0],
                "latent_image": ["4", 0],
                "seed": seed,
                "steps": req.steps,
                "cfg": req.cfg_scale,
                "sampler_name": "euler_ancestral",
                "scheduler": "normal",
                "denoise": 1.0,
            },
        },
        "6": {
            "class_type": "VAEDecode",
            "inputs": {
                "samples": ["5", 0],
                "vae": ["1", 2],
            },
        },
        "7": {
            "class_type": "SaveImage",
            "inputs": {
                "images": ["6", 0],
                "filename_prefix": "omni_2d",
            },
        },
    }

    if lora_node:
        workflow["10"] = lora_node

    if req.reference_image_b64:
        workflow["20"] = {
            "class_type": "IPAdapterApply",
            "inputs": {
                "model": ["10", 0] if lora_node else ["1", 0],
                "image": ["21", 0],
                "weight": 0.6,
                "noise": 0.0,
            },
        }
        workflow["21"] = {
            "class_type": "LoadImageFromBase64",
            "inputs": {"image": req.reference_image_b64},
        }
        workflow["5"]["inputs"]["model"] = ["20", 0]

    return workflow


def _remove_background(image: Image.Image) -> Image.Image:
    img_bytes = io.BytesIO()
    image.save(img_bytes, format="PNG")
    img_bytes.seek(0)
    result_bytes = rembg_remove(img_bytes.getvalue())
    return Image.open(io.BytesIO(result_bytes)).convert("RGBA")


def _slice_tileset(image: Image.Image, tile_size: int) -> list[Image.Image]:
    tiles = []
    w, h = image.size
    for y in range(0, h - tile_size + 1, tile_size):
        for x in range(0, w - tile_size + 1, tile_size):
            tile = image.crop((x, y, x + tile_size, y + tile_size))
            tiles.append(tile)
    return tiles


def _save_asset(image: Image.Image, request_id: str, output_format: str, suffix: str = "") -> AssetOutput:
    img_bytes = io.BytesIO()
    image.save(img_bytes, format="PNG")
    raw = img_bytes.getvalue()

    if output_format == "base64":
        return AssetOutput(
            data_b64=base64.b64encode(raw).decode(),
            width=image.width,
            height=image.height,
            has_alpha=image.mode == "RGBA",
            file_size_bytes=len(raw),
        )

    filename = f"{request_id}{suffix}.png"
    filepath = OUTPUT_DIR / filename
    filepath.write_bytes(raw)
    return AssetOutput(
        file_path=str(filepath),
        width=image.width,
        height=image.height,
        has_alpha=image.mode == "RGBA",
        file_size_bytes=len(raw),
    )


def _validate_assets(assets: list[AssetOutput], expect_alpha: bool) -> list[str]:
    errors = []
    for asset in assets:
        if expect_alpha and not asset.has_alpha:
            name = asset.file_path or "base64 asset"
            errors.append(f"{name}: expected RGBA but got no alpha channel")
        if asset.width < 16 or asset.height < 16:
            name = asset.file_path or "base64 asset"
            errors.append(f"{name}: image too small ({asset.width}x{asset.height})")
    return errors


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8100)
