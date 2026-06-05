//! Internal gRPC client for forwarding gateway requests to the
//! Agent Nervous System daemon on port 50051.

use ans_proto::ans::agent_nervous_system_client::AgentNervousSystemClient;
use tonic::transport::Channel;

/// Wrapper around the internal gRPC client.
///
/// Created once at gateway startup. All protocol adapters (MCP, REST,
/// WebSocket) share this client to forward requests to the daemon.
#[derive(Clone)]
pub struct InternalClient {
    inner: AgentNervousSystemClient<Channel>,
}

impl InternalClient {
    /// Connect to the local gRPC server.
    pub async fn connect(grpc_port: u16) -> Result<Self, tonic::transport::Error> {
        let addr: &'static str = Box::leak(format!("http://127.0.0.1:{grpc_port}").into_boxed_str());
        let channel = Channel::from_static(addr)
            .connect()
            .await?;
        Ok(Self {
            inner: AgentNervousSystemClient::new(channel),
        })
    }

    /// Reference to the underlying tonic client.
    pub const fn inner(&self) -> &AgentNervousSystemClient<Channel> {
        &self.inner
    }

    /// Mutable reference for streaming calls.
    pub const fn inner_mut(&mut self) -> &mut AgentNervousSystemClient<Channel> {
        &mut self.inner
    }

    /// Clone the underlying client for use in handlers that have `&self`.
    pub fn clone_client(&self) -> AgentNervousSystemClient<Channel> {
        self.inner.clone()
    }
}
