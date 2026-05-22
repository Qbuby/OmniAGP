extends Node
class_name GridManager

# Grid logic for {{theme}} puzzle — {{grid_size}}x{{grid_size}} board

@export var grid_size: int = {{grid_size}}
@export var piece_types: int = 5
@export var min_match_length: int = 3

var grid: Array = []
var rng := RandomNumberGenerator.new()

signal pieces_matched(positions: Array, piece_type: int)
signal grid_refilled()

func _ready() -> void:
	rng.randomize()
	initialize_grid()

func initialize_grid() -> void:
	grid.clear()
	for x in range(grid_size):
		var col: Array = []
		for y in range(grid_size):
			col.append(rng.randi_range(0, piece_types - 1))
		grid.append(col)

func get_piece(x: int, y: int) -> int:
	if x < 0 or x >= grid_size or y < 0 or y >= grid_size:
		return -1
	return grid[x][y]

func swap(a: Vector2i, b: Vector2i) -> bool:
	var tmp: int = grid[a.x][a.y]
	grid[a.x][a.y] = grid[b.x][b.y]
	grid[b.x][b.y] = tmp
	var matches := find_matches()
	if matches.is_empty():
		# revert if no match formed
		grid[b.x][b.y] = grid[a.x][a.y]
		grid[a.x][a.y] = tmp
		return false
	_resolve_matches(matches)
	return true

func find_matches() -> Array:
	var matches: Array = []
	for x in range(grid_size):
		for y in range(grid_size):
			var t: int = grid[x][y]
			if t < 0:
				continue
			if x + min_match_length <= grid_size and _line_matches(t, x, y, 1, 0):
				matches.append({"type": t, "positions": _line_positions(x, y, 1, 0)})
			if y + min_match_length <= grid_size and _line_matches(t, x, y, 0, 1):
				matches.append({"type": t, "positions": _line_positions(x, y, 0, 1)})
	return matches

func _line_matches(t: int, x: int, y: int, dx: int, dy: int) -> bool:
	for i in range(min_match_length):
		if grid[x + dx * i][y + dy * i] != t:
			return false
	return true

func _line_positions(x: int, y: int, dx: int, dy: int) -> Array:
	var out: Array = []
	for i in range(min_match_length):
		out.append(Vector2i(x + dx * i, y + dy * i))
	return out

func _resolve_matches(matches: Array) -> void:
	for m in matches:
		for p in m.positions:
			grid[p.x][p.y] = -1
		pieces_matched.emit(m.positions, m.type)
	_refill()

func _refill() -> void:
	for x in range(grid_size):
		for y in range(grid_size):
			if grid[x][y] == -1:
				grid[x][y] = rng.randi_range(0, piece_types - 1)
	grid_refilled.emit()
