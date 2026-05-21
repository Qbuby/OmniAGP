use anyhow::{bail, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourcePool {
    Gpu,
    Llm,
    Cpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagTask {
    pub id: Uuid,
    pub name: String,
    pub task_type: String,
    pub resource_pool: ResourcePool,
    pub status: TaskStatus,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl DagTask {
    pub fn new(name: &str, task_type: &str, resource_pool: ResourcePool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            task_type: task_type.to_string(),
            resource_pool,
            status: TaskStatus::Pending,
            input: serde_json::Value::Null,
            output: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }
}

pub struct TaskDag {
    graph: DiGraph<Uuid, ()>,
    tasks: HashMap<Uuid, DagTask>,
    node_map: HashMap<Uuid, NodeIndex>,
}

impl TaskDag {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            tasks: HashMap::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, task: DagTask) -> Uuid {
        let id = task.id;
        let idx = self.graph.add_node(id);
        self.node_map.insert(id, idx);
        self.tasks.insert(id, task);
        id
    }

    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) -> Result<()> {
        let from_idx = self.node_map.get(&from).ok_or_else(|| anyhow::anyhow!("task not found: {}", from))?;
        let to_idx = self.node_map.get(&to).ok_or_else(|| anyhow::anyhow!("task not found: {}", to))?;
        self.graph.add_edge(*from_idx, *to_idx, ());
        Ok(())
    }

    pub fn validate(&self) -> Result<Vec<Uuid>> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.iter().map(|idx| self.graph[*idx]).collect()),
            Err(_) => bail!("DAG contains a cycle"),
        }
    }

    pub fn get_task(&self, id: &Uuid) -> Option<&DagTask> {
        self.tasks.get(id)
    }

    pub fn get_task_mut(&mut self, id: &Uuid) -> Option<&mut DagTask> {
        self.tasks.get_mut(id)
    }

    pub fn ready_tasks(&self) -> Vec<Uuid> {
        self.tasks
            .iter()
            .filter(|(_, task)| task.status == TaskStatus::Pending || task.status == TaskStatus::Ready)
            .filter(|(id, _)| {
                let idx = self.node_map[id];
                self.graph
                    .neighbors_directed(idx, petgraph::Direction::Incoming)
                    .all(|dep_idx| {
                        let dep_id = self.graph[dep_idx];
                        matches!(
                            self.tasks[&dep_id].status,
                            TaskStatus::Completed | TaskStatus::Skipped
                        )
                    })
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn all_completed(&self) -> bool {
        self.tasks.values().all(|t| {
            matches!(t.status, TaskStatus::Completed | TaskStatus::Skipped | TaskStatus::Failed)
        })
    }

    pub fn tasks(&self) -> &HashMap<Uuid, DagTask> {
        &self.tasks
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks.values().filter(|t| t.status == TaskStatus::Completed).count()
    }

    pub fn failed_count(&self) -> usize {
        self.tasks.values().filter(|t| t.status == TaskStatus::Failed).count()
    }
}

pub struct ResourceLimits {
    pub gpu_slots: usize,
    pub llm_slots: usize,
    pub cpu_slots: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            gpu_slots: 1,
            llm_slots: 4,
            cpu_slots: 8,
        }
    }
}

pub struct DagScheduler {
    dag: Arc<Mutex<TaskDag>>,
    gpu_semaphore: Arc<Semaphore>,
    llm_semaphore: Arc<Semaphore>,
    cpu_semaphore: Arc<Semaphore>,
}

impl DagScheduler {
    pub fn new(dag: TaskDag, limits: ResourceLimits) -> Self {
        Self {
            dag: Arc::new(Mutex::new(dag)),
            gpu_semaphore: Arc::new(Semaphore::new(limits.gpu_slots)),
            llm_semaphore: Arc::new(Semaphore::new(limits.llm_slots)),
            cpu_semaphore: Arc::new(Semaphore::new(limits.cpu_slots)),
        }
    }

    pub fn dag(&self) -> &Arc<Mutex<TaskDag>> {
        &self.dag
    }

