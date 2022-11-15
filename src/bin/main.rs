use bitcoin_handshake::enums::ServiceIdentifier;
use bitcoin_handshake::message::{
    BitcoinDeserialize, BitcoinSerialize, Message, Payload, VersionData,
};
use bitcoin_handshake::PORT_MAINNET;
use color_eyre::eyre::{Context, Result};
use env_logger::Env;
use futures::future::join_all;
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{lookup_host, TcpStream},
};

/// Configuration. In real app this should be sourced from config file or cmd line args.
struct Config {
    pub dns_seed: String,
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    color_eyre::install()?;

    let conf = Config {
        dns_seed: "seed.bitcoin.sipa.be".to_string(),
        port: PORT_MAINNET,
    };

    log::info!("Resolving DNS seed `{}`", conf.dns_seed);

    let resolved_addrs = lookup_host((conf.dns_seed, conf.port)).await?;
    let resolved_addrs = resolved_addrs.collect::<Vec<_>>();
    log::info!(
        "Resolved {} addreses. Starting handshakes...",
        resolved_addrs.len()
    );

    join_all(resolved_addrs.iter().map(|t| process(*t))).await;

    Ok(())
}

async fn process(target: SocketAddr) {
    match process_inner(target).await {
        Ok(_) => log::debug!("`{}`: Handshake succeded", target),
        Err(e) => log::error!("`{}`: Failed with: {}", target, e),
    }
}

async fn process_inner(target: SocketAddr) -> Result<()> {
    log::debug!("`{}`: Starting handshake", target);
    let mut stream = TcpStream::connect(target).await?;

    // send Version
    let version_data = VersionData::new(
        ServiceIdentifier::NODE_NETWORK,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        ServiceIdentifier::NODE_NETWORK,
        stream.local_addr()?,
        ServiceIdentifier::NODE_NETWORK,
        target,
        42,
        "".to_string(),
        0,
        false,
    );
    let payload = Payload::Version(version_data);
    let message = Message::new([0xf9, 0xbe, 0xb4, 0xd9], "version", payload)
        .wrap_err_with(|| "Can not construct message")?;
    let bytes = message.to_bytes()?;
    log::trace!("`{}`: TX {:#?}", target, message);
    stream.write_all(&bytes).await?;
    log::debug!("`{}`: Sent {} bytes", target, bytes.len());

    // receive Version
    let mut br = BufReader::new(stream);
    let mut rx = br.fill_buf().await?;
    let n_recv = rx.len();
    log::debug!("`{}`: Received {} bytes", target, n_recv);

    let msg_recv = Message::from_bytes(&mut rx)?;
    log::trace!("`{}`: RX {:#?}", target, msg_recv);

    br.consume(n_recv);

    Ok(())
}
