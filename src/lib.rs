#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;
#[cfg(test)]
mod test;
pub mod types;

use std::net::SocketAddr;

#[cfg(feature = "client")]
pub use oinq::frame;
#[cfg(any(feature = "client", feature = "server"))]
pub use oinq::message::HandshakeError;
#[cfg(feature = "client")]
pub use oinq::request;
#[cfg(any(feature = "client", feature = "server"))]
use serde::{Deserialize, Serialize};

/// Properties of an agent.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AgentInfo {
    pub app_name: String,
    pub version: String,
    pub protocol_version: String,
    pub addr: SocketAddr,
}

/// Sends a unary request and returns the response.
///
/// # Errors
///
/// Returns an error if there was a problem sending the request or receiving the
/// response.
#[cfg(any(feature = "client", feature = "server"))]
pub async fn unary_request<I, O>(
    send: &mut quinn::SendStream,
    recv: &mut quinn::RecvStream,
    code: u32,
    input: I,
) -> anyhow::Result<O>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned,
{
    use anyhow::Context;

    let mut buf = vec![];
    oinq::message::send_request(send, &mut buf, code, input).await?;

    oinq::frame::recv(recv, &mut buf)
        .await
        .context("invalid response")
}

#[cfg(test)]
mod tests {
    use crate::test::{channel, TOKEN};

    #[cfg(all(feature = "client", feature = "server"))]
    #[tokio::test]
    async fn handshake() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        const APP_NAME: &str = "oinq";
        const APP_VERSION: &str = "1.0.0";
        const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

        let _lock = TOKEN.lock().await;
        let channel = channel().await;
        let (server, client) = (channel.server, channel.client);

        let handle = tokio::spawn(async move {
            super::client::handshake(&client.conn, APP_NAME, APP_VERSION, PROTOCOL_VERSION).await
        });

        let agent_info = super::server::handshake(
            &server.conn,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            PROTOCOL_VERSION,
            PROTOCOL_VERSION,
        )
        .await
        .unwrap();

        assert_eq!(agent_info.app_name, APP_NAME);
        assert_eq!(agent_info.version, APP_VERSION);
        assert_eq!(agent_info.protocol_version, PROTOCOL_VERSION);

        let res = tokio::join!(handle).0.unwrap();
        assert!(res.is_ok());
    }

    #[cfg(all(feature = "client", feature = "server"))]
    #[tokio::test]
    async fn handshake_version_incompatible_err() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        const APP_NAME: &str = "oinq";
        const APP_VERSION: &str = "1.0.0";
        const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

        let _lock = TOKEN.lock().await;
        let channel = channel().await;
        let (server, client) = (channel.server, channel.client);

        let handle = tokio::spawn(async move {
            super::client::handshake(&client.conn, APP_NAME, APP_VERSION, PROTOCOL_VERSION).await
        });

        let res = super::server::handshake(
            &server.conn,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            &format!("<{PROTOCOL_VERSION}"),
            PROTOCOL_VERSION,
        )
        .await;

        assert!(res.is_err());

        let res = tokio::join!(handle).0.unwrap();
        assert!(res.is_err());
    }

    #[cfg(all(feature = "client", feature = "server"))]
    #[tokio::test]
    async fn handshake_incompatible_err() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        const APP_NAME: &str = "oinq";
        const APP_VERSION: &str = "1.0.0";
        const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

        let version_req = semver::VersionReq::parse(&format!(">={PROTOCOL_VERSION}")).unwrap();
        let mut highest_version = semver::Version::parse(PROTOCOL_VERSION).unwrap();
        highest_version.patch += 1;
        let mut protocol_version = highest_version.clone();
        protocol_version.minor += 1;

        let _lock = TOKEN.lock().await;
        let channel = channel().await;
        let (server, client) = (channel.server, channel.client);

        let handle = tokio::spawn(async move {
            super::client::handshake(
                &client.conn,
                APP_NAME,
                APP_VERSION,
                &protocol_version.to_string(),
            )
            .await
        });

        let res = super::server::handshake(
            &server.conn,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            &version_req.to_string(),
            &highest_version.to_string(),
        )
        .await;

        assert!(res.is_err());

        let res = tokio::join!(handle).0.unwrap();
        assert!(res.is_err());
    }
}
