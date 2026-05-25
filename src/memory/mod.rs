use crate::core::Message;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::VecDeque;

const DEFAULT_SHORT_TERM_CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    messages: VecDeque<Message>,
    capacity: usize,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ShortTermMemory {
    pub fn new(capacity: Option<usize>) -> Self {
        let capacity = capacity.unwrap_or(DEFAULT_SHORT_TERM_CAPACITY);
        let now = Utc::now();
        Self {
            messages: VecDeque::with_capacity(capacity),
            capacity,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add(&mut self, message: Message) {
        if self.messages.len() >= self.capacity {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
        self.updated_at = Utc::now();
    }

    pub fn add_many(&mut self, messages: Vec<Message>) {
        for message in messages {
            self.add(message);
        }
    }

    pub fn get_recent(&self, count: usize) -> Vec<Message> {
        self.messages.iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn get_all(&self) -> Vec<Message> {
        self.messages.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        while self.messages.len() > capacity {
            self.messages.pop_front();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryItem {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: u32,
}

impl LongTermMemoryItem {
    pub fn new(content: String, metadata: Option<serde_json::Value>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            metadata: metadata.unwrap_or(serde_json::Value::Null),
            importance: 0.5,
            created_at: now,
            accessed_at: now,
            access_count: 0,
        }
    }

    pub fn access(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count += 1;
    }

    pub fn set_importance(&mut self, importance: f32) {
        self.importance = importance.clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone)]
pub struct LongTermMemory {
    items: Vec<LongTermMemoryItem>,
    capacity: usize,
}

impl LongTermMemory {
    pub fn new(capacity: Option<usize>) -> Self {
        let capacity = capacity.unwrap_or(1000);
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn add(&mut self, item: LongTermMemoryItem) {
        if self.items.len() >= self.capacity {
            self.consolidate();
        }
        self.items.push(item);
    }

    pub fn add_content(&mut self, content: String, metadata: Option<serde_json::Value>) {
        let item = LongTermMemoryItem::new(content, metadata);
        self.add(item);
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<&LongTermMemoryItem> {
        let query_lower = query.to_lowercase();
        
        let mut scored_items: Vec<_> = self.items.iter()
            .map(|item| {
                let relevance = self.calculate_relevance(&item.content, &query_lower);
                let score = relevance * 0.6 + item.importance * 0.3 + (item.access_count as f32) * 0.1;
                (item, score)
            })
            .filter(|(_, score)| *score > 0.1)
            .collect();
        
        scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        scored_items.into_iter()
            .take(limit)
            .map(|(item, _)| item)
            .collect()
    }

    fn calculate_relevance(&self, content: &str, query: &str) -> f32 {
        let content_lower = content.to_lowercase();
        
        let query_words: Vec<&str> = query.split_whitespace().collect();
        if query_words.is_empty() {
            return 0.0;
        }

        let matching_words = query_words.iter()
            .filter(|&&word| content_lower.contains(word))
            .count();
        
        matching_words as f32 / query_words.len() as f32
    }

    pub fn consolidate(&mut self) {
        if self.items.len() <= self.capacity {
            return;
        }

        let mut scored_items: Vec<_> = self.items.drain(..).map(|item| {
            let score = Self::calculate_item_score_static(&item);
            (item, score)
        }).collect();

        scored_items.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        let remove_count = scored_items.len() - self.capacity / 2;
        scored_items.truncate(self.capacity - remove_count);
        
        self.items = scored_items.into_iter().map(|(item, _)| item).collect();
    }

    fn calculate_item_score_static(item: &LongTermMemoryItem) -> f32 {
        let recency_factor = {
            let age = Utc::now().signed_duration_since(item.created_at).num_seconds() as f32;
            let days = age / 86400.0;
            (-days * 0.1).exp()
        };

        let importance = item.importance;
        let access_factor = (item.access_count as f32).ln() * 0.2;

        recency_factor * 0.4 + importance * 0.4 + access_factor * 0.2
    }

    pub fn get(&self, id: &str) -> Option<&LongTermMemoryItem> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut LongTermMemoryItem> {
        self.items.iter_mut().find(|item| item.id == id)
    }

    pub fn remove(&mut self, id: &str) -> Option<LongTermMemoryItem> {
        if let Some(pos) = self.items.iter().position(|item| item.id == id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn get_all(&self) -> Vec<&LongTermMemoryItem> {
        self.items.iter().collect()
    }
}

#[derive(Debug, Clone)]
pub struct MemoryManager {
    pub short_term: ShortTermMemory,
    pub long_term: LongTermMemory,
}

impl MemoryManager {
    pub fn new(short_term_capacity: Option<usize>, long_term_capacity: Option<usize>) -> Self {
        Self {
            short_term: ShortTermMemory::new(short_term_capacity),
            long_term: LongTermMemory::new(long_term_capacity),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.short_term.add(message.clone());
        
        if message.content.len() > 50 {
            self.long_term.add_content(
                format!("Conversation: {}", message.content),
                Some(serde_json::json!({
                    "type": "conversation",
                    "role": format!("{:?}", message.role),
                    "timestamp": message.timestamp.to_rfc3339()
                }))
            );
        }
    }

    pub fn add_message_batch(&mut self, messages: Vec<Message>) {
        for message in messages {
            self.add_message(message);
        }
    }

    pub fn get_context(&self, recent_count: usize, long_term_limit: usize) -> (Vec<Message>, Vec<&LongTermMemoryItem>) {
        let recent_messages = self.short_term.get_recent(recent_count);
        let long_term_items = self.long_term.get_all();
        
        (recent_messages, long_term_items.into_iter().take(long_term_limit).collect())
    }

    pub fn get_recent_messages(&self, count: usize) -> Vec<Message> {
        self.short_term.get_recent(count)
    }

    pub fn search_long_term(&self, query: &str, limit: usize) -> Vec<&LongTermMemoryItem> {
        self.long_term.search(query, limit)
    }

    pub fn consolidate_memory(&mut self) {
        self.long_term.consolidate();
    }
}
