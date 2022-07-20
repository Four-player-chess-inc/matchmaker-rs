use tokio::time::{Duration, Instant};

type Since = Instant;
type IsConfirm = bool;

#[derive(PartialEq, Debug)]
pub(crate) enum State {
    WaitForQuorum,
    ToRemove,
    WaitConfirm((Since, IsConfirm)),
}

impl State {
    pub(crate) fn is_wait_for_quorum(&self) -> bool {
        *self == State::WaitForQuorum
    }

    pub(crate) fn wait_for_quorum(&mut self) {
        *self = State::WaitForQuorum
    }

    pub(crate) fn is_to_remove(&self) -> bool {
        *self == State::ToRemove
    }

    pub(crate) fn to_remove(&mut self) {
        *self = State::ToRemove;
    }

    pub(crate) fn wait_confirm(&mut self, i: Instant) {
        *self = State::WaitConfirm((i, false));
    }

    pub(crate) fn is_confirm_failed(&self, now: Instant, timeout: Duration) -> bool {
        if let State::WaitConfirm((since, confirmed)) = *self {
            return !confirmed && ((now - since) > timeout);
        }
        false
    }

    pub(crate) fn is_confirm_timeout(&self, now: Instant, timeout: Duration) -> bool {
        if let State::WaitConfirm((since, confirmed)) = *self {
            return confirmed && ((now - since) > timeout);
        }
        false
    }

    pub(crate) fn is_confirm_success(&self, now: Instant, timeout: Duration) -> bool {
        if let State::WaitConfirm((since, confirmed)) = *self {
            return confirmed && ((now - since) <= timeout);
        }
        false
    }

    pub(crate) fn wait_confirm_if_actual(&mut self, timeout: Duration) {
        if let State::WaitConfirm((since, _)) = *self {
            if (Instant::now() - since) <= timeout {
                *self = State::WaitConfirm((since, true))
            }
        }
    }
}
