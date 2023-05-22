//! Error handling

use std::fmt::Display;

use crate::layout::NodeId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    External(String),
    #[error("Widget not found: {0:?}")]
    WidgetNotFound(NodeId),
    #[error("Signal send failed")]
    SignalSendFail,
    #[error("Could not acquire widget read lock for {0:?}")]
    WidgetReadLockError(NodeId),
    #[error("Could not acquire widget write lock for {0:?}")]
    WidgetWriteLockError(NodeId),
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
