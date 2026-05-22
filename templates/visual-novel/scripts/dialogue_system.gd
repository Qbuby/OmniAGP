extends Node
class_name DialogueSystem

# Dialogue and choice system for {{theme}} visual novel
# Supports {{character_count}} characters

@export var text_speed: float = 40.0  # chars per second
@export var auto_advance: bool = false

var current_line: Dictionary = {}
var displayed_chars: float = 0.0
var is_typing: bool = false

signal line_started(speaker: String, text: String)
signal line_finished()
signal choice_presented(choices: Array)
signal choice_selected(index: int, choice_id: String)

func show_line(line: Dictionary) -> void:
	current_line = line
	displayed_chars = 0.0
	is_typing = true
	line_started.emit(line.get("speaker", ""), line.get("text", ""))

func _process(delta: float) -> void:
	if not is_typing:
		return
	var full_text: String = current_line.get("text", "")
	displayed_chars += text_speed * delta
	if displayed_chars >= float(full_text.length()):
		displayed_chars = float(full_text.length())
		is_typing = false
		line_finished.emit()

func get_visible_text() -> String:
	var full_text: String = current_line.get("text", "")
	return full_text.substr(0, int(displayed_chars))

func skip_to_end() -> void:
	if not is_typing:
		return
	is_typing = false
	displayed_chars = float(current_line.get("text", "").length())
	line_finished.emit()

func present_choices(choices: Array) -> void:
	choice_presented.emit(choices)

func select_choice(index: int, choices: Array) -> void:
	if index < 0 or index >= choices.size():
		return
	var choice: Dictionary = choices[index]
	choice_selected.emit(index, choice.get("id", ""))
