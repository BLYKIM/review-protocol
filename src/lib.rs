#[cfg(feature = "client")]
pub mod client;
pub mod protocol_types;
#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub use oinq::request;

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
