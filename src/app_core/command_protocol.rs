use std::collections::VecDeque;

use super::ComponentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CommandId(pub(crate) &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandScope {
    App,
    Window,
    Component(ComponentId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandPayload {
    None,
    ControlId(i64),
    Text(String),
    ItemId(i64),
    Paths(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) id: CommandId,
    pub(crate) scope: CommandScope,
    pub(crate) payload: CommandPayload,
}

impl Command {
    pub(crate) fn window(id: CommandId) -> Self {
        Self {
            id,
            scope: CommandScope::Window,
            payload: CommandPayload::None,
        }
    }

    pub(crate) fn window_with_payload(id: CommandId, payload: CommandPayload) -> Self {
        Self {
            id,
            scope: CommandScope::Window,
            payload,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CommandQueue {
    pending: VecDeque<Command>,
}

impl CommandQueue {
    pub(crate) fn push(&mut self, command: Command) {
        self.pending.push_back(command);
    }

    pub(crate) fn pop(&mut self) -> Option<Command> {
        self.pending.pop_front()
    }

    pub(crate) fn len(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}
