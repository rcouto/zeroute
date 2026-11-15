//! User intent and Android lifecycle policy, separate from transport readiness.
use crate::engine;
use crate::readiness::Readiness;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Idle,
    Consent,
    Starting,
    Running,
    Stopping,
    Failed,
}

#[derive(Default)]
struct Session {
    token: u64,
    phase: Phase,
    network: u32,
    network_revision: u64,
    revalidating: bool,
    failure: String,
}
static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(|| Mutex::new(Session::default()));

impl Session {
    fn begin(&mut self) -> u64 {
        if !matches!(self.phase, Phase::Idle | Phase::Failed) {
            return 0;
        }
        self.token = self
            .token
            .checked_add(1)
            .expect("session counter exhausted");
        self.phase = Phase::Consent;
        self.failure.clear();
        self.revalidating = false;
        self.token
    }
    fn event(&mut self, token: u64, event: i32) -> bool {
        if token == 0 || token != self.token {
            return false;
        }
        match (event, self.phase) {
            (1, Phase::Consent) => self.phase = Phase::Starting,
            (2, Phase::Starting) => self.phase = Phase::Running,
            (3, Phase::Consent) => self.phase = Phase::Idle,
            (3, Phase::Starting | Phase::Running) => self.phase = Phase::Stopping,
            (4, Phase::Stopping) => {
                self.phase = if self.failure.is_empty() {
                    Phase::Idle
                } else {
                    Phase::Failed
                };
            }
            (5, Phase::Consent) => self.phase = Phase::Idle,
            _ => return false,
        }
        true
    }
}

pub fn begin() -> u64 {
    SESSION.lock().map(|mut s| s.begin()).unwrap_or(0)
}
pub fn event(token: u64, event: i32) -> bool {
    SESSION.lock().is_ok_and(|mut s| s.event(token, event))
}
pub fn accepts(token: u64, phase: i32) -> bool {
    SESSION
        .lock()
        .is_ok_and(|s| token != 0 && token == s.token && s.phase as i32 == phase)
}
pub fn fail(token: u64, reason: i32) {
    if let Ok(mut s) = SESSION.lock()
        && token != 0
        && token == s.token
        && !matches!(s.phase, Phase::Idle | Phase::Failed)
    {
        if s.failure.is_empty() {
            s.failure = engine::snapshot()
                .ok()
                .and_then(|e| e.error)
                .unwrap_or(match reason {
                    1 => "Could not start the VPN engine.",
                    2 => "Android could not establish the VPN interface.",
                    3 => "VPN permission was revoked or another VPN took over.",
                    4 => "The VPN service stopped unexpectedly.",
                    6 => "Android did not allow the VPN foreground service to start.",
                    _ => "The VPN could not complete shutdown.",
                })
                .into();
        }
        if reason == 5 && !s.failure.ends_with("Shutdown is incomplete.") {
            s.failure.push_str(" Shutdown is incomplete.");
        }
        s.phase = Phase::Stopping;
    }
}
pub fn clear_inactive_failure() {
    if let Ok(mut s) = SESSION.lock()
        && s.phase == Phase::Failed
    {
        s.phase = Phase::Idle;
        s.failure.clear();
    }
}
pub fn network(flags: u32, revision: u64) {
    let refresh = if let Ok(mut s) = SESSION.lock() {
        let changed = s.network & 1 != 0
            && (s.network_revision != revision || (s.network & 114) != (flags & 114));
        s.network = flags;
        s.network_revision = revision;
        let refresh = changed && s.phase == Phase::Running;
        if refresh {
            s.revalidating = true;
        }
        refresh
    } else {
        false
    };
    if refresh {
        engine::refresh_network(revision);
    }
}
pub fn can_edit_settings() -> bool {
    SESSION
        .lock()
        .is_ok_and(|s| matches!(s.phase, Phase::Idle | Phase::Failed))
}

