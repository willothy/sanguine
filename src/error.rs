//! Error handling

use std::fmt::Display;

use crate::layout::{NodeId, WidgetId};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    External(String),
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
    #[error("Widget not found: {0:?}")]
    WidgetNotFound(WidgetId),
    #[error("Signal send failed")]
    SignalSendFail,
    #[error("Could not acquire node read lock for {0:?}")]
    NodeReadLockError(NodeId),
    #[error("Could not acquire node write lock for {0:?}")]
    NodeWriteLockError(NodeId),
    #[error("Could not acquire widget write lock for {0:?}")]
    WidgetWriteLockError(WidgetId),
    #[error("Failed to poll input")]
    PollInputFailed,
    #[error("Expected node {0:?} to be a leaf")]
    ExpectedLeaf(NodeId),
    #[error("Failed to flush terminal")]
    TerminalError,
    #[error("No focused window")]
    NoFocus,
}

impl Error {
    pub fn external(msg: impl Display) -> Self {
        Self::External(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
