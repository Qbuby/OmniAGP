from pydantic import BaseModel


class AudioPipelineConfig(BaseModel):
    musicgen_model: str = "facebook/musicgen-medium"
    audiogen_model: str = "facebook/audiogen-medium"
    device: str = "cuda"
    dtype: str = "float16"
    bgm_target_lufs: float = -14.0
    sfx_target_lufs: float = -10.0
    default_duration_sec: float = 30.0
    sfx_duration_sec: float = 5.0
    output_format: str = "ogg"
    output_sample_rate: int = 44100
    fade_duration_sec: float = 2.0
    max_concurrent_jobs: int = 1
