mod network;
pub use network::Network;
mod stream;

/// [`NetworkStatus`] Represents status of the Network object.
/// It is returned when the run handle returns from performing an operation.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetworkStatus {
    /// Indicate that the network is active but there was no incoming packets
    ActivePending,
    /// Indicate that the network is still active and currently in the reading state allowing you to poll more often
    ActiveReady,
    /// Indicate that there was an incoming disconnect and the socket has been closed.
    IncomingDisconnect,
    /// Indicate that an outgoing disconnect has been transmited and the socket is closed
    OutgoingDisconnect,
    /// The server did not respond to the ping request and the socket has been closed
    NoPingResp,
}