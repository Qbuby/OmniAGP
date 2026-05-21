"""
RAG Knowledge Base Indexer for OmniAGP.

Indexes Godot 4 API docs, GDScript syntax reference, and code snippets
into Qdrant vector database for retrieval-augmented code generation.

Usage:
    python rag/index_knowledge.py --source docs
    python rag/index_knowledge.py --source snippets
    python rag/index_knowledge.py --source all
"""

import argparse
import json
import os
import uuid
from pathlib import Path
from typing import Generator

from openai import OpenAI
from qdrant_client import QdrantClient
from qdrant_client.models import Distance, PointStruct, VectorParams

COLLECTION_NAME = "godot_knowledge"
EMBEDDING_MODEL = os.getenv("EMBEDDING_MODEL", "text-embedding-3-small")
VECTOR_SIZE = 1536
CHUNK_SIZE = 1000
CHUNK_OVERLAP = 200

QDRANT_URL = os.getenv("QDRANT_URL", "http://localhost:6333")
LLM_BASE_URL = os.getenv("LLM_BASE_URL", "http://localhost:11434/v1")
LLM_API_KEY = os.getenv("LLM_API_KEY", "")


def get_qdrant() -> QdrantClient:
    return QdrantClient(url=QDRANT_URL)


def get_openai() -> OpenAI:
    return OpenAI(base_url=LLM_BASE_URL, api_key=LLM_API_KEY)


def ensure_collection(client: QdrantClient) -> None:
    collections = [c.name for c in client.get_collections().collections]
    if COLLECTION_NAME not in collections:
        client.create_collection(
            collection_name=COLLECTION_NAME,
            vectors_config=VectorParams(size=VECTOR_SIZE, distance=Distance.COSINE),
        )
        print(f"Created collection: {COLLECTION_NAME}")
    else:
        print(f"Collection exists: {COLLECTION_NAME}")


def chunk_text(text: str, chunk_size: int = CHUNK_SIZE, overlap: int = CHUNK_OVERLAP) -> list[str]:
    chunks = []
    start = 0
    while start < len(text):
        end = start + chunk_size
        chunks.append(text[start:end])
        start = end - overlap
    return chunks


def embed_texts(client: OpenAI, texts: list[str]) -> list[list[float]]:
    batch_size = 100
    all_embeddings = []
    for i in range(0, len(texts), batch_size):
        batch = texts[i : i + batch_size]
        response = client.embeddings.create(model=EMBEDDING_MODEL, input=batch)
        all_embeddings.extend([d.embedding for d in response.data])
    return all_embeddings


def load_gdscript_snippets(data_dir: Path) -> Generator[dict, None, None]:
    snippets_dir = data_dir / "snippets"
    if not snippets_dir.exists():
        print(f"Snippets directory not found: {snippets_dir}")
        print("Creating sample snippets...")
        snippets_dir.mkdir(parents=True, exist_ok=True)
        _create_sample_snippets(snippets_dir)

    for f in snippets_dir.glob("*.gd"):
        content = f.read_text(encoding="utf-8")
        yield {
            "content": content,
            "source": f"snippet:{f.stem}",
            "type": "code_snippet",
        }


def load_api_docs(data_dir: Path) -> Generator[dict, None, None]:
    docs_dir = data_dir / "docs"
    if not docs_dir.exists():
        print(f"Docs directory not found: {docs_dir}")
        print("Creating GDScript syntax reference...")
        docs_dir.mkdir(parents=True, exist_ok=True)
        _create_gdscript_reference(docs_dir)

    for f in docs_dir.glob("*.md"):
        content = f.read_text(encoding="utf-8")
        chunks = chunk_text(content)
        for i, chunk in enumerate(chunks):
            yield {
                "content": chunk,
                "source": f"doc:{f.stem}:chunk{i}",
                "type": "api_doc",
            }


def index_documents(
    qdrant: QdrantClient, openai_client: OpenAI, documents: list[dict]
) -> int:
    if not documents:
        return 0

    texts = [d["content"] for d in documents]
    embeddings = embed_texts(openai_client, texts)

    points = []
    for doc, embedding in zip(documents, embeddings):
        points.append(
            PointStruct(
                id=str(uuid.uuid4()),
                vector=embedding,
                payload=doc,
            )
        )

    batch_size = 100
    for i in range(0, len(points), batch_size):
        batch = points[i : i + batch_size]
        qdrant.upsert(collection_name=COLLECTION_NAME, points=batch)

    return len(points)


