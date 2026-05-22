extends Node
class_name UpgradeSystem

# Upgrade tree for {{theme}} idle game — {{upgrade_tiers}} tiers

@export var upgrade_tiers: int = {{upgrade_tiers}}
@export var base_cost: float = 10.0
@export var cost_growth: float = 1.15

var idle_engine: IdleEngine
var upgrades: Array = []  # [{ id, tier, level, base_yield, kind }]

signal upgrade_purchased(id: String, new_level: int)
signal upgrade_locked_changed(id: String, unlocked: bool)

func _ready() -> void:
	_build_default_tree()

func bind(engine: IdleEngine) -> void:
	idle_engine = engine

func _build_default_tree() -> void:
	upgrades.clear()
	for tier in range(upgrade_tiers):
		upgrades.append({
			"id": "click_t%d" % tier,
			"tier": tier,
			"level": 0,
			"base_yield": 0.5 * pow(2.0, tier),
			"kind": "click",
		})
		upgrades.append({
			"id": "auto_t%d" % tier,
			"tier": tier,
			"level": 0,
			"base_yield": 1.0 * pow(2.0, tier),
			"kind": "auto",
		})

func cost_for(upgrade: Dictionary) -> float:
	var tier_mult: float = pow(5.0, upgrade.tier)
	return base_cost * tier_mult * pow(cost_growth, upgrade.level)

func is_unlocked(upgrade: Dictionary) -> bool:
	if upgrade.tier == 0:
		return true
	for u in upgrades:
		if u.tier == upgrade.tier - 1 and u.kind == upgrade.kind and u.level >= 5:
			return true
	return false

func purchase(id: String) -> bool:
	if idle_engine == null:
		return false
	for u in upgrades:
		if u.id != id:
			continue
		if not is_unlocked(u):
			return false
		var cost: float = cost_for(u)
		if not idle_engine.spend(cost):
			return false
		u.level += 1
		_apply_effect(u)
		upgrade_purchased.emit(id, u.level)
		return true
	return false

func _apply_effect(u: Dictionary) -> void:
	match u.kind:
		"click":
			idle_engine.set_click_multiplier(idle_engine.click_multiplier + u.base_yield)
		"auto":
			idle_engine.add_per_second(u.base_yield)
