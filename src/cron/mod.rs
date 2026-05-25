use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task;
use chrono::{DateTime, Timelike, Datelike, Local};
use std::time::Duration;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    pub minute: String,
    pub hour: String,
    pub day: String,
    pub month: String,
    pub weekday: String,
}

impl CronSchedule {
    pub fn parse(cron_str: &str) -> Result<Self> {
        let parts: Vec<&str> = cron_str.split_whitespace().collect();
        if parts.len() != 5 {
            anyhow::bail!("Invalid cron expression: expected 5 fields");
        }
        
        Ok(Self {
            minute: parts[0].to_string(),
            hour: parts[1].to_string(),
            day: parts[2].to_string(),
            month: parts[3].to_string(),
            weekday: parts[4].to_string(),
        })
    }
    
    pub fn every_minute() -> Self {
        Self {
            minute: "*".to_string(),
            hour: "*".to_string(),
            day: "*".to_string(),
            month: "*".to_string(),
            weekday: "*".to_string(),
        }
    }
    
    pub fn hourly() -> Self {
        Self {
            minute: "0".to_string(),
            hour: "*".to_string(),
            day: "*".to_string(),
            month: "*".to_string(),
            weekday: "*".to_string(),
        }
    }
    
    pub fn daily(hour: u32) -> Self {
        Self {
            minute: "0".to_string(),
            hour: hour.to_string(),
            day: "*".to_string(),
            month: "*".to_string(),
            weekday: "*".to_string(),
        }
    }
    
    pub fn weekly(weekday: &str, hour: u32) -> Self {
        Self {
            minute: "0".to_string(),
            hour: hour.to_string(),
            day: "*".to_string(),
            month: "*".to_string(),
            weekday: weekday.to_string(),
        }
    }
    
    fn matches(&self, now: &DateTime<Local>) -> bool {
        if !self.matches_field(&self.minute, now.minute() as i64) {
            return false;
        }
        if !self.matches_field(&self.hour, now.hour() as i64) {
            return false;
        }
        if !self.matches_field(&self.day, now.day() as i64) {
            return false;
        }
        if !self.matches_field(&self.month, now.month() as i64) {
            return false;
        }
        if !self.matches_field(&self.weekday, now.weekday().number_from_monday() as i64) {
            return false;
        }
        true
    }
    
    fn matches_field(&self, pattern: &str, value: i64) -> bool {
        if pattern == "*" {
            return true;
        }
        
        for part in pattern.split(',') {
            if let Some((start, end)) = part.split_once('-') {
                if let (Ok(start), Ok(end)) = (start.parse::<i64>(), end.parse::<i64>()) {
                    if value >= start && value <= end {
                        return true;
                    }
                }
            } else if let Ok(num) = part.parse::<i64>() {
                if value == num {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: CronSchedule,
    pub task_type: TaskType,
    pub parameters: serde_json::Value,
    pub enabled: bool,
    pub last_run: Option<DateTime<Local>>,
    pub next_run: Option<DateTime<Local>>,
    pub run_count: u64,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    ChatMessage { message: String },
    SkillExecution { skill_name: String, params: serde_json::Value },
    CustomCommand { command: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskExecutionResult {
    pub task_id: String,
    pub success: bool,
    pub result: String,
    pub executed_at: DateTime<Local>,
}

type TaskCallback = dyn Fn(&ScheduledTask) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> + Send + Sync + 'static;

pub struct CronManager {
    tasks: Arc<Mutex<HashMap<String, ScheduledTask>>>,
    callback: Arc<TaskCallback>,
    running: Arc<Mutex<bool>>,
}

impl CronManager {
    pub fn new<F, Fut>(callback: F) -> Self
    where
        F: Fn(&ScheduledTask) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String>> + Send + 'static,
    {
        let callback: Arc<TaskCallback> = Arc::new(move |task| Box::pin(callback(task)));
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            callback,
            running: Arc::new(Mutex::new(false)),
        }
    }
    
    pub fn add_task(&self, task: ScheduledTask) -> Result<()> {
        let mut tasks = self.tasks.lock().map_err(|e| anyhow::anyhow!("Failed to lock tasks: {}", e))?;
        tasks.insert(task.id.clone(), task);
        Ok(())
    }
    
    pub fn remove_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.lock().map_err(|e| anyhow::anyhow!("Failed to lock tasks: {}", e))?;
        tasks.remove(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;
        Ok(())
    }
    
    pub fn update_task(&self, task: ScheduledTask) -> Result<()> {
        let mut tasks = self.tasks.lock().map_err(|e| anyhow::anyhow!("Failed to lock tasks: {}", e))?;
        tasks.insert(task.id.clone(), task);
        Ok(())
    }
    
    pub fn get_task(&self, task_id: &str) -> Option<ScheduledTask> {
        let tasks = self.tasks.lock().ok()?;
        tasks.get(task_id).cloned()
    }
    
    pub fn list_tasks(&self) -> Vec<ScheduledTask> {
        if let Ok(tasks) = self.tasks.lock() {
            tasks.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn start(&self) {
        let running = Arc::clone(&self.running);
        let tasks = Arc::clone(&self.tasks);
        let callback = Arc::clone(&self.callback);
        
        task::spawn(async move {
            *running.lock().unwrap() = true;
            
            while *running.lock().unwrap() {
                let now = Local::now();
                
                let current_tasks = {
                    let t = tasks.lock().unwrap();
                    t.values().filter(|t| t.enabled).cloned().collect::<Vec<_>>()
                };
                
                for task in current_tasks {
                    if task.schedule.matches(&now) {
                        if let Some(last_run) = task.last_run {
                            let since_last = now.signed_duration_since(last_run);
                            if since_last.num_seconds() < 59 {
                                continue;
                            }
                        }
                        
                        let result = match callback(&task).await {
                            Ok(r) => {
                                let mut t = tasks.lock().unwrap();
                                if let Some(task_entry) = t.get_mut(&task.id) {
                                    task_entry.last_run = Some(now);
                                    task_entry.run_count += 1;
                                    task_entry.last_result = Some(r.clone());
                                }
                                TaskExecutionResult {
                                    task_id: task.id.clone(),
                                    success: true,
                                    result: r,
                                    executed_at: now,
                                }
                            }
                            Err(e) => {
                                let mut t = tasks.lock().unwrap();
                                if let Some(task_entry) = t.get_mut(&task.id) {
                                    task_entry.last_run = Some(now);
                                    task_entry.run_count += 1;
                                    task_entry.last_result = Some(format!("Error: {}", e));
                                }
                                TaskExecutionResult {
                                    task_id: task.id.clone(),
                                    success: false,
                                    result: format!("Error: {}", e),
                                    executed_at: now,
                                }
                            }
                        };
                        
                        tracing::info!("Scheduled task '{}' executed: success={}, result={}", 
                            result.task_id, result.success, result.result);
                    }
                }
                
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }
    
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
    
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cron_parse() {
        let schedule = CronSchedule::parse("0 9 * * 1-5").unwrap();
        assert_eq!(schedule.minute, "0");
        assert_eq!(schedule.hour, "9");
        assert_eq!(schedule.day, "*");
        assert_eq!(schedule.month, "*");
        assert_eq!(schedule.weekday, "1-5");
    }
    
    #[test]
    fn test_cron_matches() {
        let schedule = CronSchedule {
            minute: "0".to_string(),
            hour: "9".to_string(),
            day: "*".to_string(),
            month: "*".to_string(),
            weekday: "1-5".to_string(),
        };
        
        let mut now = Local::now();
        assert_eq!(schedule.matches(&now), false);
    }
}
