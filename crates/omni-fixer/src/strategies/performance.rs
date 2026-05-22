use anyhow::Result;
use async_trait::async_trait;
use omni_llm::{ChatMessage, ChatRequest, LlmClient, Role};
use tracing::info;

use crate::types::{BugReport, DegradationLevel, Patch};
use super::FixStrategy;

pub struct PerformanceStrategy {
    llm: LlmClient,
    model: String,
}

impl PerformanceStrategy {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }
}

#[async_trait]
impl FixStrategy for PerformanceStrategy {
    fn name(&self) -> &str {
        "performance_issue"
    }

    async fn generate_fix(
        &self,
        bug: &BugReport,
        level: DegradationLevel,
        previous_attempts: &[Patch],
    ) -> Result<Patch> {
        info!(bug_id = %bug.id, level = ?level, "generating performance fix");

        let perf_issue = bug.context.get("perf_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        match level {
            DegradationLevel::Retry => {
                let pattern = select_optimization_pattern(perf_issue);
                apply_known_pattern(bug, &pattern)
            }
            DegradationLevel::AugmentedContext => {
                let mut prompt = format!(
                    "Performance issue: {}\nType: {}\nFile: {}\n",
                    bug.description,
                    perf_issue,
                    bug.related_files.first().map(|s| s.as_str()).unwrap_or("unknown"),
                );

                if let Some(source) = bug.context.get("source_code") {
                    prompt.push_str(&format!("\nSource:\n{}\n", source));
                }
                if let Some(profile) = bug.context.get("profile_data") {
                    prompt.push_str(&format!("\nProfile data:\n{}\n", profile));
                }

                if !previous_attempts.is_empty() {
                    prompt.push_str("\nPrevious failed optimizations:\n");
                    for a in previous_attempts {
                        prompt.push_str(&format!("- {}\n", a.description));
                    }
                }

                let request = ChatRequest {
                    model: self.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: Role::System,
                            content: PERF_SYSTEM_PROMPT.to_string(),
                        },
                        ChatMessage {
                            role: Role::User,
                            content: prompt,
                        },
                    ],
                    temperature: Some(0.1),
                    max_tokens: Some(4096),
                };

                let response = self.llm.chat(&request).await?;
                let fixed_code = response.choices[0].message.content.trim().to_string();

                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: bug.context.get("source_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    modified: fixed_code,
                    description: format!("Performance fix (LLM-guided) for {}", perf_issue),
                })
            }
            DegradationLevel::TemplateFallback => {
                let pattern = select_aggressive_pattern(perf_issue);
                apply_known_pattern(bug, &pattern)
            }
            DegradationLevel::Escalate => {
                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: String::new(),
                    modified: String::new(),
                    description: format!("ESCALATE: Performance issue '{}' requires profiling and manual optimization", perf_issue),
                })
            }
        }
    }
}

fn select_optimization_pattern(perf_type: &str) -> OptimizationPattern {
    match perf_type {
        "object_pool" | "spawn_heavy" => OptimizationPattern::ObjectPool,
        "draw_calls" | "rendering" => OptimizationPattern::ReduceDrawCalls,
        "physics" | "collision" => OptimizationPattern::SimplifyPhysics,
        "memory" | "leak" => OptimizationPattern::MemoryCleanup,
        "infinite_loop" | "loop" => OptimizationPattern::LoopGuard,
        _ => OptimizationPattern::GeneralOptimize,
    }
}

fn select_aggressive_pattern(perf_type: &str) -> OptimizationPattern {
    match perf_type {
        "object_pool" | "spawn_heavy" => OptimizationPattern::ObjectPool,
        "draw_calls" | "rendering" => OptimizationPattern::LodSimplification,
        "physics" | "collision" => OptimizationPattern::DisableDistantPhysics,
        "memory" | "leak" => OptimizationPattern::ForceGarbageCollect,
        "infinite_loop" | "loop" => OptimizationPattern::LoopGuard,
        _ => OptimizationPattern::ObjectPool,
    }
}

enum OptimizationPattern {
    ObjectPool,
    ReduceDrawCalls,
    LodSimplification,
    SimplifyPhysics,
    DisableDistantPhysics,
    MemoryCleanup,
    ForceGarbageCollect,
    LoopGuard,
    GeneralOptimize,
}

