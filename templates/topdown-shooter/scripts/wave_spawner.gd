extends Node
class_name WaveSpawner

# Wave manager for {{theme}} shooter — {{wave_count}} total waves

@export var wave_count: int = {{wave_count}}
@export var enemy_scenes: Array[PackedScene] = []
@export var spawn_radius: float = 600.0
@export var time_between_waves: float = 3.0
@export var base_enemies_per_wave: int = 5

var current_wave: int = 0
var alive_enemies: int = 0
var rng := RandomNumberGenerator.new()

signal wave_started(index: int, total_enemies: int)
signal wave_cleared(index: int)
signal all_waves_cleared()

func _ready() -> void:
	rng.randomize()

func start() -> void:
	current_wave = 0
	_spawn_next_wave()

func _spawn_next_wave() -> void:
	if current_wave >= wave_count:
		all_waves_cleared.emit()
		return
	var count: int = base_enemies_per_wave + current_wave * 2
	alive_enemies = count
	wave_started.emit(current_wave, count)
	for i in range(count):
		_spawn_enemy()

func _spawn_enemy() -> void:
	if enemy_scenes.is_empty():
		return
	var scene: PackedScene = enemy_scenes[rng.randi() % enemy_scenes.size()]
	var enemy: Node = scene.instantiate()
	var angle: float = rng.randf() * TAU
	if enemy is Node2D:
		(enemy as Node2D).position = Vector2(cos(angle), sin(angle)) * spawn_radius
	if enemy.has_signal("died"):
		enemy.died.connect(_on_enemy_died)
	add_child(enemy)

func _on_enemy_died() -> void:
	alive_enemies = max(0, alive_enemies - 1)
	if alive_enemies == 0:
		wave_cleared.emit(current_wave)
		current_wave += 1
		await get_tree().create_timer(time_between_waves).timeout
		_spawn_next_wave()
