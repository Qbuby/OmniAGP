from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    host: str = "0.0.0.0"
    port: int = 8100
    workers: int = 1
    max_concurrent_jobs: int = 4
    output_dir: str = "/tmp/asset-sidecar/output"
    log_level: str = "info"
    rust_orchestrator_url: str = "http://localhost:8080"

    model_config = {"env_prefix": "SIDECAR_"}


settings = Settings()
