extends Node
class_name StoryManager

# Branching narrative for {{theme}} VN — {{ending_count}} possible endings

@export var ending_count: int = {{ending_count}}
@export var script_path: String = "res://story/script.json"

var nodes: Dictionary = {}
var current_node_id: String = ""
var flags: Dictionary = {}
var history: Array = []

signal node_entered(node_id: String, node_data: Dictionary)
signal ending_reached(ending_id: String)

func load_script(path: String = "") -> bool:
	var p: String = path if path != "" else script_path
	if not FileAccess.file_exists(p):
		push_warning("Story script not found: " + p)
		return false
	var f := FileAccess.open(p, FileAccess.READ)
	var data = JSON.parse_string(f.get_as_text())
	f.close()
	if typeof(data) != TYPE_DICTIONARY:
		return false
	nodes = data.get("nodes", {})
	return true

func start(start_id: String = "start") -> void:
	flags.clear()
	history.clear()
	goto(start_id)

func goto(node_id: String) -> void:
	if not nodes.has(node_id):
		push_warning("Unknown story node: " + node_id)
		return
	current_node_id = node_id
	history.append(node_id)
	var node: Dictionary = nodes[node_id]
	for flag in node.get("set_flags", {}).keys():
		flags[flag] = node.set_flags[flag]
	if node.get("type", "") == "ending":
		ending_reached.emit(node.get("ending_id", node_id))
		return
	node_entered.emit(node_id, node)

func choose(choice_id: String) -> void:
	var node: Dictionary = nodes.get(current_node_id, {})
	for choice in node.get("choices", []):
		if choice.get("id", "") == choice_id and _check_requirements(choice.get("requires", {})):
			goto(choice.get("next", ""))
			return

func _check_requirements(req: Dictionary) -> bool:
	for k in req.keys():
		if flags.get(k) != req[k]:
			return false
	return true
