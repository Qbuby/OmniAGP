# 3D Asset Generation Pipeline

从文本描述生成游戏用 3D 模型（.glb 格式）的微服务。

## 架构

```
pipeline_3d/
├── src/
│   ├── api/            # FastAPI 路由
│   ├── generators/     # 3D 生成后端
│   │   ├── reference_image.py  # 文本→SDXL参考图
│   │   ├── triposr.py          # TripoSR 本地推理
│   │   └── hunyuan3d.py        # Hunyuan3D-2 云端 API
│   ├── postprocess/    # Mesh 后处理
│   │   ├── mesh_processor.py   # 顶点裁剪/法线/Pivot/UV
│   │   └── quality_validator.py # 质量验证
│   ├── config.py       # 配置
│   ├── models.py       # 数据模型
│   ├── pipeline.py     # 管线编排
│   └── main.py         # 入口
├── tests/
├── Dockerfile
└── requirements.txt
```

## 快速启动

```bash
cd pipelines/pipeline_3d
pip install -r requirements.txt
python -m src.main
```

服务启动在 `http://localhost:8090`。

## API

### POST /generate/3d

```json
{
  "prompt": "a medieval sword with golden handle",
  "asset_type": "prop",
  "backend": "triposr",
  "negative_prompt": "",
  "seed": 42
}
```

响应：

```json
{
  "task_id": "a1b2c3d4e5f6",
  "status": "success",
  "glb_path": "./output/a1b2c3d4e5f6/a1b2c3d4e5f6.glb",
  "metrics": {
    "vertex_count": 3200,
    "face_count": 6400,
    "is_manifold": true,
    "file_size_mb": 1.2,
    "has_valid_normals": true,
    "has_uv": true,
    "within_budget": true
  },
  "generation_time_seconds": 45.2
}
```

### GET /health

返回服务健康状态。

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| PIPELINE3D_TRIPOSR_DEVICE | 推理设备 | cuda |
| PIPELINE3D_SDXL_API_URL | SDXL API 地址 | (空=本地推理) |
| PIPELINE3D_HUNYUAN3D_API_URL | Hunyuan3D-2 API 地址 | (空) |
| PIPELINE3D_HUNYUAN3D_API_KEY | Hunyuan3D-2 API Key | (空) |
| PIPELINE3D_DEFAULT_BACKEND | 默认后端 | triposr |
| PIPELINE3D_MAX_VERTICES_CHARACTER | 角色顶点上限 | 50000 |
| PIPELINE3D_MAX_VERTICES_PROP | 物件顶点上限 | 5000 |
| PIPELINE3D_OUTPUT_DIR | 输出目录 | ./output |

## Docker

```bash
docker build -t omniagp-pipeline-3d .
docker run --gpus all -p 8090:8090 omniagp-pipeline-3d
```