/// Stable, bounded presentation wire format: numeric header, public node ID, error.
/// No configuration, credentials, endpoint addresses or packet contents cross here.
pub fn snapshot() -> Result<String, &'static str> {
    let mut s = SESSION.lock().map_err(|_| "Session state unavailable.")?;
    let e = engine::snapshot()?;
    if s.revalidating && e.proof_revision == s.network_revision && e.readiness == Readiness::Ready {
        s.revalidating = false;
    }
    let mut state = presentation(s.phase, e.readiness, e.dns_configured, e.issue, s.network);
    if s.revalidating && matches!(state, 5..=7) {
        state = 10;
    }
    Ok(format!(
        "{} {} {} {} {} {} {}\n{:010x}\n{}",
        s.token,
        s.phase as u8,
        state,
        e.readiness.code(),
        u8::from(e.dns_configured),
        u8::from(e.control_plane_online),
        s.network,
        e.address,
        if s.failure.is_empty() {
            e.error.unwrap_or("")
        } else {
            &s.failure
        }
    ))
}
fn presentation(phase: Phase, readiness: Readiness, dns: bool, issue: u32, network: u32) -> u8 {
    match phase {
        Phase::Idle => 0,
        Phase::Consent => 1,
        Phase::Starting => 2,
        Phase::Stopping => 13,
        Phase::Failed => 12,
        Phase::Running => {
            if network & 1 != 0 && network & 2 == 0 {
                return 9;
            }
            if readiness == Readiness::Ready {
                return if dns { 6 } else { 7 };
            }
            if issue == 2 {
                return 8;
            }
            if issue == 3 {
                return 12;
            }
            if network & 8 != 0
                && network & 4 == 0
                && !matches!(readiness, Readiness::WaitingForDns)
            {
                return 11;
            }
            match readiness {
                Readiness::Stopped => 12,
                Readiness::WaitingForEngine => 2,
                Readiness::WaitingForConfiguration => 3,
                Readiness::WaitingForTunnel => 4,
                Readiness::WaitingForDns => 5,
                Readiness::Ready => unreachable!(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consent_cancellation_and_stale_results_cannot_start() {
        let mut s = Session::default();
        let old = s.begin();
        assert_eq!(s.begin(), 0);
        assert!(s.event(old, 3));
        assert!(!s.event(old, 1));
        let new = s.begin();
        assert_ne!(old, new);
        assert!(!s.event(old, 1));
        assert!(s.event(new, 1));
    }
    #[test]
    fn stop_waits_for_acknowledgement_and_rejects_late_start() {
        let mut s = Session::default();
        let t = s.begin();
        assert!(s.event(t, 1));
        assert!(s.event(t, 3));
        assert_eq!(s.phase, Phase::Stopping);
        assert_eq!(s.begin(), 0);
        assert!(!s.event(t, 2));
        assert!(s.event(t, 4));
        assert_eq!(s.phase, Phase::Idle);
        assert_ne!(s.begin(), 0);
    }
    #[test]
    fn failures_survive_cleanup_until_explicit_retry() {
        let mut s = Session::default();
        let t = s.begin();
        s.failure = "Original cause".into();
        s.phase = Phase::Stopping;
        assert!(s.event(t, 4));
        assert_eq!(s.phase, Phase::Failed);
        assert_eq!(s.failure, "Original cause");
        assert_ne!(s.begin(), 0);
        assert!(s.failure.is_empty());
    }
    #[test]
    fn network_evidence_does_not_turn_working_private_transport_into_portal_failure() {
        assert_eq!(
            presentation(Phase::Running, Readiness::Ready, true, 0, 1 | 2 | 8),
            6
        );
        assert_eq!(
            presentation(
                Phase::Running,
                Readiness::WaitingForConfiguration,
                false,
                0,
                1 | 2 | 8
            ),
            11
        );
        assert_eq!(
            presentation(
                Phase::Running,
                Readiness::WaitingForConfiguration,
                false,
                0,
                1 | 2
            ),
            3
        );
        assert_eq!(
            presentation(Phase::Running, Readiness::Ready, false, 0, 1 | 2),
            7
        );
        assert_eq!(
            presentation(Phase::Running, Readiness::Ready, true, 0, 1),
            9
        );
    }
}
