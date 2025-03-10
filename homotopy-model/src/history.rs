use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use homotopy_common::tree::{Node, NodeData, Tree};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::proof::ProofState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Move(Direction),
    // TODO: history pruning
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Linear(homotopy_core::Direction),
    // TODO: branch moves
}

#[derive(Clone, Eq, PartialEq, Default)]
pub struct Snapshot {
    proof: ProofState,
    action: Option<super::proof::Action>,
}

impl Deref for Snapshot {
    type Target = ProofState;

    fn deref(&self) -> &Self::Target {
        &self.proof
    }
}

impl DerefMut for Snapshot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.proof
    }
}

pub type Proof = NodeData<Snapshot>;

pub trait UndoState {
    fn can_undo(&self) -> bool;

    fn can_redo(&self) -> bool;

    fn can_move(&self, dir: &Direction) -> bool {
        match dir {
            Direction::Linear(homotopy_core::Direction::Forward) => self.can_redo(),
            Direction::Linear(homotopy_core::Direction::Backward) => self.can_undo(),
        }
    }
}

impl UndoState for Proof {
    fn can_undo(&self) -> bool {
        self.parent().is_some()
    }

    fn can_redo(&self) -> bool {
        !self.is_empty()
    }
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.action)
    }
}

impl Snapshot {
    fn new(action: Option<super::proof::Action>, proof: ProofState) -> Self {
        Self { proof, action }
    }
}

#[derive(Debug, Clone)]
pub struct History {
    snapshots: Tree<Snapshot>,
    current: Node,
}

impl Default for History {
    fn default() -> Self {
        let snapshots = Tree::<Snapshot>::default();
        let current = snapshots.root();
        Self { snapshots, current }
    }
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("error while performing undo")]
    Undo,
    #[error("error while performing redo")]
    Redo,
}

impl History {
    pub fn proof(&self) -> &Proof {
        &self.snapshots[self.current]
    }

    pub fn add(&mut self, action: super::proof::Action, proof: Proof) {
        if let Some(child) = self.snapshots.push_onto(
            self.current,
            Snapshot::new(Some(action), proof.into_inner().proof),
        ) {
            self.current = child;
        }
    }

    pub fn undo(&mut self) -> Result<(), HistoryError> {
        let prev = self.proof().parent().ok_or(HistoryError::Undo)?;
        self.current = prev;
        Ok(())
    }

    pub fn redo(&mut self) -> Result<(), HistoryError> {
        let next = self.proof().last().ok_or(HistoryError::Redo)?;
        self.current = next;
        Ok(())
    }

    pub fn try_redo(&mut self, action: &super::proof::Action) -> Result<(), HistoryError> {
        let next = self
            .proof()
            .children()
            .find(|id| {
                self.snapshots
                    .with(*id, |n| n.action.as_ref() == Some(action))
                    .unwrap_or_default()
            })
            .ok_or(HistoryError::Redo)?;
        self.current = next;
        Ok(())
    }

    pub fn get_actions(&self) -> Vec<super::proof::Action> {
        let mut actions: Vec<_> = self
            .snapshots
            .ancestors_of(self.current)
            .filter_map(|n| self.snapshots.with(n, |s| s.action.clone()).flatten())
            .collect();
        actions.reverse();
        actions
    }

    pub fn get_last_import_segment(&self) -> Vec<super::proof::Action> {
        let mut actions = Vec::new();
        for a in self
            .snapshots
            .ancestors_of(self.current)
            .filter_map(|n| self.snapshots.with(n, |s| s.action.clone()).flatten())
        {
            actions.push(a.clone());
            if matches!(a, super::proof::Action::ImportProof(_)) {
                break;
            }
        }
        actions.reverse();
        actions
    }

    pub fn last_action(&self) -> Option<super::proof::Action> {
        self.snapshots
            .with(self.current, |s| s.action.clone())
            .flatten()
    }
}
