use std::sync::mpsc;

use super::{action::Action, component::FmtDuration, thread as bg_thread};

/// Terminal output mode.
#[derive(Debug, Copy, Clone)]
pub enum Mode {
    /// Default mode with enabled terminal rendering.
    Enabled,
    /// No output mode.
    Disabled,
}

pub struct TerminalHandle(Option<TerminalHandleInner>);

struct TerminalHandleInner {
    thread: bg_thread::ThreadHandle,
    ctx_sender: Option<mpsc::Sender<()>>,
}

impl Default for TerminalHandleInner {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalHandleInner {
    pub fn new() -> Self {
        Self {
            thread: bg_thread::ThreadHandle::new(),
            ctx_sender: None,
        }
    }
}

impl TerminalHandle {
    pub fn new(mode: Mode) -> Self {
        match mode {
            Mode::Enabled => Self::new_enabled(),
            Mode::Disabled => Self(None),
        }
    }

    pub fn new_enabled() -> Self {
        if superconsole::SuperConsole::compatible() {
            Self(TerminalHandleInner::new().into())
        } else {
            Self(None)
        }
    }

    pub fn context(&mut self, step_header: &'static str) -> TerminalContext<'_> {
        let _ = self.0.as_mut().map(|handle| handle.ctx_sender.take());
        let action = if self.0.is_some() {
            Some(Action {
                step_header,
                ..Default::default()
            })
        } else {
            None
        };

        TerminalContext {
            term: self,
            action,
            steps_left: 1,
        }
    }
}

pub struct TerminalContext<'a> {
    term: &'a mut TerminalHandle,
    action: Option<Action>,
    steps_left: usize,
}

impl TerminalContext<'_> {
    pub fn with_loading_bar(self, loading_header: &'static str) -> Self {
        Self {
            action: self.action.map(|action| Action {
                loading_bar_header: Some(loading_header),
                ..action
            }),
            ..self
        }
    }

    pub fn num_steps(self, num_steps: usize) -> Self {
        assert!(num_steps > 0);
        Self {
            action: self.action.map(|action| Action {
                iter_num: num_steps,
                ..action
            }),
            steps_left: num_steps,
            ..self
        }
    }

    pub fn on_step<F>(self, on_step: F) -> Self
    where
        F: Fn(usize) -> String + Send + 'static,
    {
        Self {
            action: self.action.map(|action| Action {
                step_trailing: Box::new(on_step),
                ..action
            }),
            ..self
        }
    }

    pub fn completion_header(self, completion_header: &'static str) -> Self {
        Self {
            action: self.action.map(|action| Action {
                completion_header,
                ..action
            }),
            ..self
        }
    }

    pub fn completion_stats<F>(self, on_completion: F) -> Self
    where
        F: Fn(FmtDuration) -> String + Send + 'static,
    {
        Self {
            action: self.action.map(|action| Action {
                completion_trailing: Box::new(on_completion),
                ..action
            }),
            ..self
        }
    }

    pub fn display_step(&mut self) -> Guard<'_> {
        let Some(term) = &mut self.term.0 else {
            return Guard { sender: None };
        };

        self.steps_left = self
            .steps_left
            .checked_sub(1)
            .expect("steps number overflow");
        let ctx_sender = &mut term.ctx_sender;

        let sender = if let Some(action) = self.action.take() {
            ctx_sender.get_or_insert_with(|| {
                let (tx, rx) = mpsc::channel();
                let _ = term.thread.sender().send((action, rx));
                tx
            })
        } else {
            let tx = ctx_sender.as_ref().unwrap();
            let _ = tx.send(());
            tx
        }
        .into();

        Guard { sender }
    }
}

#[derive(Default)]
pub struct Guard<'a> {
    sender: Option<&'a mpsc::Sender<()>>,
}

impl Guard<'_> {
    pub fn abort(mut self) {
        let _ = self.sender.take();
    }
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        if let Some(sender) = self.sender {
            let _ = sender.send(());
        }
    }
}
