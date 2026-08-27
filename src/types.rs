use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub note: String,
    pub position: usize,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "startedAt")]
    pub started_at: Option<u64>,
    #[serde(rename = "doneAt")]
    pub done_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueState {
    pub version: u8,
    pub current: Option<String>,
    pub tasks: Vec<Task>,
}

impl QueueState {
    pub fn empty() -> Self {
        QueueState {
            version: 1,
            current: None,
            tasks: Vec::new(),
        }
    }
}
