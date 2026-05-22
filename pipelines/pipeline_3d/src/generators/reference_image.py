import logging
from pathlib import Path
from typing import Optional

import httpx
from PIL import Image

from ..config import settings

logger = logging.getLogger(__name__)


class ReferenceImageGenerator:
    """Generate reference images from text prompts via SDXL API."""

    def __init__(self):
        self._client: Optional[httpx.AsyncClient] = None

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None or self._client.is_closed:
            self._client = httpx.AsyncClient(timeout=60.0)
        return self._client

    async def generate(
        self,
        prompt: str,
        output_path: Path,
        negative_prompt: str = "",
        seed: Optional[int] = None,
    ) -> Path:
        if settings.sdxl_api_url:
            return await self._generate_via_api(prompt, output_path, negative_prompt, seed)
        return await self._generate_local(prompt, output_path, negative_prompt, seed)

    async def _generate_via_api(
        self,
        prompt: str,
        output_path: Path,
        negative_prompt: str,
        seed: Optional[int],
    ) -> Path:
        client = await self._get_client()
        payload = {
            "prompt": f"3d model reference, white background, centered object, {prompt}",
            "negative_prompt": f"blurry, low quality, text, watermark, {negative_prompt}",
            "width": settings.sdxl_width,
            "height": settings.sdxl_height,
            "steps": settings.sdxl_steps,
        }
        if seed is not None:
            payload["seed"] = seed

        headers = {}
        if settings.sdxl_api_key:
            headers["Authorization"] = f"Bearer {settings.sdxl_api_key}"

        response = await client.post(
            f"{settings.sdxl_api_url}/generate",
            json=payload,
            headers=headers,
        )
        response.raise_for_status()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(response.content)
        logger.info(f"Reference image saved to {output_path}")
        return output_path

    async def _generate_local(
        self,
        prompt: str,
        output_path: Path,
        negative_prompt: str,
        seed: Optional[int],
    ) -> Path:
        import torch
        from diffusers import StableDiffusionXLPipeline

        pipe = StableDiffusionXLPipeline.from_pretrained(
            settings.sdxl_model,
            torch_dtype=torch.float16,
            variant="fp16",
            use_safetensors=True,
        )
        pipe = pipe.to("cuda")

        generator = None
        if seed is not None:
            generator = torch.Generator(device="cuda").manual_seed(seed)

        image = pipe(
            prompt=f"3d model reference, white background, centered object, {prompt}",
            negative_prompt=f"blurry, low quality, text, watermark, {negative_prompt}",
            width=settings.sdxl_width,
            height=settings.sdxl_height,
            num_inference_steps=settings.sdxl_steps,
            generator=generator,
        ).images[0]

        output_path.parent.mkdir(parents=True, exist_ok=True)
        image.save(str(output_path))
        logger.info(f"Reference image generated locally: {output_path}")

        del pipe
        torch.cuda.empty_cache()
        return output_path

    async def close(self):
        if self._client and not self._client.is_closed:
            await self._client.aclose()
