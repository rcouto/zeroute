//! Readiness observations for one connection attempt.
//!
//! These facts describe the VPN transport. Reachability of an individual peer
//! remains a separate observation, even when the transport is ready.

/// The next unmet prerequisite, in connection order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Readiness {
    Stopped,
    WaitingForEngine,
    WaitingForConfiguration,
    WaitingForTunnel,
    WaitingForDns,
    Ready,
}

/// Reports from the engine and Android adapter for the current attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observation {
    EngineRunning(bool),
    ControlPlaneOnline(bool),
    NetworkConfigured(bool),
    TunnelEstablished(bool),
    DnsReady(bool),
}

/// Attempt identifier used to reject callbacks from an earlier connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attempt(u64);

/// Rust-owned state; this model performs no networking or Android API calls.
#[derive(Debug, Default)]
pub struct Connection {
    generation: u64,
    active: bool,
    engine: bool,
    control_plane_online: bool,
    configuration: bool,
    tunnel: bool,
    dns: bool,
}

impl Readiness {
    /// Stable JNI state code; changes require an ABI version update.
    pub fn code(self) -> i32 {
        match self {
            Self::Stopped => 0,
            Self::WaitingForEngine => 1,
            Self::WaitingForConfiguration => 2,
            Self::WaitingForTunnel => 3,
            Self::WaitingForDns => 4,
            Self::Ready => 5,
        }
    }
}

impl Connection {
    /// Begin a fresh attempt, discarding observations from the previous one.
    pub fn start(&mut self) -> Attempt {
        self.stop();
        self.generation = self
            .generation
            .checked_add(1)
            .expect("attempt counter exhausted");
        self.active = true;
        Attempt(self.generation)
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.engine = false;
        self.control_plane_online = false;
        self.configuration = false;
        self.tunnel = false;
        self.dns = false;
    }

    /// Return false for a stopped or obsolete attempt. Events that assert a
    /// downstream prerequisite too early are also ignored, requiring a fresh
    /// observation after its prerequisite becomes available.
    pub fn accepts(&self, attempt: Attempt) -> bool {
        self.active && attempt == Attempt(self.generation)
    }

    pub fn observe(&mut self, attempt: Attempt, observation: Observation) -> bool {
        if !self.accepts(attempt) {
            return false;
        }
        match observation {
            Observation::ControlPlaneOnline(online) => {
                self.control_plane_online = online && self.engine;
            }
            Observation::EngineRunning(online) => {
                self.engine = online;
                if !online {
                    self.control_plane_online = false;
                    self.configuration = false;
                    self.tunnel = false;
                    self.dns = false;
                }
            }
            Observation::NetworkConfigured(configured) => {
                if configured && !self.engine {
                    return false;
                }
                self.configuration = configured;
                if !configured {
                    self.tunnel = false;
                    self.dns = false;
                }
            }
            Observation::TunnelEstablished(established) => {
                if established && !self.configuration {
                    return false;
                }
                self.tunnel = established;
                if !established {
                    self.dns = false;
                }
            }
            Observation::DnsReady(ready) => {
                if ready && !self.tunnel {
                    return false;
                }
                self.dns = ready;
            }
        }
        true
    }

    /// Root connectivity is diagnostic; direct peer traffic can outlive it.
    pub fn control_plane_online(&self) -> bool {
        self.active && self.engine && self.control_plane_online
    }

    /// IP transport can be available while hostname resolution is still pending.
    pub fn ip_transport_ready(&self) -> bool {
        self.active && self.engine && self.configuration && self.tunnel
    }

