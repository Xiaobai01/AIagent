use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action: WorkflowAction,
    pub next_step_id: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowAction {
    Message(String),
    ToolCall { tool_name: String, parameters: serde_json::Value },
    Plan(String),
    Reflect,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub current_step_id: Option<String>,
    pub status: WorkflowStatus,
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Idle,
    Running,
    Completed,
    Failed,
    Paused,
}

pub struct WorkflowEngine {
    workflows: HashMap<String, Workflow>,
    running_workflows: VecDeque<Workflow>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            running_workflows: VecDeque::new(),
        }
    }

    pub fn register_workflow(&mut self, workflow: Workflow) {
        self.workflows.insert(workflow.id.clone(), workflow);
    }

    pub fn start_workflow(&mut self, workflow_id: &str) -> Result<Workflow> {
        let workflow = self.workflows.get(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found"))?
            .clone();
        
        let mut running_workflow = workflow.clone();
        running_workflow.status = WorkflowStatus::Running;
        running_workflow.current_step_id = workflow.steps.first().map(|s| s.id.clone());
        
        self.running_workflows.push_back(running_workflow.clone());
        Ok(running_workflow)
    }

    pub fn get_next_step<'a>(&self, workflow: &'a Workflow) -> Option<&'a WorkflowStep> {
        match &workflow.current_step_id {
            Some(step_id) => workflow.steps.iter().find(|s| s.id == *step_id),
            None => None,
        }
    }

    pub fn advance_workflow(&mut self, workflow_id: &str) -> Result<()> {
        if let Some(workflow) = self.running_workflows.iter_mut().find(|w| w.id == workflow_id) {
            let current_step_id = workflow.current_step_id.clone();
            
            if let Some(current_step_id) = current_step_id {
                if let Some(step) = workflow.steps.iter().find(|s| s.id == current_step_id) {
                    if matches!(step.action, WorkflowAction::End) {
                        workflow.status = WorkflowStatus::Completed;
                        workflow.current_step_id = None;
                        return Ok(());
                    }
                    
                    workflow.current_step_id = step.next_step_id.clone();
                    
                    if workflow.current_step_id.is_none() {
                        workflow.status = WorkflowStatus::Completed;
                    }
                    return Ok(());
                }
            }
        }
        anyhow::bail!("Workflow not found")
    }

    pub fn update_context(&mut self, workflow_id: &str, key: &str, value: serde_json::Value) -> Result<()> {
        if let Some(workflow) = self.running_workflows.iter_mut().find(|w| w.id == workflow_id) {
            workflow.context.insert(key.to_string(), value);
            Ok(())
        } else {
            anyhow::bail!("Workflow not found")
        }
    }

    pub fn get_workflows(&self) -> Vec<&Workflow> {
        self.workflows.values().collect()
    }

    pub fn get_running_workflows(&self) -> Vec<&Workflow> {
        self.running_workflows.iter().collect()
    }
}