fn apply_known_pattern(bug: &BugReport, pattern: &OptimizationPattern) -> Result<Patch> {
    let file_path = bug.related_files.first().cloned().unwrap_or_default();
    let original = bug.context.get("source_code")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (modified, desc) = match pattern {
        OptimizationPattern::ObjectPool => {
            let code = format!(
                "{}\n\n# Object Pool optimization\n\
                 var _pool: Array[Node] = []\n\
                 var _pool_size: int = 20\n\n\
                 func _init_pool(scene: PackedScene) -> void:\n\
                 \tfor i in range(_pool_size):\n\
                 \t\tvar obj := scene.instantiate()\n\
                 \t\tobj.set_process(false)\n\
                 \t\tobj.hide()\n\
                 \t\t_pool.append(obj)\n\n\
                 func _get_from_pool() -> Node:\n\
                 \tfor obj in _pool:\n\
                 \t\tif not obj.visible:\n\
                 \t\t\tobj.set_process(true)\n\
                 \t\t\tobj.show()\n\
                 \t\t\treturn obj\n\
                 \treturn null\n\n\
                 func _return_to_pool(obj: Node) -> void:\n\
                 \tobj.set_process(false)\n\
                 \tobj.hide()\n",
                original
            );
            (code, "Applied object pool pattern")
        }
        OptimizationPattern::ReduceDrawCalls | OptimizationPattern::LodSimplification => {
            let code = format!(
                "{}\n\n# LOD / Draw call reduction\n\
                 func _apply_lod(camera_pos: Vector3) -> void:\n\
                 \tvar dist := global_position.distance_to(camera_pos)\n\
                 \tif dist > 100.0:\n\
                 \t\tvisible = false\n\
                 \telif dist > 50.0:\n\
                 \t\t# Low detail\n\
                 \t\tif has_node(\"MeshHigh\"):\n\
                 \t\t\t$MeshHigh.visible = false\n\
                 \t\tif has_node(\"MeshLow\"):\n\
                 \t\t\t$MeshLow.visible = true\n\
                 \telse:\n\
                 \t\tvisible = true\n\
                 \t\tif has_node(\"MeshHigh\"):\n\
                 \t\t\t$MeshHigh.visible = true\n",
                original
            );
            (code, "Applied LOD/draw call reduction")
        }
        OptimizationPattern::SimplifyPhysics | OptimizationPattern::DisableDistantPhysics => {
            let code = format!(
                "{}\n\n# Physics optimization\n\
                 var _physics_enabled: bool = true\n\n\
                 func _optimize_physics(player_pos: Vector3) -> void:\n\
                 \tvar dist := global_position.distance_to(player_pos)\n\
                 \tvar should_enable := dist < 30.0\n\
                 \tif should_enable != _physics_enabled:\n\
                 \t\t_physics_enabled = should_enable\n\
                 \t\tset_physics_process(should_enable)\n\
                 \t\tif has_node(\"CollisionShape3D\"):\n\
                 \t\t\t$CollisionShape3D.disabled = not should_enable\n",
                original
            );
            (code, "Applied distance-based physics optimization")
        }
        OptimizationPattern::MemoryCleanup | OptimizationPattern::ForceGarbageCollect => {
            let code = format!(
                "{}\n\n# Memory cleanup\n\
                 func _cleanup_resources() -> void:\n\
                 \tfor child in get_children():\n\
                 \t\tif child is GPUParticles2D or child is GPUParticles3D:\n\
                 \t\t\tif not child.emitting:\n\
                 \t\t\t\tchild.queue_free()\n\
                 \tif get_child_count() > 100:\n\
                 \t\tfor i in range(get_child_count() - 50):\n\
                 \t\t\tget_child(i).queue_free()\n",
                original
            );
            (code, "Applied memory cleanup pattern")
        }
        OptimizationPattern::LoopGuard => {
            let modified = if original.contains("while") {
                original.replace(
                    "while",
                    "var _loop_guard: int = 0\n# Loop guard added\nwhile",
                ).replace(
                    ":\n",
                    ":\n\t\t_loop_guard += 1\n\t\tif _loop_guard > 10000:\n\t\t\tpush_error(\"Loop guard triggered\")\n\t\t\tbreak\n",
                )
            } else {
                format!(
                    "{}\n\n# Loop guard utility\n\
                     const MAX_ITERATIONS: int = 10000\n",
                    original
                )
            };
            (modified, "Applied loop guard pattern")
        }
        OptimizationPattern::GeneralOptimize => {
            let code = format!(
                "{}\n\n# General optimization: cache and throttle\n\
                 var _update_timer: float = 0.0\n\
                 const UPDATE_INTERVAL: float = 0.1\n\n\
                 func _process(delta: float) -> void:\n\
                 \t_update_timer += delta\n\
                 \tif _update_timer < UPDATE_INTERVAL:\n\
                 \t\treturn\n\
                 \t_update_timer = 0.0\n\
                 \t_throttled_update()\n\n\
                 func _throttled_update() -> void:\n\
                 \tpass\n",
                original
            );
            (code, "Applied general throttle optimization")
        }
    };

    Ok(Patch {
        file_path,
        original,
        modified,
        description: desc.to_string(),
    })
}

const PERF_SYSTEM_PROMPT: &str = "You are a Godot 4 performance optimization expert. \
Apply these patterns as appropriate:\n\
- Object pooling for frequently spawned/destroyed nodes\n\
- LOD (Level of Detail) for distant objects\n\
- Reduce draw calls by batching or hiding distant objects\n\
- Disable physics for off-screen or distant objects\n\
- Use _physics_process only when needed, prefer _process with throttling\n\
- Cache expensive calculations\n\
- Use signals instead of polling\n\
- Add loop guards to prevent infinite loops\n\n\
Output ONLY the corrected full file content.";
