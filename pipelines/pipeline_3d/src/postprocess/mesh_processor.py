import logging
from pathlib import Path

import numpy as np
import trimesh

from ..config import settings
from ..models import AssetType

logger = logging.getLogger(__name__)


class MeshProcessor:
    """Post-process 3D meshes: vertex budget, normals, pivot, UV check."""

    def process(self, glb_path: Path, asset_type: AssetType) -> Path:
        logger.info(f"Post-processing mesh: {glb_path}")
        mesh = trimesh.load(str(glb_path), force="mesh")

        mesh = self._enforce_vertex_budget(mesh, asset_type)
        mesh = self._fix_normals(mesh)
        mesh = self._center_pivot(mesh)
        self._check_uv(mesh)

        mesh.export(str(glb_path), file_type="glb")
        logger.info(f"Post-processing complete: {glb_path}")
        return glb_path

    def _enforce_vertex_budget(self, mesh: trimesh.Trimesh, asset_type: AssetType) -> trimesh.Trimesh:
        max_verts = (
            settings.max_vertices_character
            if asset_type == AssetType.CHARACTER
            else settings.max_vertices_prop
        )

        if len(mesh.vertices) > max_verts:
            ratio = max_verts / len(mesh.vertices)
            target_faces = int(len(mesh.faces) * ratio)
            target_faces = max(target_faces, 100)

            mesh = mesh.simplify_quadric_decimation(target_faces)
            logger.info(
                f"Decimated mesh: {len(mesh.vertices)} vertices "
                f"(budget: {max_verts})"
            )

        return mesh

    def _fix_normals(self, mesh: trimesh.Trimesh) -> trimesh.Trimesh:
        mesh.fix_normals()
        return mesh

    def _center_pivot(self, mesh: trimesh.Trimesh) -> trimesh.Trimesh:
        centroid = mesh.centroid
        mesh.vertices -= centroid

        bounds = mesh.bounds
        min_y = bounds[0][1]
        mesh.vertices[:, 1] -= min_y

        return mesh

    def _check_uv(self, mesh: trimesh.Trimesh):
        if not hasattr(mesh.visual, "uv") or mesh.visual.uv is None:
            logger.warning("Mesh has no UV coordinates — auto-generating box projection")
            self._generate_box_uv(mesh)

    def _generate_box_uv(self, mesh: trimesh.Trimesh):
        vertices = mesh.vertices
        normals = mesh.vertex_normals

        abs_normals = np.abs(normals)
        dominant = np.argmax(abs_normals, axis=1)

        uv = np.zeros((len(vertices), 2), dtype=np.float64)

        x_mask = dominant == 0
        y_mask = dominant == 1
        z_mask = dominant == 2

        uv[x_mask] = vertices[x_mask][:, [1, 2]]
        uv[y_mask] = vertices[y_mask][:, [0, 2]]
        uv[z_mask] = vertices[z_mask][:, [0, 1]]

        uv_min = uv.min(axis=0)
        uv_max = uv.max(axis=0)
        uv_range = uv_max - uv_min
        uv_range[uv_range == 0] = 1.0
        uv = (uv - uv_min) / uv_range

        mesh.visual = trimesh.visual.TextureVisual(uv=uv)
