# 2D Asset Generation Pipeline

ComfyUI-based sidecar service for generating game-ready 2D assets (sprites, icons, tilesets).

## Architecture

```
API Gateway (Rust/Axum :8080)
  └─► asset-2d sidecar (Python/FastAPI :8100)
        └─► ComfyUI (SDXL :8188)
```

## Features

- SDXL fp16 inference (optimized for 5060Ti 16GB VRAM)
- Three style presets: pixel (with LoRA), anime, realistic
- Automatic background removal (rembg)
- Tileset auto-slicing
- IP-Adapter style consistency (via reference image)
- Base64 or file output modes
- Model unload endpoint for VRAM management
- Workflow template loading from JSON files
- Serial execution with async lock (single GPU)

## Quick Start

```bash
# Start ComfyUI + sidecar
docker compose -f docker/docker-compose.2d.yml up -d

# Generate a pixel art sprite
curl -X POST http://localhost:8080/api/v1/generate/2d \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "a medieval knight character, side view, idle pose",
    "style": "pixel",
    "category": "sprite",
    "width": 512,
    "height": 512,
    "remove_background": true
  }'
```

## API

### POST /api/v1/generate/2d

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| prompt | string | required | Text description of the asset |
| negative_prompt | string | "blurry, low quality..." | What to avoid |
| style | "pixel" / "anime" / "realistic" | "pixel" | Visual style preset |
| category | "sprite" / "icon" / "tileset" | "sprite" | Asset category |
| width | int (64-2048) | 1024 | Output width |
| height | int (64-2048) | 1024 | Output height |
| remove_background | bool | true | Auto background removal |
| tile_size | int (16-512) | null | Tileset slice size (only for tileset category) |
| seed | int | -1 (random) | Reproducibility seed |
| steps | int (1-100) | 25 | Diffusion steps |
| cfg_scale | float (1-30) | 7.0 | Classifier-free guidance scale |
| reference_image_b64 | string | null | Base64 reference image for IP-Adapter |
| output_format | "file" / "base64" | "file" | Output mode |

### Response

```json
{
  "request_id": "uuid",
  "status": "success",
  "generation_time_ms": 12500,
  "assets": [
    {
      "file_path": "/tmp/omni-assets/uuid.png",
      "data_b64": null,
      "width": 512,
      "height": 512,
      "has_alpha": true,
      "file_size_bytes": 45230
    }
  ],
  "errors": []
}
```

### GET /api/v1/generate/2d/health

Returns ComfyUI connection status and VRAM info.

### POST /api/v1/generate/2d/unload

Frees VRAM by unloading all loaded models from ComfyUI.

## Testing

```bash
cd pipelines/asset-2d
pip install -r requirements.txt -r requirements-dev.txt
pytest test_main.py -v
```

## Required Models

Place in ComfyUI models directory:
- `checkpoints/sd_xl_base_1.0.safetensors` — SDXL base
- `loras/pixel-art-xl-v1.1.safetensors` — Pixel Art LoRA (for pixel style)
- IP-Adapter model (for reference image feature)

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| COMFYUI_URL | http://127.0.0.1:8188 | ComfyUI server address |
| ASSET_2D_URL | http://127.0.0.1:8100 | Sidecar address (for api-gateway) |
| OUTPUT_DIR | /tmp/omni-assets | Generated asset output directory |
| WORKFLOW_DIR | ./workflows | Directory containing workflow JSON templates |
