import asyncio
import logging
import time
import uuid
from pathlib import Path

from .config import settings
from .models import AssetType, GenerateRequest, GenerateResponse, GenerationBackend, MeshMetrics
from .generators.reference_image import ReferenceImageGenerator
from .generators.triposr import TripoSRGenerator
from .generators.hunyuan3d import Hunyuan3DGenerator
from .postprocess.mesh_processor import MeshProcessor
from .postprocess.quality_validator import QualityValidator

logger = logging.getLogger(__name__)


class Pipeline3D:
    """Orchestrates the full text-to-3D generation pipeline."""

    def __init__(self):
        self.ref_image_gen = ReferenceImageGenerator()
        self.triposr = TripoSRGenerator()
        self.hunyuan3d = Hunyuan3DGenerator()
        self.mesh_processor = MeshProcessor()
        self.quality_validator = QualityValidator()

    async def generate(self, request: GenerateRequest) -> GenerateResponse:
        task_id = uuid.uuid4().hex[:12]
        start_time = time.time()

        output_dir = Path(settings.output_dir) / task_id
        temp_dir = Path(settings.temp_dir) / task_id
        output_dir.mkdir(parents=True, exist_ok=True)
        temp_dir.mkdir(parents=True, exist_ok=True)

        try:
            # Step 1: Generate reference image from text prompt
            ref_image_path = temp_dir / "reference.png"
            logger.info(f"[{task_id}] Generating reference image...")
            await self.ref_image_gen.generate(
                prompt=request.prompt,
                output_path=ref_image_path,
                negative_prompt=request.negative_prompt,
                seed=request.seed,
            )

            # Step 2: Generate 3D mesh from reference image
            raw_glb_path = temp_dir / "raw_model.glb"
            backend = request.backend or GenerationBackend(settings.default_backend)

            logger.info(f"[{task_id}] Generating 3D mesh via {backend.value}...")
            if backend == GenerationBackend.TRIPOSR:
                await asyncio.to_thread(
                    self.triposr.generate, ref_image_path, raw_glb_path
                )
            else:
                await self.hunyuan3d.generate(ref_image_path, raw_glb_path)

            # Step 3: Post-process mesh
            final_glb_path = output_dir / f"{task_id}.glb"
            raw_glb_path.rename(final_glb_path)

            logger.info(f"[{task_id}] Post-processing mesh...")
            self.mesh_processor.process(final_glb_path, request.asset_type)

            # Step 4: Quality validation
            logger.info(f"[{task_id}] Validating mesh quality...")
            metrics = self.quality_validator.validate(final_glb_path, request.asset_type)

            elapsed = time.time() - start_time
            logger.info(f"[{task_id}] Pipeline complete in {elapsed:.1f}s")

            return GenerateResponse(
                task_id=task_id,
                status="success",
                glb_path=str(final_glb_path),
                metrics=metrics,
                generation_time_seconds=round(elapsed, 2),
            )

        except Exception as e:
            elapsed = time.time() - start_time
            logger.error(f"[{task_id}] Pipeline failed: {e}")
            return GenerateResponse(
                task_id=task_id,
                status="error",
                error=str(e),
                generation_time_seconds=round(elapsed, 2),
            )
        finally:
            # Cleanup temp files
            import shutil
            shutil.rmtree(temp_dir, ignore_errors=True)

    async def close(self):
        await self.ref_image_gen.close()
        await self.hunyuan3d.close()
        self.triposr.unload()
