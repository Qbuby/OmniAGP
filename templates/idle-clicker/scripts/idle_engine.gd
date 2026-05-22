extends Node
class_name IdleEngine

# Core idle loop for {{theme}} idle/clicker

@export var base_click_value: float = 1.0
@export var offline_progress_cap_seconds: float = 86400.0  # 24h

var currency: float = 0.0
var per_second: float = 0.0
var click_multiplier: float = 1.0
var prestige_multiplier: float = 1.0

signal currency_changed(new_total: float)
signal clicked(value: float)

func _process(delta: float) -> void:
	if per_second > 0.0:
		_add(per_second * delta)

func click() -> void:
	var value: float = base_click_value * click_multiplier * prestige_multiplier
	_add(value)
	clicked.emit(value)

func add_per_second(amount: float) -> void:
	per_second += amount

func set_click_multiplier(value: float) -> void:
	click_multiplier = max(1.0, value)

func set_prestige_multiplier(value: float) -> void:
	prestige_multiplier = max(1.0, value)

func apply_offline_progress(seconds_away: float) -> float:
	var capped: float = min(seconds_away, offline_progress_cap_seconds)
	var earned: float = per_second * capped * 0.5  # 50% efficiency offline
	_add(earned)
	return earned

func spend(amount: float) -> bool:
	if currency < amount:
		return false
	_add(-amount)
	return true

func _add(amount: float) -> void:
	currency = max(0.0, currency + amount)
	currency_changed.emit(currency)

func to_save_dict() -> Dictionary:
	return {
		"currency": currency,
		"per_second": per_second,
		"click_multiplier": click_multiplier,
		"prestige_multiplier": prestige_multiplier,
		"timestamp": Time.get_unix_time_from_system(),
	}

func from_save_dict(data: Dictionary) -> void:
	currency = data.get("currency", 0.0)
	per_second = data.get("per_second", 0.0)
	click_multiplier = data.get("click_multiplier", 1.0)
	prestige_multiplier = data.get("prestige_multiplier", 1.0)
	var ts: float = data.get("timestamp", Time.get_unix_time_from_system())
	apply_offline_progress(Time.get_unix_time_from_system() - ts)
