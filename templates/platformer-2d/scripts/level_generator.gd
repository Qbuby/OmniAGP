extends Node
class_name LevelGenerator

# Procedural level generator for {{theme}} platformer
# Generates {{level_count}} levels with {{difficulty}} difficulty curve

@export var level_count: int = {{level_count}}
@export var difficulty: String = "{{difficulty}}"
@export var theme: String = "{{theme}}"
@export var tile_size: int = 32
@export var min_platform_width: int = 3
@export var max_platform_width: int = 8

var rng := RandomNumberGenerator.new()
var generated_levels: Array = []

signal level_generated(index: int, data: Dictionary)

func _ready() -> void:
	rng.randomize()

func generate_all() -> void:
	generated_levels.clear()
	for i in range(level_count):
		var data := generate_level(i)
		generated_levels.append(data)
		level_generated.emit(i, data)

func generate_level(index: int) -> Dictionary:
	var difficulty_factor: float = _difficulty_factor() * (1.0 + float(index) / float(max(1, level_count)))
	var platform_count: int = int(8 + difficulty_factor * 4)
	var enemy_count: int = int(2 + difficulty_factor * 3)
	var platforms: Array = []
	var x: int = 0
	for _i in range(platform_count):
		var width := rng.randi_range(min_platform_width, max_platform_width)
		var y := rng.randi_range(-3, 3) * tile_size
		platforms.append({"x": x, "y": y, "width": width})
		x += (width + rng.randi_range(2, 5)) * tile_size
	return {
		"index": index,
		"theme": theme,
		"platforms": platforms,
		"enemy_count": enemy_count,
		"has_boss": (index + 1) % 5 == 0,
	}

func _difficulty_factor() -> float:
	match difficulty:
		"easy": return 0.5
		"hard": return 1.5
		_: return 1.0
