import logging
from pathlib import Path

import trimesh

from ..config import settings
from ..models import AssetType, MeshMetrics

logger = logging.getLogger(__name__)


class QualityValidator:
    """Validate mesh quality: manifold check, vertex budget, file size."""

    def validate(self, glb_path: Path, asset_type: AssetType) -> MeshMetrics:
        mesh = trimesh.load(str(glb_path), force="mesh")
        file_size_mb = glb_path.stat().st_size / (1024 * 1024)

        max_verts = (
            settings.max_vertices_character
            if asset_type == AssetType.CHARACTER
            else settings.max_vertices_prop
        )

        is_manifold = mesh.is_watertight
        has_valid_normals = self._check_normals(mesh)
        has_uv = hasattr(mesh.visual, "uv") and mesh.visual.uv is not None
        within_budget = (
            len(mesh.vertices) <= max_verts
            and file_size_mb <= settings.max_file_size_mb
        )

        metrics = MeshMetrics(
            vertex_count=len(mesh.vertices),
            face_count=len(mesh.faces),
            is_manifold=is_manifold,
            file_size_mb=round(file_size_mb, 2),
            has_valid_normals=has_valid_normals,
            has_uv=has_uv,
            within_budget=within_budget,
        )

        logger.info(
            f"Quality validation: vertices={metrics.vertex_count}, "
            f"faces={metrics.face_count}, manifold={is_manifold}, "
            f"size={metrics.file_size_mb}MB, budget_ok={within_budget}"
        )

        return metrics

    def _check_normals(self, mesh: trimesh.Trimesh) -> bool:
        if mesh.vertex_normals is None:
            return False
        norms = (mesh.vertex_normals ** 2).sum(axis=1) ** 0.5
        return bool((abs(norms - 1.0) < 0.01).all())
