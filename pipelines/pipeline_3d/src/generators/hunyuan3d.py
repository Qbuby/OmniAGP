import logging
from pathlib import Path
from typing import Optional
import base64

import httpx

from ..config import settings

logger = logging.getLogger(__name__)


class Hunyuan3DGenerator:
    """Generate 3D mesh from a reference image using Hunyuan3D-2 cloud API."""

    def __init__(self):
        self._client: Optional[httpx.AsyncClient] = None

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None or self._client.is_closed:
            self._client = httpx.AsyncClient(timeout=settings.hunyuan3d_timeout)
        return self._client

    async def generate(self, image_path: Path, output_path: Path) -> Path:
        if not settings.hunyuan3d_api_url:
            raise RuntimeError("Hunyuan3D-2 API URL not configured (PIPELINE3D_HUNYUAN3D_API_URL)")

        client = await self._get_client()

        image_data = image_path.read_bytes()
        image_b64 = base64.b64encode(image_data).decode("utf-8")

        payload = {
            "image": image_b64,
            "output_format": "glb",
            "remove_background": True,
            "foreground_ratio": 0.85,
        }

        headers = {}
        if settings.hunyuan3d_api_key:
            headers["Authorization"] = f"Bearer {settings.hunyuan3d_api_key}"

        logger.info("Calling Hunyuan3D-2 API...")
        response = await client.post(
            f"{settings.hunyuan3d_api_url}/generate",
            json=payload,
            headers=headers,
        )
        response.raise_for_status()

        result = response.json()

        if "model" in result:
            model_data = base64.b64decode(result["model"])
        elif "glb" in result:
            model_data = base64.b64decode(result["glb"])
        else:
            model_data = response.content

        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(model_data)
        logger.info(f"Hunyuan3D-2 mesh saved: {output_path}")
        return output_path

    async def close(self):
        if self._client and not self._client.is_closed:
            await self._client.aclose()
