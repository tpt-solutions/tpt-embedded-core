//! Core state machine definitions.

/// The role of a node in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Primary coordinator.
    Primary,
    /// Secondary (backup) node.
    Secondary,
    /// Uninitialised node.
    Unknown,
}