    pub fn readiness(&self) -> Readiness {
        if !self.active {
            Readiness::Stopped
        } else if !self.engine {
            Readiness::WaitingForEngine
        } else if !self.configuration {
            Readiness::WaitingForConfiguration
        } else if !self.tunnel {
            Readiness::WaitingForTunnel
        } else if !self.dns {
            Readiness::WaitingForDns
        } else {
            Readiness::Ready
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Connection, Observation, Readiness};

    #[test]
    fn engine_running_does_not_mean_vpn_ready() {
        let mut connection = Connection::default();
        assert_eq!(connection.readiness(), Readiness::Stopped);
        let attempt = connection.start();
        assert_eq!(connection.readiness(), Readiness::WaitingForEngine);
        assert!(connection.observe(attempt, Observation::EngineRunning(true)));
        assert_eq!(connection.readiness(), Readiness::WaitingForConfiguration);
        assert!(!connection.ip_transport_ready());
        connection.observe(attempt, Observation::NetworkConfigured(true));
        assert_eq!(connection.readiness(), Readiness::WaitingForTunnel);
        connection.observe(attempt, Observation::TunnelEstablished(true));
        assert_eq!(connection.readiness(), Readiness::WaitingForDns);
        assert!(connection.ip_transport_ready());
        connection.observe(attempt, Observation::DnsReady(true));
        assert_eq!(connection.readiness(), Readiness::Ready);
        connection.observe(attempt, Observation::DnsReady(false));
        assert_eq!(connection.readiness(), Readiness::WaitingForDns);
        assert!(connection.ip_transport_ready());
    }

    #[test]
    fn reconnect_discards_old_callbacks_and_cached_readiness() {
        let mut connection = Connection::default();
        let old = connection.start();
        connection.observe(old, Observation::EngineRunning(true));
        connection.observe(old, Observation::NetworkConfigured(true));
        connection.observe(old, Observation::TunnelEstablished(true));
        connection.observe(old, Observation::DnsReady(true));
        let current = connection.start();
        assert_eq!(connection.readiness(), Readiness::WaitingForEngine);
        assert!(!connection.observe(old, Observation::DnsReady(true)));
        assert!(!connection.observe(old, Observation::EngineRunning(true)));
        connection.observe(current, Observation::EngineRunning(true));
        assert_eq!(connection.readiness(), Readiness::WaitingForConfiguration);
        connection.stop();
        assert!(!connection.observe(current, Observation::EngineRunning(true)));
        assert_eq!(connection.readiness(), Readiness::Stopped);
    }

    #[test]
    fn loss_of_a_prerequisite_requires_fresh_downstream_observations() {
        for loss in [
            Observation::EngineRunning(false),
            Observation::NetworkConfigured(false),
            Observation::TunnelEstablished(false),
        ] {
            let mut connection = Connection::default();
            let attempt = connection.start();
            connection.observe(attempt, Observation::EngineRunning(true));
            connection.observe(attempt, Observation::NetworkConfigured(true));
            connection.observe(attempt, Observation::TunnelEstablished(true));
            connection.observe(attempt, Observation::DnsReady(true));
            connection.observe(attempt, loss);
            assert!(!connection.ip_transport_ready());
            assert!(!connection.observe(attempt, Observation::DnsReady(true)));
            connection.observe(attempt, Observation::EngineRunning(true));
            connection.observe(attempt, Observation::NetworkConfigured(true));
            connection.observe(attempt, Observation::TunnelEstablished(true));
            assert_eq!(connection.readiness(), Readiness::WaitingForDns);
        }
    }

    #[test]
    fn root_connectivity_loss_does_not_invalidate_a_working_transport() {
        let mut connection = Connection::default();
        let attempt = connection.start();
        connection.observe(attempt, Observation::EngineRunning(true));
        connection.observe(attempt, Observation::ControlPlaneOnline(true));
        connection.observe(attempt, Observation::NetworkConfigured(true));
        connection.observe(attempt, Observation::TunnelEstablished(true));
        connection.observe(attempt, Observation::DnsReady(true));
        assert!(connection.control_plane_online());
        connection.observe(attempt, Observation::ControlPlaneOnline(false));
        assert!(!connection.control_plane_online());
        assert!(connection.ip_transport_ready());
        assert_eq!(connection.readiness(), Readiness::Ready);
        connection.observe(attempt, Observation::EngineRunning(false));
        assert!(!connection.ip_transport_ready());
    }

    #[test]
    fn early_observations_are_not_reused_later() {
        let mut connection = Connection::default();
        let attempt = connection.start();
        assert!(!connection.observe(attempt, Observation::NetworkConfigured(true)));
        assert!(!connection.observe(attempt, Observation::TunnelEstablished(true)));
        assert!(!connection.observe(attempt, Observation::DnsReady(true)));
        connection.observe(attempt, Observation::EngineRunning(true));
        assert_eq!(connection.readiness(), Readiness::WaitingForConfiguration);
    }
}
