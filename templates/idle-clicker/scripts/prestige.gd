extends Node
class_name Prestige

# Prestige/rebirth for {{theme}} idle game

@export var enabled: bool = {{has_prestige}}
@export var prestige_threshold: float = 1.0e6
@export var points_divisor: float = 1.0e4

var idle_engine: IdleEngine
var upgrade_system: UpgradeSystem
var prestige_points: int = 0
var prestige_count: int = 0

signal prestige_available(points_to_gain: int)
signal prestige_performed(new_count: int, points_earned: int)

func bind(engine: IdleEngine, upgrades: UpgradeSystem) -> void:
	idle_engine = engine
	upgrade_system = upgrades
	if engine != null:
		engine.currency_changed.connect(_on_currency_changed)

func points_for(currency: float) -> int:
	if currency < prestige_threshold:
		return 0
	return int(sqrt(currency / points_divisor))

func can_prestige() -> bool:
	if not enabled or idle_engine == null:
		return false
	return idle_engine.currency >= prestige_threshold

func perform() -> bool:
	if not can_prestige():
		return false
	var earned: int = points_for(idle_engine.currency)
	prestige_points += earned
	prestige_count += 1
	_reset_run()
	idle_engine.set_prestige_multiplier(1.0 + 0.1 * float(prestige_points))
	prestige_performed.emit(prestige_count, earned)
	return true

func _reset_run() -> void:
	idle_engine.currency = 0.0
	idle_engine.per_second = 0.0
	idle_engine.click_multiplier = 1.0
	idle_engine.currency_changed.emit(0.0)
	if upgrade_system != null:
		for u in upgrade_system.upgrades:
			u.level = 0

func _on_currency_changed(total: float) -> void:
	if enabled and total >= prestige_threshold:
		prestige_available.emit(points_for(total))