def _create_sample_snippets(snippets_dir: Path) -> None:
    snippets = {
        "character_body_2d_movement": '''extends CharacterBody2D

@export var speed: float = 300.0
@export var jump_velocity: float = -400.0

var gravity: float = ProjectSettings.get_setting("physics/2d/default_gravity")

func _physics_process(delta: float) -> void:
    if not is_on_floor():
        velocity.y += gravity * delta

    if Input.is_action_just_pressed("ui_accept") and is_on_floor():
        velocity.y = jump_velocity

    var direction := Input.get_axis("ui_left", "ui_right")
    if direction:
        velocity.x = direction * speed
    else:
        velocity.x = move_toward(velocity.x, 0, speed)

    move_and_slide()
''',
        "signal_connection": '''extends Node

signal health_changed(new_value: int)
signal died

var health: int = 100:
    set(value):
        health = clampi(value, 0, max_health)
        health_changed.emit(health)
        if health <= 0:
            died.emit()

var max_health: int = 100

func _ready() -> void:
    health_changed.connect(_on_health_changed)
    died.connect(_on_died)

func _on_health_changed(new_value: int) -> void:
    print("Health: ", new_value)

func _on_died() -> void:
    print("Entity died")
    queue_free()
''',
        "resource_custom": '''extends Resource
class_name ItemData

@export var id: String = ""
@export var name: String = ""
@export var description: String = ""
@export var icon: Texture2D
@export var stackable: bool = true
@export var max_stack: int = 99
@export var value: int = 0

@export_category("Combat")
@export var damage: float = 0.0
@export var defense: float = 0.0
''',
        "tween_animation": '''extends Node2D

func fade_in(duration: float = 0.5) -> void:
    modulate.a = 0.0
    var tween := create_tween()
    tween.tween_property(self, "modulate:a", 1.0, duration)

func fade_out(duration: float = 0.5) -> void:
    var tween := create_tween()
    tween.tween_property(self, "modulate:a", 0.0, duration)
    tween.tween_callback(queue_free)

func bounce() -> void:
    var tween := create_tween()
    tween.tween_property(self, "scale", Vector2(1.2, 0.8), 0.1)
    tween.tween_property(self, "scale", Vector2(0.9, 1.1), 0.1)
    tween.tween_property(self, "scale", Vector2.ONE, 0.1)

func shake(amount: float = 5.0, duration: float = 0.3) -> void:
    var original_pos := position
    var tween := create_tween()
    var steps := int(duration / 0.05)
    for i in range(steps):
        var offset := Vector2(randf_range(-amount, amount), randf_range(-amount, amount))
        tween.tween_property(self, "position", original_pos + offset, 0.05)
    tween.tween_property(self, "position", original_pos, 0.05)
''',
        "autoload_singleton": '''extends Node

var game_data: Dictionary = {}
var current_level: int = 0
var is_paused: bool = false

func _ready() -> void:
    process_mode = Node.PROCESS_MODE_ALWAYS

func _input(event: InputEvent) -> void:
    if event.is_action_pressed("pause"):
        toggle_pause()

func toggle_pause() -> void:
    is_paused = !is_paused
    get_tree().paused = is_paused

func change_level(level_path: String) -> void:
    get_tree().change_scene_to_file(level_path)

func quit_game() -> void:
    get_tree().quit()
''',
        "area2d_detection": '''extends Area2D

signal target_entered(body: Node2D)
signal target_exited(body: Node2D)

@export var detection_group: String = "player"
var targets_in_range: Array[Node2D] = []

func _ready() -> void:
    body_entered.connect(_on_body_entered)
    body_exited.connect(_on_body_exited)

func _on_body_entered(body: Node2D) -> void:
    if body.is_in_group(detection_group):
        targets_in_range.append(body)
        target_entered.emit(body)

func _on_body_exited(body: Node2D) -> void:
    if body.is_in_group(detection_group):
        targets_in_range.erase(body)
        target_exited.emit(body)

func get_nearest_target() -> Node2D:
    var nearest: Node2D = null
    var min_dist: float = INF
    for target in targets_in_range:
        var dist := global_position.distance_to(target.global_position)
        if dist < min_dist:
            min_dist = dist
            nearest = target
    return nearest
''',
        "tilemap_interaction": '''extends TileMap

func get_tile_at_position(world_pos: Vector2) -> Vector2i:
    return local_to_map(to_local(world_pos))

func is_tile_walkable(map_pos: Vector2i) -> bool:
    var tile_data := get_cell_tile_data(0, map_pos)
    if tile_data == null:
        return true
    return not tile_data.get_custom_data("is_wall")

func get_neighbors(map_pos: Vector2i) -> Array[Vector2i]:
    var neighbors: Array[Vector2i] = []
    var offsets := [Vector2i.UP, Vector2i.DOWN, Vector2i.LEFT, Vector2i.RIGHT]
    for offset in offsets:
        var neighbor := map_pos + offset
        if is_tile_walkable(neighbor):
            neighbors.append(neighbor)
    return neighbors
''',
    }

    for name, code in snippets.items():
        (snippets_dir / f"{name}.gd").write_text(code, encoding="utf-8")
    print(f"Created {len(snippets)} sample snippets")


