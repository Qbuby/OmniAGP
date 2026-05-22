extends CharacterBody2D
class_name PlayerController

# Auto-generated for {{theme}} platformer (difficulty: {{difficulty}})

@export var speed: float = 300.0
@export var jump_velocity: float = -450.0
@export var gravity: float = 980.0
@export var has_double_jump: bool = {{has_double_jump}}
@export var max_health: int = 3

var jumps_remaining: int = 1
var current_health: int = max_health

signal health_changed(new_health: int)
signal player_died()

func _ready() -> void:
	current_health = max_health
	jumps_remaining = 2 if has_double_jump else 1

func _physics_process(delta: float) -> void:
	if not is_on_floor():
		velocity.y += gravity * delta
	else:
		jumps_remaining = 2 if has_double_jump else 1

	if Input.is_action_just_pressed("jump") and jumps_remaining > 0:
		velocity.y = jump_velocity
		jumps_remaining -= 1

	var direction := Input.get_axis("move_left", "move_right")
	if direction != 0.0:
		velocity.x = direction * speed
	else:
		velocity.x = move_toward(velocity.x, 0.0, speed)

	move_and_slide()

func take_damage(amount: int) -> void:
	current_health = max(0, current_health - amount)
	health_changed.emit(current_health)
	if current_health == 0:
		player_died.emit()
