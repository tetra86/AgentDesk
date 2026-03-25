use std::fmt;

/// Lifecycle hooks that policies can register handlers for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hook {
    OnSessionStatusChange,
    OnCardTransition,
    OnCardTerminal,
    OnDispatchCompleted,
    OnReviewEnter,
    OnReviewVerdict,
    OnTick,
}

impl Hook {
    /// The JS property name used when registering this hook in a policy object.
    pub fn js_name(&self) -> &'static str {
        match self {
            Hook::OnSessionStatusChange => "onSessionStatusChange",
            Hook::OnCardTransition => "onCardTransition",
            Hook::OnCardTerminal => "onCardTerminal",
            Hook::OnDispatchCompleted => "onDispatchCompleted",
            Hook::OnReviewEnter => "onReviewEnter",
            Hook::OnReviewVerdict => "onReviewVerdict",
            Hook::OnTick => "onTick",
        }
    }

    /// All known hooks.
    pub fn all() -> &'static [Hook] {
        &[
            Hook::OnSessionStatusChange,
            Hook::OnCardTransition,
            Hook::OnCardTerminal,
            Hook::OnDispatchCompleted,
            Hook::OnReviewEnter,
            Hook::OnReviewVerdict,
            Hook::OnTick,
        ]
    }

    /// Parse a hook name string back into a Hook variant.
    pub fn from_str(s: &str) -> Option<Hook> {
        Hook::all().iter().find(|h| h.js_name() == s).copied()
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.js_name())
    }
}