def _create_gdscript_reference(docs_dir: Path) -> None:
    reference = """# GDScript 4.x Quick Reference

## Type System
- Static typing with `:` syntax: `var x: int = 5`
- Type inference with `:=`: `var x := 5`
- Return types: `func foo() -> int:`
- Typed arrays: `var arr: Array[int] = []`

## Common Types
- `int`, `float`, `bool`, `String`
- `Vector2`, `Vector2i`, `Vector3`, `Vector3i`
- `Color`, `Rect2`, `Transform2D`, `Transform3D`
- `Array`, `Dictionary`, `PackedByteArray`, `PackedStringArray`
- `NodePath`, `StringName`

## Signals
- Declaration: `signal my_signal(param: Type)`
- Emit: `my_signal.emit(value)`
- Connect: `my_signal.connect(callable)`
- Disconnect: `my_signal.disconnect(callable)`

## Export Annotations
- `@export var x: int = 0`
- `@export_range(0, 100) var health: int`
- `@export_file("*.tscn") var scene_path: String`
- `@export_enum("Walk", "Run", "Fly") var mode: int`
- `@export_category("Section")`
- `@export_group("Group")`

## Node Lifecycle
- `_ready()` - Called when node enters tree
- `_process(delta)` - Called every frame
- `_physics_process(delta)` - Called every physics frame
- `_input(event)` - Called on input events
- `_unhandled_input(event)` - Called on unhandled input

## Common Patterns
- Get node: `$NodeName` or `get_node("NodeName")`
- Unique node: `%NodeName` (must be marked as unique in editor)
- Groups: `add_to_group("name")`, `is_in_group("name")`
- Scene instantiation: `var scene := preload("res://scene.tscn").instantiate()`
- Timer: `await get_tree().create_timer(1.0).timeout`

## CharacterBody2D
- `move_and_slide()` - Move with collision
- `is_on_floor()`, `is_on_wall()`, `is_on_ceiling()`
- `velocity: Vector2` - Movement vector
- `get_wall_normal()` - Normal of wall collision

## Input
- `Input.is_action_pressed("action")`
- `Input.is_action_just_pressed("action")`
- `Input.get_axis("negative", "positive")` -> float
- `Input.get_vector("left", "right", "up", "down")` -> Vector2

## Tweens
- `var tween := create_tween()`
- `tween.tween_property(node, "property", value, duration)`
- `tween.tween_callback(callable)`
- `tween.set_parallel(true)` - Run tweens in parallel
- `tween.chain()` - Sequential after parallel

## File I/O
- `FileAccess.open(path, mode)` - Open file
- `FileAccess.file_exists(path)` - Check existence
- `JSON.stringify(data)` / `JSON.new().parse(string)`
- `ConfigFile` - INI-style config

## Resources
- `class_name MyResource extends Resource`
- `@export` properties become editable in inspector
- `ResourceLoader.load("res://path")` - Load resource
- `ResourceSaver.save(resource, "res://path")` - Save resource
"""
    (docs_dir / "gdscript_reference.md").write_text(reference, encoding="utf-8")
    print("Created GDScript reference document")


def main():
    parser = argparse.ArgumentParser(description="Index Godot knowledge into Qdrant")
    parser.add_argument(
        "--source",
        choices=["docs", "snippets", "all"],
        default="all",
        help="What to index",
    )
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path(__file__).parent / "data",
        help="Data directory",
    )
    args = parser.parse_args()

    qdrant = get_qdrant()
    openai_client = get_openai()
    ensure_collection(qdrant)

    total = 0

    if args.source in ("docs", "all"):
        docs = list(load_api_docs(args.data_dir))
        count = index_documents(qdrant, openai_client, docs)
        print(f"Indexed {count} document chunks")
        total += count

    if args.source in ("snippets", "all"):
        snippets = list(load_gdscript_snippets(args.data_dir))
        count = index_documents(qdrant, openai_client, snippets)
        print(f"Indexed {count} code snippets")
        total += count

    print(f"Total indexed: {total} points")


if __name__ == "__main__":
    main()
