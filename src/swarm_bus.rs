use tokio::sync::broadcast;
use anyhow::Result;
use crate::models::Letter;
use std::collections::HashMap;

/// SwarmBus provides real-time pub/sub communication between agents.
///
/// Each task gets a broadcast channel. When an agent sends a letter,
/// it is published to the task's channel and immediately received by
/// all other agents subscribed to that task.
///
/// Letters are ALSO persisted to SQLite (via db.rs) for audit trail
/// and dashboard display. The broadcast channel is the hot path;
/// SQLite is the cold storage.
pub struct SwarmBus {
    channels: HashMap<String, broadcast::Sender<Letter>>,
}

impl SwarmBus {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Create a broadcast channel for a task.
    pub fn create_task_channel(&mut self, task_id: &str, capacity: usize) {
        let (tx, _rx) = broadcast::channel(capacity);
        self.channels.insert(task_id.to_string(), tx);
    }

    /// Publish a letter to all agents in a task.
    pub fn publish(&self, task_id: &str, letter: Letter) -> Result<()> {
        if let Some(tx) = self.channels.get(task_id) {
            let _ = tx.send(letter); // broadcast to all subscribers; ignore inactive receivers
        }
        Ok(())
    }

    /// Subscribe to a task's letter stream.
    pub fn subscribe(&self, task_id: &str) -> Option<broadcast::Receiver<Letter>> {
        self.channels.get(task_id).map(|tx| tx.subscribe())
    }

    /// List all active task channel IDs.
    pub fn list_channels(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
    }

    /// Remove a task channel when the task completes.
    pub fn close_task_channel(&mut self, task_id: &str) {
        self.channels.remove(task_id);
    }

    /// Check if a task channel exists.
    pub fn has_channel(&self, task_id: &str) -> bool {
        self.channels.contains_key(task_id)
    }
}

/// A LetterStream wraps a broadcast receiver and provides
/// convenience methods for reading peer letters.
pub struct LetterStream {
    receiver: broadcast::Receiver<Letter>,
    buffer: Vec<Letter>,
}

impl LetterStream {
    pub fn new(receiver: broadcast::Receiver<Letter>) -> Self {
        Self {
            receiver,
            buffer: Vec::new(),
        }
    }

    /// Try to receive a letter without blocking.
    pub fn try_recv(&mut self) -> Option<Letter> {
        match self.receiver.try_recv() {
            Ok(letter) => {
                self.buffer.push(letter.clone());
                Some(letter)
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => None,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // If we lagged, read what we can
                self.drain_pending();
                None
            }
        }
    }

    /// Drain all pending letters into the buffer.
    pub fn drain_pending(&mut self) {
        while let Ok(letter) = self.receiver.try_recv() {
            self.buffer.push(letter);
        }
    }

    /// Get all letters received so far (excluding those from a specific persona).
    pub fn peer_letters(&self, exclude_persona: &str) -> Vec<&Letter> {
        self.buffer
            .iter()
            .filter(|l| l.from_persona != exclude_persona)
            .collect()
    }

    /// Get the last N letters from the buffer.
    pub fn recent_letters(&self, n: usize) -> Vec<&Letter> {
        self.buffer.iter().rev().take(n).collect()
    }
}
