extends Node
class_name LevelProgression

# Difficulty scaling across {{level_count}} levels of {{theme}} puzzle

@export var level_count: int = {{level_count}}
@export var base_target_score: int = 1000
@export var base_time_limit: float = 60.0
@export var base_moves: int = 25

var current_level: int = 0
var current_score: int = 0

signal level_started(index: int, config: Dictionary)
signal level_completed(index: int, score: int, stars: int)
signal campaign_completed()

func get_level_config(index: int) -> Dictionary:
	var t: float = float(index) / float(max(1, level_count - 1))
	var difficulty_factor: float = lerp(1.0, 3.0, t)
	return {
		"index": index,
		"target_score": int(base_target_score * difficulty_factor),
		"time_limit": max(15.0, base_time_limit - t * 30.0),
		"move_limit": int(base_moves - t * 10.0),
		"piece_types": min(7, 4 + int(t * 3)),
		"obstacles": int(t * 5),
	}

func start_level(index: int) -> void:
	current_level = clamp(index, 0, level_count - 1)
	current_score = 0
	level_started.emit(current_level, get_level_config(current_level))

func add_score(points: int) -> void:
	current_score += points

func complete_level() -> void:
	var cfg := get_level_config(current_level)
	var stars: int = 1
	if current_score >= cfg.target_score * 1.25:
		stars = 2
	if current_score >= cfg.target_score * 1.5:
		stars = 3
	level_completed.emit(current_level, current_score, stars)
	if current_level >= level_count - 1:
		campaign_completed.emit()
