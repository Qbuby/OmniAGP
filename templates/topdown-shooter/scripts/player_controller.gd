extends CharacterBody2D
class_name PlayerController

# Twin-stick controller for {{theme}} top-down shooter

@export var move_speed: float = 280.0
@export var weapon_count: int = {{weapon_types}}
@export var fire_rate: float = 0.15
@export var max_health: int = 100

var current_weapon: int = 0
var current_health: int = max_health
var fire_cooldown: float = 0.0

signal weapon_switched(index: int)
signal projectile_fired(origin: Vector2, direction: Vector2, weapon: int)
signal health_changed(new_health: int)

func _ready() -> void:
	current_health = max_health

func _physics_process(delta: float) -> void:
	var move_dir := Vector2(
		Input.get_axis("move_left", "move_right"),
		Input.get_axis("move_up", "move_down")
	).limit_length(1.0)
	velocity = move_dir * move_speed
	move_and_slide()

	var aim_dir := _get_aim_direction()
	if aim_dir.length_squared() > 0.01:
		rotation = aim_dir.angle()

	fire_cooldown = max(0.0, fire_cooldown - delta)
	if Input.is_action_pressed("fire") and fire_cooldown <= 0.0:
		projectile_fired.emit(global_position, aim_dir, current_weapon)
		fire_cooldown = fire_rate

	if Input.is_action_just_pressed("switch_weapon"):
		current_weapon = (current_weapon + 1) % weapon_count
		weapon_switched.emit(current_weapon)

func _get_aim_direction() -> Vector2:
	var stick := Vector2(
		Input.get_axis("aim_left", "aim_right"),
		Input.get_axis("aim_up", "aim_down")
	)
	if stick.length_squared() > 0.04:
		return stick.normalized()
	return (get_global_mouse_position() - global_position).normalized()

func take_damage(amount: int) -> void:
	current_health = max(0, current_health - amount)
	health_changed.emit(current_health)
