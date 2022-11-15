use bitcoin_handshake::message::{Message, Payload, VersionData};
use bitcoin_handshake::BitcoinSerialize;
use std::time::SystemTime;
use std::{error::Error, io::Cursor};
use tokio::io::AsyncBufReadExt;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{lookup_host, TcpStream},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let resolved_addrs = lookup_host("seed.bitcoin.sipa.be:8333").await?;
    let target = resolved_addrs.take(1).collect::<Vec<_>>()[0];
    println!("{:?}", target);
    let mut stream = TcpStream::connect(target).await?;
    let mut buf = Cursor::new(Vec::with_capacity(24));
    buf.write_all(&[0xf9, 0xbe, 0xb4, 0xd9]).await?;
    buf.write_all(b"verack").await?;
    buf.set_position(16);
    buf.write_u32(0).await?;
    buf.write_all(&[0x5d, 0xf6, 0xe0, 0xe2]).await?;

    let version_data = VersionData::new(
        0x00,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        0x00,
        stream.local_addr()?,
        0x00,
        target,
        42,
        "".to_string(),
        0,
        false,
    );

    let payload = Payload::Version(version_data);

    let message = Message::new([0xf9, 0xbe, 0xb4, 0xd9], "version", payload)?;

    let bytes = message.to_bytes()?;

    stream.write_all(&bytes).await?;

    println!("TX {} bytes", bytes.len());

    let mut br = BufReader::new(stream);

    let rx = br.fill_buf().await.unwrap();

    println!("RX {} bytes", rx.len());

    Ok(())
}
