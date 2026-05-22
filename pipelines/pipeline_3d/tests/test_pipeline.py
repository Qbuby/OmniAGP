import pytest
from pathlib import Path
import tempfile
import numpy as np

import trimesh

from src.postprocess.mesh_processor import MeshProcessor
from src.postprocess.quality_validator import QualityValidator
from src.models import AssetType


@pytest.fixture
def sample_glb(tmp_path: Path) -> Path:
    mesh = trimesh.creation.box(extents=[1.0, 2.0, 1.0])
    glb_path = tmp_path / "test_model.glb"
    mesh.export(str(glb_path), file_type="glb")
    return glb_path


@pytest.fixture
def high_poly_glb(tmp_path: Path) -> Path:
    sphere = trimesh.creation.icosphere(subdivisions=5)
    glb_path = tmp_path / "high_poly.glb"
    sphere.export(str(glb_path), file_type="glb")
    return glb_path


class TestMeshProcessor:
    def test_process_centers_pivot(self, sample_glb: Path):
        processor = MeshProcessor()
        processor.process(sample_glb, AssetType.PROP)

        mesh = trimesh.load(str(sample_glb), force="mesh")
        centroid_xz = mesh.centroid[[0, 2]]
        assert np.allclose(centroid_xz, 0, atol=0.01)
        assert mesh.bounds[0][1] >= -0.01

    def test_process_fixes_normals(self, sample_glb: Path):
        processor = MeshProcessor()
        processor.process(sample_glb, AssetType.PROP)

        mesh = trimesh.load(str(sample_glb), force="mesh")
        norms = np.linalg.norm(mesh.vertex_normals, axis=1)
        assert np.allclose(norms, 1.0, atol=0.01)

    def test_decimation_enforces_budget(self, high_poly_glb: Path):
        processor = MeshProcessor()
        processor.process(high_poly_glb, AssetType.PROP)

        mesh = trimesh.load(str(high_poly_glb), force="mesh")
        assert len(mesh.vertices) <= 5000


class TestQualityValidator:
    def test_validate_returns_metrics(self, sample_glb: Path):
        validator = QualityValidator()
        metrics = validator.validate(sample_glb, AssetType.PROP)

        assert metrics.vertex_count > 0
        assert metrics.face_count > 0
        assert metrics.file_size_mb > 0
        assert metrics.within_budget is True

    def test_validate_detects_over_budget(self, high_poly_glb: Path):
        validator = QualityValidator()
        metrics = validator.validate(high_poly_glb, AssetType.PROP)

        assert metrics.vertex_count > 5000
        assert metrics.within_budget is False


class TestPipelineModels:
    def test_generate_request_validation(self):
        from src.models import GenerateRequest

        req = GenerateRequest(prompt="a sword", asset_type="prop")
        assert req.prompt == "a sword"
        assert req.asset_type == AssetType.PROP

    def test_generate_request_rejects_empty_prompt(self):
        from src.models import GenerateRequest
        from pydantic import ValidationError

        with pytest.raises(ValidationError):
            GenerateRequest(prompt="")
