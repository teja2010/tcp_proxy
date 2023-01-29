use crate::config::Pair;
use log::info;
use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub async fn open_pipe(pair: Pair) -> io::Result<()> {
    let listener = TcpListener::bind(pair.in_sock).await?;
    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(pipe_data(socket, pair.out_sock, addr));
    }
}

async fn pipe_data(
    mut in_stream: TcpStream,
    out_sock: SocketAddr,
    addr: SocketAddr,
) -> io::Result<()> {
    info!(
        "opened connection {} >> {} >> {}",
        addr,
        in_stream.local_addr()?,
        out_sock
    );

    let mut out_stream = TcpStream::connect(out_sock).await?;

    let (into, outto) = tokio::io::copy_bidirectional(&mut in_stream, &mut out_stream).await?;

    info!(
        "connection closed {} >> {} >> {} ({} bytes, {} bytes)",
        addr,
        in_stream.local_addr()?,
        out_sock,
        into,
        outto,
    );
    Ok(())
}

async fn pipe_write(
    in_stream: &TcpStream,
    out_stream: &TcpStream,
    msg: &mut Vec<u8>,
) -> io::Result<bool> {
    match in_stream.try_read(msg) {
        Ok(0) => return Ok(true),
        Ok(n) => msg.truncate(n),
        Err(e) => {
            if e.kind() == io::ErrorKind::WouldBlock {
                return Ok(false);
            } else {
                return Err(e);
            }
        }
    };
    out_stream.writable().await?;
    if let Err(e) = out_stream.try_write(msg) {
        if e.kind() != io::ErrorKind::WouldBlock {
            return Err(e);
        }
    }

    Ok(false)
}