    fn semaphore_for(&self, pool: ResourcePool) -> &Arc<Semaphore> {
        match pool {
            ResourcePool::Gpu => &self.gpu_semaphore,
            ResourcePool::Llm => &self.llm_semaphore,
            ResourcePool::Cpu => &self.cpu_semaphore,
        }
    }

    pub async fn next_batch(&self) -> Vec<(Uuid, ResourcePool)> {
        let dag = self.dag.lock().await;
        dag.ready_tasks()
            .into_iter()
            .map(|id| {
                let pool = dag.get_task(&id).unwrap().resource_pool;
                (id, pool)
            })
            .collect()
    }

    pub async fn acquire_resource(&self, pool: ResourcePool) -> tokio::sync::OwnedSemaphorePermit {
        let sem = self.semaphore_for(pool).clone();
        sem.acquire_owned().await.unwrap()
    }

    pub async fn mark_running(&self, task_id: &Uuid) {
        let mut dag = self.dag.lock().await;
        if let Some(task) = dag.get_task_mut(task_id) {
            task.status = TaskStatus::Running;
        }
    }

    pub async fn mark_completed(&self, task_id: &Uuid, output: serde_json::Value) {
        let mut dag = self.dag.lock().await;
        if let Some(task) = dag.get_task_mut(task_id) {
            task.status = TaskStatus::Completed;
            task.output = Some(output);
        }
    }

    pub async fn mark_failed(&self, task_id: &Uuid, error: String) {
        let mut dag = self.dag.lock().await;
        if let Some(task) = dag.get_task_mut(task_id) {
            task.error = Some(error);
            task.retry_count += 1;
            if task.retry_count >= task.max_retries {
                task.status = TaskStatus::Failed;
            } else {
                task.status = TaskStatus::Pending;
            }
        }
    }

    pub async fn is_complete(&self) -> bool {
        let dag = self.dag.lock().await;
        dag.all_completed()
    }

    pub async fn progress(&self) -> (usize, usize, usize) {
        let dag = self.dag.lock().await;
        (dag.completed_count(), dag.failed_count(), dag.task_count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_ordering() {
        let mut dag = TaskDag::new();
        let a = dag.add_task(DagTask::new("analyze", "llm", ResourcePool::Llm));
        let b = dag.add_task(DagTask::new("codegen", "llm", ResourcePool::Llm));
        let c = dag.add_task(DagTask::new("asset_2d", "gpu", ResourcePool::Gpu));
        let d = dag.add_task(DagTask::new("assemble", "cpu", ResourcePool::Cpu));

        dag.add_dependency(a, b).unwrap();
        dag.add_dependency(a, c).unwrap();
        dag.add_dependency(b, d).unwrap();
        dag.add_dependency(c, d).unwrap();

        let order = dag.validate().unwrap();
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], a);

        let ready = dag.ready_tasks();
        assert_eq!(ready, vec![a]);
    }

    #[test]
    fn test_parallel_ready() {
        let mut dag = TaskDag::new();
        let a = dag.add_task(DagTask::new("analyze", "llm", ResourcePool::Llm));
        let b = dag.add_task(DagTask::new("codegen", "llm", ResourcePool::Llm));
        let c = dag.add_task(DagTask::new("asset_2d", "gpu", ResourcePool::Gpu));

        dag.add_dependency(a, b).unwrap();
        dag.add_dependency(a, c).unwrap();

        dag.get_task_mut(&a).unwrap().status = TaskStatus::Completed;

        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&b));
        assert!(ready.contains(&c));
    }

    #[test]
    fn test_cycle_detection() {
        let mut dag = TaskDag::new();
        let a = dag.add_task(DagTask::new("a", "t", ResourcePool::Cpu));
        let b = dag.add_task(DagTask::new("b", "t", ResourcePool::Cpu));

        dag.add_dependency(a, b).unwrap();
        dag.add_dependency(b, a).unwrap();

        assert!(dag.validate().is_err());
    }
}
