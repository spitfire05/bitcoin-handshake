#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

use bitcoin_handshake::*;
use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use futures::future::join_all;
use std::{
    fmt::Display,
    net::SocketAddr,
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    net::{lookup_host, TcpStream},
    time::timeout,
};
use tracing::instrument;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bitcoin DNS seed to connect to.
    dns_seed: String,

    /// TCP port to connect to.
    #[arg(short, long, default_value_t = PORT_MAINNET)]
    port: u16,

    /// Handshake timeout, in seconds.
    #[arg(short, long, default_value_t = 10)]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let args = Args::parse();

    tracing::info!("Resolving DNS seed `{}`", args.dns_seed);

    let resolved_addrs = lookup_host((args.dns_seed, args.port)).await?;
    let resolved_addrs = resolved_addrs.collect::<Vec<_>>();
    tracing::info!(
        "Resolved {} addreses. Starting handshakes...",
        resolved_addrs.len()
    );

    let results = join_all(resolved_addrs.iter().map(|t| process(*t, args.timeout))).await;

    let fails = results.iter().filter(|x| x.is_err()).count();
    let partial_ok = results
        .iter()
        .filter(|x| matches!(x, Ok(MessageExchangeResult::PartialOk)))
        .count();
    let ok = results
        .iter()
        .filter(|x| matches!(x, Ok(MessageExchangeResult::Ok)))
        .count();

    tracing::info!(
        "Finished! Handshake results: {} OK | {} PARTIALLY OK | {} FAILED",
        ok,
        partial_ok,
        fails
    );

    Ok(())
}

#[instrument]
async fn process(target: SocketAddr, timeout_secs: u64) -> Result<MessageExchangeResult> {
    let result = timeout(Duration::from_secs(timeout_secs), process_inner(target)).await;

    let result = match result {
        Ok(r) => r,
        Err(e) => Err(e.into()),
    };

    match result {
        Ok(MessageExchangeResult::Ok) => tracing::debug!("`{}`: Handshake succeeded", target),
        Ok(MessageExchangeResult::PartialOk) => {
            tracing::debug!("`{}`: Handshake *partially* succeeded", target)
        }
        Err(ref e) => tracing::error!("`{}`: Handshake attempt failed with: {}", target, e),
    };

    result
}

#[instrument]
async fn process_inner(target: SocketAddr) -> Result<MessageExchangeResult> {
    tracing::debug!("`{}`: Starting handshake", target);
    let mut stream = TcpStream::connect(target).await?;

    // send Version
    let version_data = VersionData::new(
        ServiceIdentifier::NODE_NETWORK,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64,
        ServiceIdentifier::NODE_NETWORK,
        stream.local_addr()?,
        ServiceIdentifier::NODE_NETWORK,
        target,
        "".to_string(),
        0,
        false,
    );
    let payload = Payload::Version(version_data);
    let version = Message::new(START_STRING_MAINNET, Command::Version, payload);
    match send_and_expect(target, &mut stream, &version).await {
        Ok(MessageExchangeResult::Ok) => {}
        Ok(MessageExchangeResult::PartialOk) => {
            return Err(eyre!("Partial OK on `version` exchange is an error"))
        }
        Err(e) => return Err(e),
    }

    let verack = Message::new(START_STRING_MAINNET, Command::VerAck, Payload::Empty);

    send_and_expect(target, &mut stream, &verack).await
}

enum MessageExchangeResult {
    Ok,
    PartialOk,
}

async fn send_and_expect(
    target: impl Display,
    stream: &mut (impl AsyncWrite + AsyncRead + Unpin),
    message: &Message,
) -> Result<MessageExchangeResult> {
    // send
    let nonce = match message.payload() {
        Payload::Empty => None,
        Payload::Version(d) => Some(d.nonce()),
    };
    let bytes = message.to_bytes()?;
    tracing::trace!("`{}`: TX {:#?}", target, message);
    stream.write_all(&bytes).await?;
    tracing::debug!("`{}`: Sent {} bytes", target, bytes.len());

    // expect same message type
    let mut br = BufReader::new(stream);
    let mut rx = br.fill_buf().await?;
    let n_recv = rx.len();
    tracing::debug!("`{}`: Received {} bytes", target, n_recv);

    let msg_recv = match Message::from_bytes(&mut rx) {
        Ok(m) => m,
        Err(bitcoin_handshake::errors::BitcoinMessageError::CommandNameUnknown(m)) => {
            tracing::warn!(
                "`{}`: expected message command `{}` but got `{}` instead",
                target,
                message.command(),
                m
            );
            return Ok(MessageExchangeResult::PartialOk);
        }
        Err(e) => return Err(e.into()),
    };
    tracing::trace!("`{}`: RX {:#?}", target, msg_recv);
    if let Some(n) = nonce {
        if let Payload::Version(version_data) = msg_recv.payload() {
            if version_data.nonce() == n {
                return Err(eyre!("nonce conflict"));
            }
        }
    }
    if msg_recv.command() != message.command() {
        tracing::warn!(
            "`{}`: expected message command `{}` but got `{}` instead",
            target,
            message.command(),
            msg_recv.command()
        );
        return Ok(MessageExchangeResult::PartialOk);
    }

    br.consume(n_recv);

    Ok(MessageExchangeResult::Ok)
}
