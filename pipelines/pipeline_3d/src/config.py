from pydantic_settings import BaseSettings
from pydantic import Field


class Settings(BaseSettings):
    model_config = {"env_prefix": "PIPELINE3D_"}

    # TripoSR local inference
    triposr_model_id: str = "stabilityai/TripoSR"
    triposr_device: str = "cuda"
    triposr_dtype: str = "float16"
    triposr_chunk_size: int = 8192
    triposr_resolution: int = 256

    # Hunyuan3D-2 cloud API
    hunyuan3d_api_url: str = ""
    hunyuan3d_api_key: str = ""
    hunyuan3d_timeout: int = 90

    # SDXL reference image generation
    sdxl_api_url: str = ""
    sdxl_api_key: str = ""
    sdxl_model: str = "stabilityai/stable-diffusion-xl-base-1.0"
    sdxl_steps: int = 30
    sdxl_width: int = 1024
    sdxl_height: int = 1024

    # Mesh constraints
    max_vertices_character: int = 50000
    max_vertices_prop: int = 5000
    max_file_size_mb: float = 50.0

    # Output
    output_dir: str = "./output"
    temp_dir: str = "./tmp"

    # Generation
    default_backend: str = "triposr"
    generation_timeout: int = 120


settings = Settings()
