use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub code: String,
    pub parameters: Vec<String>,
}

pub struct TemplateLibrary {
    templates: Vec<Template>,
}

impl TemplateLibrary {
    pub fn load_builtin() -> Self {
        Self {
            templates: builtin_templates(),
        }
    }

    pub fn find_match(&self, description: &str) -> Option<String> {
        let desc_lower = description.to_lowercase();
        let mut best_match: Option<(&Template, usize)> = None;

        for tmpl in &self.templates {
            let score = tmpl
                .keywords
                .iter()
                .filter(|kw| desc_lower.contains(kw.as_str()))
                .count();
            if score > 0 {
                if best_match.map_or(true, |(_, s)| score > s) {
                    best_match = Some((tmpl, score));
                }
            }
        }

        best_match.map(|(tmpl, _)| tmpl.code.clone())
    }

    pub fn find_match_with_params(
        &self,
        description: &str,
        params: &HashMap<String, String>,
    ) -> Option<String> {
        let code = self.find_match(description)?;
        let mut result = code;
        for (key, value) in params {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        Some(result)
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }
}

fn builtin_templates() -> Vec<Template> {
    vec![
        Template {
            name: "PlayerController2D".into(),
            description: "2D character movement with CharacterBody2D".into(),
            keywords: vec!["player".into(), "movement".into(), "character".into(), "controller".into(), "2d".into(), "walk".into(), "run".into()],
            parameters: vec!["class_name".into(), "speed".into(), "jump_force".into()],
            code: r#"extends CharacterBody2D

@export var speed: float = {{speed}}
@export var jump_force: float = {{jump_force}}
@export var gravity: float = 980.0

func _physics_process(delta: float) -> void:
    if not is_on_floor():
        velocity.y += gravity * delta

    if Input.is_action_just_pressed("jump") and is_on_floor():
        velocity.y = -jump_force

    var direction := Input.get_axis("move_left", "move_right")
    velocity.x = direction * speed

    move_and_slide()
"#.into(),
        },
        Template {
            name: "EnemyFSM".into(),
            description: "Enemy AI with finite state machine (patrol/chase/attack)".into(),
            keywords: vec!["enemy".into(), "ai".into(), "state machine".into(), "fsm".into(), "patrol".into(), "chase".into(), "attack".into()],
            parameters: vec!["class_name".into(), "patrol_speed".into(), "chase_speed".into(), "detection_range".into()],
            code: r#"extends CharacterBody2D

enum State { PATROL, CHASE, ATTACK }

@export var patrol_speed: float = {{patrol_speed}}
@export var chase_speed: float = {{chase_speed}}
@export var detection_range: float = {{detection_range}}
@export var attack_range: float = 50.0

var current_state: State = State.PATROL
var patrol_direction: float = 1.0
var target: Node2D = null

func _physics_process(delta: float) -> void:
    match current_state:
        State.PATROL:
            _patrol(delta)
        State.CHASE:
            _chase(delta)
        State.ATTACK:
            _attack()

func _patrol(_delta: float) -> void:
    velocity.x = patrol_direction * patrol_speed
    move_and_slide()
    if is_on_wall():
        patrol_direction *= -1.0
    _check_player_in_range()

func _chase(_delta: float) -> void:
    if target == null:
        current_state = State.PATROL
        return
    var dir := sign(target.global_position.x - global_position.x)
    velocity.x = dir * chase_speed
    move_and_slide()
    if global_position.distance_to(target.global_position) < attack_range:
        current_state = State.ATTACK
    elif global_position.distance_to(target.global_position) > detection_range * 1.5:
        current_state = State.PATROL
        target = null

func _attack() -> void:
    velocity.x = 0.0
    # Attack logic here
    current_state = State.CHASE

func _check_player_in_range() -> void:
    var players := get_tree().get_nodes_in_group("player")
    for player in players:
        if global_position.distance_to(player.global_position) < detection_range:
            target = player
            current_state = State.CHASE
            break
"#.into(),
        },
        Template {
            name: "InventorySystem".into(),
            description: "Item inventory with add/remove/stack support".into(),
            keywords: vec!["inventory".into(), "item".into(), "backpack".into(), "bag".into(), "slot".into(), "pickup".into()],
            parameters: vec!["class_name".into(), "max_slots".into()],
            code: r#"extends Node

signal inventory_changed
signal item_added(item_id: String, quantity: int)
signal item_removed(item_id: String, quantity: int)

@export var max_slots: int = {{max_slots}}

var items: Dictionary = {}

func add_item(item_id: String, quantity: int = 1) -> bool:
    if items.has(item_id):
        items[item_id] += quantity
    elif items.size() < max_slots:
        items[item_id] = quantity
    else:
        return false
    item_added.emit(item_id, quantity)
    inventory_changed.emit()
    return true

func remove_item(item_id: String, quantity: int = 1) -> bool:
    if not items.has(item_id):
        return false
    items[item_id] -= quantity
    if items[item_id] <= 0:
        items.erase(item_id)
    item_removed.emit(item_id, quantity)
    inventory_changed.emit()
    return true

func has_item(item_id: String, quantity: int = 1) -> bool:
    return items.get(item_id, 0) >= quantity

func get_quantity(item_id: String) -> int:
    return items.get(item_id, 0)

func get_all_items() -> Dictionary:
    return items.duplicate()

func clear() -> void:
    items.clear()
    inventory_changed.emit()
"#.into(),
        },
        Template {
            name: "SaveLoadSystem".into(),
            description: "Save/load game state with JSON serialization".into(),
            keywords: vec!["save".into(), "load".into(), "persist".into(), "serialize".into(), "json".into(), "file".into()],
            parameters: vec!["class_name".into(), "save_path".into()],
            code: r#"extends Node

const SAVE_PATH: String = "{{save_path}}"

func save_game(data: Dictionary) -> void:
    var file := FileAccess.open(SAVE_PATH, FileAccess.WRITE)
    if file == null:
        push_error("Failed to open save file: " + SAVE_PATH)
        return
    var json_string := JSON.stringify(data, "\t")
    file.store_string(json_string)
    file.close()

func load_game() -> Dictionary:
    if not FileAccess.file_exists(SAVE_PATH):
        return {}
    var file := FileAccess.open(SAVE_PATH, FileAccess.READ)
    if file == null:
        return {}
    var json_string := file.get_as_text()
    file.close()
    var json := JSON.new()
    var error := json.parse(json_string)
    if error != OK:
        push_error("Failed to parse save file")
        return {}
    return json.data

func delete_save() -> void:
    if FileAccess.file_exists(SAVE_PATH):
        DirAccess.remove_absolute(SAVE_PATH)

func has_save() -> bool:
    return FileAccess.file_exists(SAVE_PATH)
"#.into(),
        },
        Template {
            name: "MainMenu".into(),
            description: "Main menu with start, options, and quit buttons".into(),
            keywords: vec!["menu".into(), "main menu".into(), "start".into(), "title".into(), "ui".into()],
            parameters: vec!["class_name".into(), "game_scene_path".into()],
            code: r#"extends Control

@export var game_scene: PackedScene

func _ready() -> void:
    %StartButton.pressed.connect(_on_start_pressed)
    %OptionsButton.pressed.connect(_on_options_pressed)
    %QuitButton.pressed.connect(_on_quit_pressed)

func _on_start_pressed() -> void:
    get_tree().change_scene_to_packed(game_scene)

func _on_options_pressed() -> void:
    # Open options menu
    pass

func _on_quit_pressed() -> void:
    get_tree().quit()
"#.into(),
        },
        Template {
            name: "PauseMenu".into(),
            description: "Pause menu overlay with resume and quit".into(),
            keywords: vec!["pause".into(), "pause menu".into(), "resume".into(), "escape".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Control

func _ready() -> void:
    visible = false
    process_mode = Node.PROCESS_MODE_ALWAYS
    %ResumeButton.pressed.connect(_on_resume_pressed)
    %QuitButton.pressed.connect(_on_quit_pressed)

func _input(event: InputEvent) -> void:
    if event.is_action_pressed("pause"):
        toggle_pause()

func toggle_pause() -> void:
    visible = !visible
    get_tree().paused = visible

func _on_resume_pressed() -> void:
    toggle_pause()

func _on_quit_pressed() -> void:
    get_tree().paused = false
    get_tree().change_scene_to_file("res://scenes/main_menu.tscn")
"#.into(),
        },
        Template {
            name: "HealthBar".into(),
            description: "Health bar UI component with damage/heal".into(),
            keywords: vec!["health".into(), "hp".into(), "bar".into(), "damage".into(), "heal".into(), "life".into()],
            parameters: vec!["class_name".into(), "max_health".into()],
            code: r#"extends Control

signal health_changed(new_health: float, max_health: float)
signal died

@export var max_health: float = {{max_health}}
@onready var progress_bar: ProgressBar = %ProgressBar

var current_health: float

func _ready() -> void:
    current_health = max_health
    _update_bar()

func take_damage(amount: float) -> void:
    current_health = max(0.0, current_health - amount)
    health_changed.emit(current_health, max_health)
    _update_bar()
    if current_health <= 0.0:
        died.emit()

func heal(amount: float) -> void:
    current_health = min(max_health, current_health + amount)
    health_changed.emit(current_health, max_health)
    _update_bar()

func _update_bar() -> void:
    if progress_bar:
        progress_bar.max_value = max_health
        progress_bar.value = current_health
"#.into(),
        },
        Template {
            name: "Projectile".into(),
            description: "Projectile that moves in a direction and deals damage".into(),
            keywords: vec!["projectile".into(), "bullet".into(), "shoot".into(), "missile".into(), "fire".into()],
            parameters: vec!["class_name".into(), "speed".into(), "damage".into(), "lifetime".into()],
            code: r#"extends Area2D

@export var speed: float = {{speed}}
@export var damage: float = {{damage}}
@export var lifetime: float = {{lifetime}}

var direction: Vector2 = Vector2.RIGHT

func _ready() -> void:
    body_entered.connect(_on_body_entered)
    var timer := get_tree().create_timer(lifetime)
    timer.timeout.connect(queue_free)

func _physics_process(delta: float) -> void:
    position += direction * speed * delta

func _on_body_entered(body: Node2D) -> void:
    if body.has_method("take_damage"):
        body.take_damage(damage)
    queue_free()

func set_direction(dir: Vector2) -> void:
    direction = dir.normalized()
    rotation = direction.angle()
"#.into(),
        },
        Template {
            name: "Collectible".into(),
            description: "Collectible item with pickup effect".into(),
            keywords: vec!["collectible".into(), "coin".into(), "gem".into(), "pickup".into(), "collect".into()],
            parameters: vec!["class_name".into(), "value".into()],
            code: r#"extends Area2D

signal collected(value: int)

@export var value: int = {{value}}
@export var bob_amplitude: float = 4.0
@export var bob_speed: float = 3.0

var initial_y: float

func _ready() -> void:
    initial_y = position.y
    body_entered.connect(_on_body_entered)

func _process(delta: float) -> void:
    position.y = initial_y + sin(Time.get_ticks_msec() * 0.001 * bob_speed) * bob_amplitude

func _on_body_entered(body: Node2D) -> void:
    if body.is_in_group("player"):
        collected.emit(value)
        queue_free()
"#.into(),
        },
        Template {
            name: "LevelTransition".into(),
            description: "Area-based level/scene transition trigger".into(),
            keywords: vec!["level".into(), "transition".into(), "door".into(), "portal".into(), "scene change".into(), "warp".into()],
            parameters: vec!["class_name".into(), "target_scene".into()],
            code: r#"extends Area2D

@export_file("*.tscn") var target_scene: String
@export var spawn_point: String = "SpawnPoint"

func _ready() -> void:
    body_entered.connect(_on_body_entered)

func _on_body_entered(body: Node2D) -> void:
    if body.is_in_group("player"):
        _transition()

func _transition() -> void:
    get_tree().change_scene_to_file(target_scene)
"#.into(),
        },
            Template {
            name: "DialogueSystem".into(),
            description: "Branching dialogue tree system".into(),
            keywords: vec!["dialogue".into(), "dialog".into(), "conversation".into(), "npc".into(), "talk".into(), "text".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Control

signal dialogue_started
signal dialogue_ended

@onready var label: RichTextLabel = %DialogueLabel
@onready var choices_container: VBoxContainer = %ChoicesContainer

var dialogue_data: Dictionary = {}
var current_node: String = ""

func start_dialogue(data: Dictionary, start_node: String = "start") -> void:
    dialogue_data = data
    current_node = start_node
    visible = true
    dialogue_started.emit()
    _show_current_node()

func _show_current_node() -> void:
    if not dialogue_data.has(current_node):
        end_dialogue()
        return
    var node_data: Dictionary = dialogue_data[current_node]
    label.text = node_data.get("text", "")
    _clear_choices()
    var choices: Array = node_data.get("choices", [])
    if choices.is_empty():
        # Auto-advance on click
        label.gui_input.connect(_on_label_clicked.bind(node_data.get("next", "")))
    else:
        for choice in choices:
            _add_choice_button(choice["text"], choice["next"])

func _add_choice_button(text: String, next_node: String) -> void:
    var btn := Button.new()
    btn.text = text
    btn.pressed.connect(_on_choice_selected.bind(next_node))
    choices_container.add_child(btn)

func _on_choice_selected(next_node: String) -> void:
    current_node = next_node
    _show_current_node()

func _on_label_clicked(event: InputEvent, next_node: String) -> void:
    if event is InputEventMouseButton and event.pressed:
        label.gui_input.disconnect(_on_label_clicked)
        if next_node.is_empty():
            end_dialogue()
        else:
            current_node = next_node
            _show_current_node()

func _clear_choices() -> void:
    for child in choices_container.get_children():
        child.queue_free()

func end_dialogue() -> void:
    visible = false
    dialogue_ended.emit()
"#.into(),
        },
        Template {
            name: "CameraFollow".into(),
            description: "Smooth camera follow with screen shake".into(),
            keywords: vec!["camera".into(), "follow".into(), "shake".into(), "screen shake".into(), "smooth".into()],
            parameters: vec!["class_name".into(), "smoothing_speed".into()],
            code: r#"extends Camera2D

@export var target: Node2D
@export var smoothing_speed: float = {{smoothing_speed}}
@export var look_ahead: float = 50.0

var shake_amount: float = 0.0
var shake_decay: float = 5.0

func _process(delta: float) -> void:
    if target:
        var target_pos := target.global_position
        target_pos.x += target.velocity.x * 0.1 if target.has_method("get") else 0.0
        global_position = global_position.lerp(target_pos, smoothing_speed * delta)

    if shake_amount > 0.0:
        offset = Vector2(
            randf_range(-shake_amount, shake_amount),
            randf_range(-shake_amount, shake_amount)
        )
        shake_amount = lerp(shake_amount, 0.0, shake_decay * delta)
    else:
        offset = Vector2.ZERO

func shake(amount: float) -> void:
    shake_amount = max(shake_amount, amount)
"#.into(),
        },
        Template {
            name: "AudioManager".into(),
            description: "Global audio manager for music and SFX".into(),
            keywords: vec!["audio".into(), "sound".into(), "music".into(), "sfx".into(), "play".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node

var music_player: AudioStreamPlayer
var sfx_players: Array[AudioStreamPlayer] = []
var sfx_pool_size: int = 8

func _ready() -> void:
    music_player = AudioStreamPlayer.new()
    music_player.bus = "Music"
    add_child(music_player)
    for i in range(sfx_pool_size):
        var player := AudioStreamPlayer.new()
        player.bus = "SFX"
        add_child(player)
        sfx_players.append(player)

func play_music(stream: AudioStream, fade_in: float = 0.5) -> void:
    music_player.stream = stream
    music_player.volume_db = -80.0
    music_player.play()
    var tween := create_tween()
    tween.tween_property(music_player, "volume_db", 0.0, fade_in)

func stop_music(fade_out: float = 0.5) -> void:
    var tween := create_tween()
    tween.tween_property(music_player, "volume_db", -80.0, fade_out)
    tween.tween_callback(music_player.stop)

func play_sfx(stream: AudioStream) -> void:
    for player in sfx_players:
        if not player.playing:
            player.stream = stream
            player.play()
            return
    sfx_players[0].stream = stream
    sfx_players[0].play()

func set_music_volume(linear: float) -> void:
    AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Music"), linear_to_db(linear))

func set_sfx_volume(linear: float) -> void:
    AudioServer.set_bus_volume_db(AudioServer.get_bus_index("SFX"), linear_to_db(linear))
"#.into(),
        },
        Template {
            name: "ScoreSystem".into(),
            description: "Score tracking with timer and combo".into(),
            keywords: vec!["score".into(), "point".into(), "timer".into(), "combo".into(), "counter".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node

signal score_changed(new_score: int)
signal combo_changed(new_combo: int)
signal time_updated(seconds: float)

var score: int = 0
var combo: int = 0
var combo_timer: float = 0.0
var combo_timeout: float = 2.0
var elapsed_time: float = 0.0
var running: bool = false

func _process(delta: float) -> void:
    if running:
        elapsed_time += delta
        time_updated.emit(elapsed_time)
    if combo > 0:
        combo_timer -= delta
        if combo_timer <= 0.0:
            combo = 0
            combo_changed.emit(combo)

func add_score(points: int) -> void:
    combo += 1
    combo_timer = combo_timeout
    score += points * combo
    score_changed.emit(score)
    combo_changed.emit(combo)

func start() -> void:
    running = true

func stop() -> void:
    running = false

func reset() -> void:
    score = 0
    combo = 0
    elapsed_time = 0.0
    running = false
    score_changed.emit(score)
    combo_changed.emit(combo)
"#.into(),
        },
        Template {
            name: "SceneTransitionManager".into(),
            description: "Scene transition with fade effect".into(),
            keywords: vec!["scene".into(), "transition".into(), "fade".into(), "switch".into(), "change scene".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends CanvasLayer

@onready var color_rect: ColorRect = %FadeRect

var is_transitioning: bool = false

func change_scene(scene_path: String, duration: float = 0.5) -> void:
    if is_transitioning:
        return
    is_transitioning = true
    var tween := create_tween()
    tween.tween_property(color_rect, "color:a", 1.0, duration)
    tween.tween_callback(get_tree().change_scene_to_file.bind(scene_path))
    tween.tween_property(color_rect, "color:a", 0.0, duration)
    tween.tween_callback(func(): is_transitioning = false)

func fade_in(duration: float = 0.5) -> void:
    color_rect.color.a = 1.0
    var tween := create_tween()
    tween.tween_property(color_rect, "color:a", 0.0, duration)

func fade_out(duration: float = 0.5) -> void:
    var tween := create_tween()
    tween.tween_property(color_rect, "color:a", 1.0, duration)
"#.into(),
        },
        Template {
            name: "PlatformerCharacter".into(),
            description: "Full platformer character with jump, dash, wall jump".into(),
            keywords: vec!["platformer".into(), "jump".into(), "dash".into(), "wall jump".into(), "coyote".into()],
            parameters: vec!["class_name".into(), "speed".into(), "jump_force".into()],
            code: r#"extends CharacterBody2D

@export var speed: float = {{speed}}
@export var jump_force: float = {{jump_force}}
@export var dash_speed: float = 600.0
@export var gravity: float = 980.0
@export var coyote_time: float = 0.1
@export var wall_jump_force: Vector2 = Vector2(300, -400)

var coyote_timer: float = 0.0
var is_dashing: bool = false
var dash_timer: float = 0.0
var dash_duration: float = 0.15
var can_dash: bool = true

func _physics_process(delta: float) -> void:
    if is_dashing:
        _handle_dash(delta)
        return

    if not is_on_floor():
        velocity.y += gravity * delta
        coyote_timer -= delta
    else:
        coyote_timer = coyote_time
        can_dash = true

    if Input.is_action_just_pressed("jump"):
        if is_on_floor() or coyote_timer > 0.0:
            velocity.y = -jump_force
            coyote_timer = 0.0
        elif is_on_wall():
            var wall_dir := get_wall_normal().x
            velocity = Vector2(wall_dir * wall_jump_force.x, wall_jump_force.y)

    if Input.is_action_just_pressed("dash") and can_dash:
        _start_dash()

    var direction := Input.get_axis("move_left", "move_right")
    velocity.x = direction * speed
    move_and_slide()

func _start_dash() -> void:
    is_dashing = true
    can_dash = false
    dash_timer = dash_duration
    var dir := Input.get_axis("move_left", "move_right")
    velocity = Vector2(sign(dir) * dash_speed if dir != 0.0 else dash_speed, 0.0)

func _handle_dash(delta: float) -> void:
    dash_timer -= delta
    if dash_timer <= 0.0:
        is_dashing = false
    move_and_slide()
"#.into(),
        },
        Template {
            name: "TilemapGenerator".into(),
            description: "Procedural tilemap generation".into(),
            keywords: vec!["tilemap".into(), "procedural".into(), "generate".into(), "random".into(), "terrain".into(), "map".into()],
            parameters: vec!["class_name".into(), "width".into(), "height".into()],
            code: r#"extends TileMap

@export var map_width: int = {{width}}
@export var map_height: int = {{height}}
@export var fill_percent: float = 0.45
@export var smooth_iterations: int = 5

var grid: Array[Array] = []

func _ready() -> void:
    generate()

func generate() -> void:
    _initialize_grid()
    for i in range(smooth_iterations):
        _smooth()
    _apply_to_tilemap()

func _initialize_grid() -> void:
    grid.clear()
    for x in range(map_width):
        var col: Array = []
        for y in range(map_height):
            if x == 0 or x == map_width - 1 or y == 0 or y == map_height - 1:
                col.append(1)
            else:
                col.append(1 if randf() < fill_percent else 0)
        grid.append(col)

func _smooth() -> void:
    var new_grid: Array[Array] = []
    for x in range(map_width):
        var col: Array = []
        for y in range(map_height):
            var neighbors := _count_neighbors(x, y)
            if neighbors > 4:
                col.append(1)
            elif neighbors < 4:
                col.append(0)
            else:
                col.append(grid[x][y])
        new_grid.append(col)
    grid = new_grid

func _count_neighbors(cx: int, cy: int) -> int:
    var count := 0
    for x in range(cx - 1, cx + 2):
        for y in range(cy - 1, cy + 2):
            if x == cx and y == cy:
                continue
            if x < 0 or x >= map_width or y < 0 or y >= map_height:
                count += 1
            elif grid[x][y] == 1:
                count += 1
    return count

func _apply_to_tilemap() -> void:
    clear()
    for x in range(map_width):
        for y in range(map_height):
            if grid[x][y] == 1:
                set_cell(0, Vector2i(x, y), 0, Vector2i(0, 0))
"#.into(),
        },
        Template {
            name: "PhysicsPuzzle".into(),
            description: "Physics puzzle with rigid bodies and triggers".into(),
            keywords: vec!["physics".into(), "puzzle".into(), "rigid".into(), "trigger".into(), "pressure plate".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node2D

signal puzzle_solved

@export var required_weight: float = 10.0
@onready var pressure_plate: Area2D = %PressurePlate

var current_weight: float = 0.0
var is_solved: bool = false

func _ready() -> void:
    pressure_plate.body_entered.connect(_on_body_entered)
    pressure_plate.body_exited.connect(_on_body_exited)

func _on_body_entered(body: Node2D) -> void:
    if body is RigidBody2D:
        current_weight += body.mass
        _check_solved()

func _on_body_exited(body: Node2D) -> void:
    if body is RigidBody2D:
        current_weight -= body.mass
        _check_solved()

func _check_solved() -> void:
    if current_weight >= required_weight and not is_solved:
        is_solved = true
        puzzle_solved.emit()
"#.into(),
        },
        Template {
            name: "ParticleController".into(),
            description: "Particle effect controller with presets".into(),
            keywords: vec!["particle".into(), "effect".into(), "vfx".into(), "explosion".into(), "emit".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node2D

@onready var particles: GPUParticles2D = %Particles

func play_at(pos: Vector2) -> void:
    global_position = pos
    particles.restart()
    particles.emitting = true

func play_explosion(pos: Vector2, color: Color = Color.WHITE) -> void:
    global_position = pos
    particles.process_material.set("color", color)
    particles.amount = 32
    particles.explosiveness = 1.0
    particles.restart()
    particles.emitting = true

func play_trail(pos: Vector2) -> void:
    global_position = pos
    particles.amount = 8
    particles.explosiveness = 0.0
    particles.emitting = true

func stop() -> void:
    particles.emitting = false
"#.into(),
        },
        Template {
            name: "SpawnSystem".into(),
            description: "Enemy/object spawner with wave support".into(),
            keywords: vec!["spawn".into(), "spawner".into(), "wave".into(), "enemy spawn".into(), "instantiate".into()],
            parameters: vec!["class_name".into(), "spawn_interval".into()],
            code: r#"extends Node2D

signal wave_completed(wave_number: int)
signal all_waves_completed

@export var enemy_scene: PackedScene
@export var spawn_interval: float = {{spawn_interval}}
@export var enemies_per_wave: int = 5
@export var total_waves: int = 3

var current_wave: int = 0
var enemies_spawned: int = 0
var enemies_alive: int = 0
var spawn_timer: float = 0.0
var spawning: bool = false

func start() -> void:
    current_wave = 0
    _start_next_wave()

func _start_next_wave() -> void:
    current_wave += 1
    enemies_spawned = 0
    spawning = true

func _process(delta: float) -> void:
    if not spawning:
        return
    spawn_timer -= delta
    if spawn_timer <= 0.0 and enemies_spawned < enemies_per_wave:
        _spawn_enemy()
        spawn_timer = spawn_interval

func _spawn_enemy() -> void:
    var enemy := enemy_scene.instantiate()
    enemy.global_position = global_position + Vector2(randf_range(-50, 50), 0)
    enemy.tree_exited.connect(_on_enemy_died)
    get_parent().add_child(enemy)
    enemies_spawned += 1
    enemies_alive += 1
    if enemies_spawned >= enemies_per_wave:
        spawning = false

func _on_enemy_died() -> void:
    enemies_alive -= 1
    if enemies_alive <= 0 and not spawning:
        wave_completed.emit(current_wave)
        if current_wave >= total_waves:
            all_waves_completed.emit()
        else:
            _start_next_wave()
"#.into(),
        },
        Template {
            name: "RPGBattleSystem".into(),
            description: "Turn-based RPG battle system".into(),
            keywords: vec!["rpg".into(), "battle".into(), "turn".into(), "turn-based".into(), "combat".into(), "fight".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node

signal battle_started
signal battle_ended(won: bool)
signal turn_changed(is_player_turn: bool)

enum BattleState { PLAYER_TURN, ENEMY_TURN, WON, LOST }

var state: BattleState = BattleState.PLAYER_TURN
var player_stats: Dictionary = {"hp": 100, "max_hp": 100, "attack": 20, "defense": 10}
var enemy_stats: Dictionary = {"hp": 80, "max_hp": 80, "attack": 15, "defense": 8}

func start_battle(player: Dictionary, enemy: Dictionary) -> void:
    player_stats = player
    enemy_stats = enemy
    state = BattleState.PLAYER_TURN
    battle_started.emit()
    turn_changed.emit(true)

func player_attack() -> int:
    if state != BattleState.PLAYER_TURN:
        return 0
    var damage := max(1, player_stats["attack"] - enemy_stats["defense"])
    enemy_stats["hp"] -= damage
    if enemy_stats["hp"] <= 0:
        state = BattleState.WON
        battle_ended.emit(true)
    else:
        state = BattleState.ENEMY_TURN
        turn_changed.emit(false)
        _enemy_turn()
    return damage

func _enemy_turn() -> void:
    await get_tree().create_timer(1.0).timeout
    var damage := max(1, enemy_stats["attack"] - player_stats["defense"])
    player_stats["hp"] -= damage
    if player_stats["hp"] <= 0:
        state = BattleState.LOST
        battle_ended.emit(false)
    else:
        state = BattleState.PLAYER_TURN
        turn_changed.emit(true)
"#.into(),
        },
        Template {
            name: "DungeonGenerator".into(),
            description: "Procedural dungeon generation with BSP".into(),
            keywords: vec!["dungeon".into(), "bsp".into(), "procedural".into(), "room".into(), "corridor".into(), "generate".into()],
            parameters: vec!["class_name".into(), "width".into(), "height".into()],
            code: r#"extends Node2D

@export var dungeon_width: int = {{width}}
@export var dungeon_height: int = {{height}}
@export var min_room_size: int = 6
@export var max_depth: int = 4

var rooms: Array[Rect2i] = []
var corridors: Array[Array] = []

func generate() -> Array[Rect2i]:
    rooms.clear()
    corridors.clear()
    var root := Rect2i(0, 0, dungeon_width, dungeon_height)
    _split(root, 0)
    return rooms

func _split(rect: Rect2i, depth: int) -> void:
    if depth >= max_depth or (rect.size.x < min_room_size * 2 and rect.size.y < min_room_size * 2):
        var room := _create_room(rect)
        rooms.append(room)
        return

    var split_h := randf() > 0.5
    if rect.size.x > rect.size.y * 1.5:
        split_h = false
    elif rect.size.y > rect.size.x * 1.5:
        split_h = true

    if split_h:
        var split_y := randi_range(min_room_size, rect.size.y - min_room_size)
        _split(Rect2i(rect.position, Vector2i(rect.size.x, split_y)), depth + 1)
        _split(Rect2i(rect.position + Vector2i(0, split_y), Vector2i(rect.size.x, rect.size.y - split_y)), depth + 1)
    else:
        var split_x := randi_range(min_room_size, rect.size.x - min_room_size)
        _split(Rect2i(rect.position, Vector2i(split_x, rect.size.y)), depth + 1)
        _split(Rect2i(rect.position + Vector2i(split_x, 0), Vector2i(rect.size.x - split_x, rect.size.y)), depth + 1)

func _create_room(bounds: Rect2i) -> Rect2i:
    var w := randi_range(min_room_size, bounds.size.x - 2)
    var h := randi_range(min_room_size, bounds.size.y - 2)
    var x := bounds.position.x + randi_range(1, bounds.size.x - w - 1)
    var y := bounds.position.y + randi_range(1, bounds.size.y - h - 1)
    return Rect2i(x, y, w, h)

func connect_rooms() -> void:
    for i in range(rooms.size() - 1):
        var a := rooms[i].get_center()
        var b := rooms[i + 1].get_center()
        corridors.append([a, b])
"#.into(),
        },
        Template {
            name: "NetworkSync".into(),
            description: "Basic multiplayer network synchronization".into(),
            keywords: vec!["network".into(), "multiplayer".into(), "sync".into(), "online".into(), "peer".into(), "server".into()],
            parameters: vec!["class_name".into(), "port".into()],
            code: r#"extends Node

signal player_connected(id: int)
signal player_disconnected(id: int)
signal connection_failed

@export var default_port: int = {{port}}
var peer: ENetMultiplayerPeer

func host_game(port: int = 0) -> Error:
    if port == 0:
        port = default_port
    peer = ENetMultiplayerPeer.new()
    var error := peer.create_server(port)
    if error != OK:
        return error
    multiplayer.multiplayer_peer = peer
    multiplayer.peer_connected.connect(_on_peer_connected)
    multiplayer.peer_disconnected.connect(_on_peer_disconnected)
    return OK

func join_game(address: String, port: int = 0) -> Error:
    if port == 0:
        port = default_port
    peer = ENetMultiplayerPeer.new()
    var error := peer.create_client(address, port)
    if error != OK:
        return error
    multiplayer.multiplayer_peer = peer
    multiplayer.connection_failed.connect(func(): connection_failed.emit())
    return OK

func _on_peer_connected(id: int) -> void:
    player_connected.emit(id)

func _on_peer_disconnected(id: int) -> void:
    player_disconnected.emit(id)

func disconnect_game() -> void:
    if peer:
        peer.close()
        multiplayer.multiplayer_peer = null
"#.into(),
        },
        Template {
            name: "WaterShader".into(),
            description: "Water surface shader effect".into(),
            keywords: vec!["shader".into(), "water".into(), "wave".into(), "surface".into(), "visual".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Sprite2D

# Attach this shader to a Sprite2D or ColorRect
# shader_type canvas_item;
# uniform float wave_speed = 2.0;
# uniform float wave_amplitude = 0.02;
# uniform float wave_frequency = 10.0;
# uniform vec4 water_color : source_color = vec4(0.1, 0.3, 0.8, 0.7);
# void fragment() {
#     vec2 uv = UV;
#     uv.y += sin(uv.x * wave_frequency + TIME * wave_speed) * wave_amplitude;
#     uv.x += cos(uv.y * wave_frequency * 0.5 + TIME * wave_speed * 0.7) * wave_amplitude * 0.5;
#     vec4 tex = texture(TEXTURE, uv);
#     COLOR = mix(tex, water_color, water_color.a);
# }

@export var wave_speed: float = 2.0
@export var wave_amplitude: float = 0.02

func _ready() -> void:
    var shader_material := material as ShaderMaterial
    if shader_material:
        shader_material.set_shader_parameter("wave_speed", wave_speed)
        shader_material.set_shader_parameter("wave_amplitude", wave_amplitude)

func set_wave_params(speed: float, amplitude: float) -> void:
    wave_speed = speed
    wave_amplitude = amplitude
    var shader_material := material as ShaderMaterial
    if shader_material:
        shader_material.set_shader_parameter("wave_speed", speed)
        shader_material.set_shader_parameter("wave_amplitude", amplitude)
"#.into(),
        },
        Template {
            name: "StateMachine".into(),
            description: "Generic state machine pattern".into(),
            keywords: vec!["state machine".into(), "state".into(), "generic".into(), "pattern".into(), "transition".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node

signal state_changed(old_state: String, new_state: String)

@export var initial_state: NodePath

var current_state: Node
var states: Dictionary = {}

func _ready() -> void:
    for child in get_children():
        if child.has_method("enter"):
            states[child.name] = child
            child.state_machine = self
    if initial_state:
        current_state = get_node(initial_state)
        current_state.enter()

func transition_to(state_name: String, msg: Dictionary = {}) -> void:
    if not states.has(state_name):
        push_warning("State not found: " + state_name)
        return
    var old_name := current_state.name if current_state else ""
    if current_state:
        current_state.exit()
    current_state = states[state_name]
    current_state.enter(msg)
    state_changed.emit(old_name, state_name)

func _process(delta: float) -> void:
    if current_state and current_state.has_method("update"):
        current_state.update(delta)

func _physics_process(delta: float) -> void:
    if current_state and current_state.has_method("physics_update"):
        current_state.physics_update(delta)
"#.into(),
        },
        Template {
            name: "MovingPlatform".into(),
            description: "Moving platform with waypoints".into(),
            keywords: vec!["platform".into(), "moving".into(), "waypoint".into(), "elevator".into(), "lift".into()],
            parameters: vec!["class_name".into(), "speed".into()],
            code: r#"extends AnimatableBody2D

@export var speed: float = {{speed}}
@export var waypoints: Array[Vector2] = []
@export var loop: bool = true

var current_waypoint: int = 0
var direction: int = 1

func _ready() -> void:
    if waypoints.is_empty():
        waypoints = [global_position, global_position + Vector2(0, -200)]

func _physics_process(delta: float) -> void:
    if waypoints.is_empty():
        return
    var target := waypoints[current_waypoint]
    var move_delta := speed * delta
    global_position = global_position.move_toward(target, move_delta)

    if global_position.distance_to(target) < 1.0:
        if loop:
            current_waypoint = (current_waypoint + 1) % waypoints.size()
        else:
            current_waypoint += direction
            if current_waypoint >= waypoints.size() or current_waypoint < 0:
                direction *= -1
                current_waypoint += direction * 2
"#.into(),
        },
        Template {
            name: "Interactable".into(),
            description: "Interactable object with prompt display".into(),
            keywords: vec!["interact".into(), "interactable".into(), "prompt".into(), "use".into(), "action".into(), "npc".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Area2D

signal interacted

@export var interaction_text: String = "Press E to interact"
@onready var label: Label = %InteractLabel

var player_in_range: bool = false

func _ready() -> void:
    body_entered.connect(_on_body_entered)
    body_exited.connect(_on_body_exited)
    label.visible = false

func _input(event: InputEvent) -> void:
    if player_in_range and event.is_action_pressed("interact"):
        interacted.emit()

func _on_body_entered(body: Node2D) -> void:
    if body.is_in_group("player"):
        player_in_range = true
        label.text = interaction_text
        label.visible = true

func _on_body_exited(body: Node2D) -> void:
    if body.is_in_group("player"):
        player_in_range = false
        label.visible = false
"#.into(),
        },
        Template {
            name: "DamageNumbers".into(),
            description: "Floating damage number popup".into(),
            keywords: vec!["damage number".into(), "floating text".into(), "popup".into(), "hit".into(), "number".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Node2D

@export var float_speed: float = 80.0
@export var fade_duration: float = 0.8
@export var spread: float = 40.0

func show_damage(amount: int, pos: Vector2, is_crit: bool = false) -> void:
    var label := Label.new()
    label.text = str(amount)
    label.position = pos + Vector2(randf_range(-spread, spread), 0)
    label.z_index = 100
    if is_crit:
        label.add_theme_font_size_override("font_size", 24)
        label.add_theme_color_override("font_color", Color.RED)
    else:
        label.add_theme_font_size_override("font_size", 16)
        label.add_theme_color_override("font_color", Color.WHITE)
    add_child(label)

    var tween := create_tween()
    tween.set_parallel(true)
    tween.tween_property(label, "position:y", label.position.y - float_speed, fade_duration)
    tween.tween_property(label, "modulate:a", 0.0, fade_duration)
    tween.chain().tween_callback(label.queue_free)
"#.into(),
        },
        Template {
            name: "OptionsMenu".into(),
            description: "Options/settings menu with audio and display".into(),
            keywords: vec!["options".into(), "settings".into(), "volume".into(), "fullscreen".into(), "resolution".into(), "config".into()],
            parameters: vec!["class_name".into()],
            code: r#"extends Control

@onready var music_slider: HSlider = %MusicSlider
@onready var sfx_slider: HSlider = %SFXSlider
@onready var fullscreen_check: CheckButton = %FullscreenCheck
@onready var vsync_check: CheckButton = %VsyncCheck

func _ready() -> void:
    music_slider.value_changed.connect(_on_music_changed)
    sfx_slider.value_changed.connect(_on_sfx_changed)
    fullscreen_check.toggled.connect(_on_fullscreen_toggled)
    vsync_check.toggled.connect(_on_vsync_toggled)
    _load_settings()

func _on_music_changed(value: float) -> void:
    AudioServer.set_bus_volume_db(AudioServer.get_bus_index("Music"), linear_to_db(value))

func _on_sfx_changed(value: float) -> void:
    AudioServer.set_bus_volume_db(AudioServer.get_bus_index("SFX"), linear_to_db(value))

func _on_fullscreen_toggled(enabled: bool) -> void:
    if enabled:
        DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
    else:
        DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)

func _on_vsync_toggled(enabled: bool) -> void:
    DisplayServer.window_set_vsync_mode(
        DisplayServer.VSYNC_ENABLED if enabled else DisplayServer.VSYNC_DISABLED
    )

func _load_settings() -> void:
    var config := ConfigFile.new()
    if config.load("user://settings.cfg") == OK:
        music_slider.value = config.get_value("audio", "music", 1.0)
        sfx_slider.value = config.get_value("audio", "sfx", 1.0)
        fullscreen_check.button_pressed = config.get_value("display", "fullscreen", false)

func save_settings() -> void:
    var config := ConfigFile.new()
    config.set_value("audio", "music", music_slider.value)
    config.set_value("audio", "sfx", sfx_slider.value)
    config.set_value("display", "fullscreen", fullscreen_check.button_pressed)
    config.save("user://settings.cfg")
"#.into(),
        },
        Template {
            name: "Minimap".into(),
            description: "Minimap display with player and enemy markers".into(),
            keywords: vec!["minimap".into(), "map".into(), "radar".into(), "marker".into(), "hud".into()],
            parameters: vec!["class_name".into(), "zoom".into()],
            code: r#"extends Control

@export var zoom: float = {{zoom}}
@export var player_color: Color = Color.GREEN
@export var enemy_color: Color = Color.RED
@export var map_size: Vector2 = Vector2(150, 150)

var player: Node2D

func _ready() -> void:
    custom_minimum_size = map_size

func _process(_delta: float) -> void:
    if not player:
        var players := get_tree().get_nodes_in_group("player")
        if not players.is_empty():
            player = players[0]
    queue_redraw()

func _draw() -> void:
    draw_rect(Rect2(Vector2.ZERO, map_size), Color(0, 0, 0, 0.5))
    if not player:
        return
    var center := map_size / 2.0
    draw_circle(center, 4.0, player_color)
    var enemies := get_tree().get_nodes_in_group("enemy")
    for enemy in enemies:
        var offset := (enemy.global_position - player.global_position) / zoom
        var marker_pos := center + offset
        if Rect2(Vector2.ZERO, map_size).has_point(marker_pos):
            draw_circle(marker_pos, 3.0, enemy_color)
"#.into(),
        },
    ]
}
