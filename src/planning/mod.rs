use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use crate::llm::{LLMProvider, LLMConfig, create_llm_provider};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub subtasks: Vec<String>,
    pub dependencies: Vec<String>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub tasks: Vec<Task>,
    pub current_task_index: usize,
    pub status: PlanStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanStatus {
    Planning,
    Executing,
    Completed,
    Failed,
    Paused,
}

pub struct Planner {
    llm_provider: Box<dyn LLMProvider>,
    plans: VecDeque<Plan>,
}

impl Planner {
    pub fn new(llm_config: LLMConfig) -> Result<Self> {
        let llm_provider = create_llm_provider(llm_config)?;
        Ok(Self {
            llm_provider,
            plans: VecDeque::new(),
        })
    }

    pub async fn create_plan(&mut self, goal: &str) -> Result<Plan> {
        let prompt = format!(
            r#"
            You are a task planning AI. Given a goal, break it down into a step-by-step plan.
            
            Goal: {}
            
            Please provide a detailed plan with numbered steps. Each step should be:
            1. Clear and actionable
            2. Independent (can be done without other steps unless specified)
            3. Specific (not vague)
            
            Format your response as JSON with the following structure:
            {{
                "tasks": [
                    {{"id": "task-1", "description": "...", "subtasks": [], "dependencies": []}},
                    {{"id": "task-2", "description": "...", "subtasks": [], "dependencies": ["task-1"]}},
                    ...
                ]
            }}
            "#,
            goal
        );

        let messages = vec![crate::core::Message::user(prompt.clone())];
        let response = self.llm_provider.chat(messages).await?;
        let response_text = match response {
            crate::core::LLMResponse::Text(text) => text,
            _ => "{}".to_string(),
        };
        
        let plan_data: serde_json::Value = serde_json::from_str(&response_text)?;
        let tasks: Vec<Task> = serde_json::from_value(plan_data["tasks"].clone())?;
        
        let plan = Plan {
            id: uuid::Uuid::new_v4().to_string(),
            goal: goal.to_string(),
            tasks: tasks.into_iter()
                .map(|mut t| {
                    t.status = TaskStatus::Pending;
                    t
                })
                .collect(),
            current_task_index: 0,
            status: PlanStatus::Planning,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        self.plans.push_back(plan.clone());
        Ok(plan)
    }

    pub async fn reflect_on_plan(&self, plan: &Plan) -> Result<String> {
        let completed_tasks = plan.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed))
            .count();
        let failed_tasks = plan.tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Failed))
            .count();
        
        let prompt = format!(
            r#"
            You are a reflective AI assistant. Analyze the following plan execution:
            
            Goal: {}
            Total tasks: {}
            Completed tasks: {}
            Failed tasks: {}
            
            Tasks:
            {}
            
            Please provide a reflection on:
            1. What went well
            2. What could be improved
            3. Suggestions for revision if the plan failed
            "#,
            plan.goal,
            plan.tasks.len(),
            completed_tasks,
            failed_tasks,
            plan.tasks.iter()
                .map(|t| format!("{}: {} - {:?}", t.id, t.description, t.status))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        let messages = vec![crate::core::Message::user(prompt.clone())];
        let response = self.llm_provider.chat(messages).await?;
        match response {
            crate::core::LLMResponse::Text(text) => Ok(text),
            _ => Ok("Failed to get reflection".to_string()),
        }
    }

    pub fn get_current_plan(&self) -> Option<&Plan> {
        self.plans.back()
    }

    pub fn update_task_status(&mut self, task_id: &str, new_status: TaskStatus) -> Result<()> {
        if let Some(plan) = self.plans.back_mut() {
            if let Some(task) = plan.tasks.iter_mut().find(|t| t.id == task_id) {
                let status = new_status.clone();
                task.status = new_status;
                plan.updated_at = chrono::Utc::now();
                
                if matches!(status, TaskStatus::Completed) {
                    if plan.tasks.iter().all(|t| matches!(t.status, TaskStatus::Completed)) {
                        plan.status = PlanStatus::Completed;
                    } else {
                        plan.status = PlanStatus::Executing;
                    }
                } else if matches!(status, TaskStatus::Failed) {
                    plan.status = PlanStatus::Failed;
                }
                return Ok(());
            }
        }
        anyhow::bail!("Task not found")
    }

    pub fn get_plans(&self) -> Vec<&Plan> {
        self.plans.iter().collect()
    }
}
