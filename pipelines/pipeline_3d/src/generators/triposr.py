import logging
from pathlib import Path
from typing import Optional

import numpy as np
from PIL import Image

from ..config import settings

logger = logging.getLogger(__name__)


class TripoSRGenerator:
    """Generate 3D mesh from a reference image using TripoSR (local GPU inference)."""

    def __init__(self):
        self._model = None

    def _load_model(self):
        if self._model is not None:
            return

        import torch
        from tsr.system import TSR

        logger.info("Loading TripoSR model...")
        self._model = TSR.from_pretrained(
            settings.triposr_model_id,
            config_name="config.yaml",
            weight_name="model.ckpt",
        )
        self._model.renderer.set_chunk_size(settings.triposr_chunk_size)
        self._model.to(settings.triposr_device)
        logger.info("TripoSR model loaded successfully")

    def generate(self, image_path: Path, output_path: Path) -> Path:
        import torch

        self._load_model()

        image = Image.open(image_path).convert("RGB")

        logger.info("Running TripoSR inference...")
        with torch.no_grad():
            scene_codes = self._model([image], device=settings.triposr_device)

        meshes = self._model.extract_mesh(
            scene_codes,
            resolution=settings.triposr_resolution,
        )

        mesh = meshes[0]
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Export as OBJ first (TripoSR native), then convert to GLB
        obj_path = output_path.with_suffix(".obj")
        mesh.export(str(obj_path))

        self._convert_obj_to_glb(obj_path, output_path)
        obj_path.unlink(missing_ok=True)

        logger.info(f"3D mesh generated: {output_path}")
        return output_path

    def _convert_obj_to_glb(self, obj_path: Path, glb_path: Path):
        import trimesh

        mesh = trimesh.load(str(obj_path), force="mesh")
        mesh.export(str(glb_path), file_type="glb")

    def unload(self):
        if self._model is not None:
            import torch
            del self._model
            self._model = None
            torch.cuda.empty_cache()
            logger.info("TripoSR model unloaded")
