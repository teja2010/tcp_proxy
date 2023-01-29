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

async fn pipe_data(in_stream: TcpStream, out_sock: SocketAddr, addr: SocketAddr) -> io::Result<()> {
    info!(
        "opened connection {} >> {} >> {}",
        addr,
        in_stream.local_addr()?,
        out_sock
    );

    let out_stream = TcpStream::connect(out_sock).await?;

    let mut msg = vec![0; 65536];
    loop {
        tokio::select! {
            _ = in_stream.readable() => {
                if pipe_write(&in_stream, &out_stream, &mut msg).await? {
                    break;
                }
            }
            _ = out_stream.readable() => {
                if pipe_write(&out_stream, &in_stream, &mut msg).await? {
                    break;
                }
            }
        }
    }

    info!(
        "connection closed {} >> {} >> {}",
        addr,
        in_stream.local_addr()?,
        out_sock,
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
