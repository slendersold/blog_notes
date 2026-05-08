use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePostRequest {
    pub title: String,
    pub content: String,
}

impl Post {
    pub fn new(id: i64, author_id: i64, title: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            content,
            author_id,
            created_at: now,
            updated_at: now,
        }
    }
}
