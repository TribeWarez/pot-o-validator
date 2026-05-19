//! mDNS (Multicast DNS) service discovery for PoT-O validators.
//!
//! Enables validators on the same local network to automatically discover each other
//! without requiring a Bootstrap Registry. Validators register themselves as mDNS services
//! and can discover peers via mDNS queries.

use flume::RecvTimeoutError;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Information about a discovered peer validator
#[derive(Clone, Debug)]
pub struct PeerDiscovery {
    /// Unique identifier for the validator node
    pub node_id: String,
    /// Hostname (FQDN) of the validator
    pub hostname: String,
    /// IP address of the validator
    pub ip: IpAddr,
    /// Port on which the validator is listening
    pub port: u16,
}

/// mDNS service discovery for validator peer discovery
pub struct MdnsDiscovery {
    /// Unique identifier for this validator
    node_id: String,
    /// Port on which this validator listens
    port: u16,
    /// mDNS service type for PoT-O validators
    service_type: String,
    /// mDNS daemon for service registration and discovery
    daemon: Arc<Mutex<Option<ServiceDaemon>>>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl MdnsDiscovery {
    /// Service type string for PoT-O validators following mDNS conventions
    const SERVICE_TYPE: &'static str = "_pot-o-validator._tcp.local.";

    /// Create a new mDNS discovery instance and start the daemon.
    ///
    /// # Arguments
    /// * `node_id` - Unique identifier for this validator
    /// * `port` - Port on which this validator listens
    ///
    /// # Returns
    /// Result with MdnsDiscovery instance or error if daemon fails to start
    pub fn new(node_id: &str, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        // Create and start the mDNS daemon
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;

        Ok(Self {
            node_id: node_id.to_string(),
            port,
            service_type: Self::SERVICE_TYPE.to_string(),
            daemon: Arc::new(Mutex::new(Some(daemon))),
        })
    }

    /// Get the node ID of this validator
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Get the port of this validator
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the mDNS service type
    pub fn service_type(&self) -> &str {
        &self.service_type
    }

    /// Register this validator as an mDNS service.
    ///
    /// # Arguments
    /// * `hostname` - Hostname/FQDN for this validator (e.g., "validator-1.local")
    ///
    /// # Returns
    /// Result indicating success or error
    pub fn register_service(&self, hostname: &str) -> Result<(), Box<dyn std::error::Error>> {
        let daemon_lock = self
            .daemon
            .lock()
            .map_err(|e| format!("Failed to lock daemon: {}", e))?;

        let daemon = daemon_lock.as_ref().ok_or("mDNS daemon not initialized")?;

        // Create service info with validator details
        let service_info = ServiceInfo::new(
            Self::SERVICE_TYPE,
            &format!("{}._pot-o-validator._tcp.local.", self.node_id),
            &self.node_id,
            hostname,
            self.port,
            None,
        )
        .map_err(|e| format!("Failed to create ServiceInfo: {}", e))?;

        // Register the service with the daemon
        daemon
            .register(service_info)
            .map_err(|e| format!("Failed to register service: {}", e))?;

        Ok(())
    }

    /// Discover other validators on the local network via mDNS.
    ///
    /// # Arguments
    /// * `timeout_secs` - How long to wait for discovery responses
    ///
    /// # Returns
    /// Result with vector of discovered PeerDiscovery instances
    pub fn discover_peers(
        &self,
        timeout_secs: u64,
    ) -> Result<Vec<PeerDiscovery>, Box<dyn std::error::Error>> {
        let daemon_lock = self
            .daemon
            .lock()
            .map_err(|e| format!("Failed to lock daemon: {}", e))?;

        let daemon = daemon_lock.as_ref().ok_or("mDNS daemon not initialized")?;

        // Browse for all PoT-O validator services
        let receiver = daemon
            .browse(Self::SERVICE_TYPE)
            .map_err(|e| format!("Failed to browse services: {}", e))?;

        let mut peers = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);

        // Collect discovery events until timeout
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                    // Extract peer information from resolved service
                    if let Some(peer) = self.extract_peer_from_service(&info) {
                        // Avoid adding ourselves to the peer list
                        if peer.node_id != self.node_id {
                            peers.push(peer);
                        }
                    }
                }
                Ok(mdns_sd::ServiceEvent::ServiceRemoved(_, _)) => {
                    // Service was removed, could filter from peers if we track them
                }
                Ok(mdns_sd::ServiceEvent::SearchStarted(_)) => {
                    // Search has started, ignore
                }
                Ok(mdns_sd::ServiceEvent::ServiceFound(_, _)) => {
                    // Service found but not yet resolved, ignore (we'll get ServiceResolved)
                }
                Ok(mdns_sd::ServiceEvent::SearchStopped(_)) => {
                    // Search stopped, could exit discovery here
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    break; // Timeout reached, exit discovery
                }
                Err(_) => {
                    // Other receive errors, exit discovery
                    break;
                }
            }
        }

        Ok(peers)
    }

    /// Unregister this validator from mDNS (cleanup).
    ///
    /// # Returns
    /// Result indicating success or error
    pub fn unregister_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let daemon_lock = self
            .daemon
            .lock()
            .map_err(|e| format!("Failed to lock daemon: {}", e))?;

        let daemon = daemon_lock.as_ref().ok_or("mDNS daemon not initialized")?;

        let service_name = format!("{}._pot-o-validator._tcp.local.", self.node_id);
        daemon
            .unregister(&service_name)
            .map_err(|e| format!("Failed to unregister service: {}", e))?;

        Ok(())
    }

    /// Extract peer discovery information from an mDNS ServiceInfo.
    fn extract_peer_from_service(&self, info: &ServiceInfo) -> Option<PeerDiscovery> {
        // Get the fullname (e.g., "validator-1._pot-o-validator._tcp.local.")
        let fullname = info.get_fullname();

        // Extract node_id by removing the service type suffix
        let node_id = fullname
            .strip_suffix("._pot-o-validator._tcp.local.")?
            .to_string();

        // Get hostname
        let hostname = info.get_hostname().to_string();

        // Get IP address (first available IPv4)
        let ip = info
            .get_addresses()
            .iter()
            .next()
            .map(|ipv4| IpAddr::V4(*ipv4))?;

        let port = info.get_port();

        Some(PeerDiscovery {
            node_id,
            hostname,
            ip,
            port,
        })
    }
}

// Ensure MdnsDiscovery is Send + Sync for use in async contexts
#[allow(dead_code)]
fn _assert_send_sync()
where
    MdnsDiscovery: Send + Sync,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type_constant() {
        assert_eq!(MdnsDiscovery::SERVICE_TYPE, "_pot-o-validator._tcp.local.");
    }

    #[test]
    fn test_peer_discovery_creation() {
        let peer = PeerDiscovery {
            node_id: "test-node".to_string(),
            hostname: "test-node.local".to_string(),
            ip: "127.0.0.1".parse().unwrap(),
            port: 5555,
        };

        assert_eq!(peer.node_id, "test-node");
        assert_eq!(peer.port, 5555);
    }
}
