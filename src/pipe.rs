use crate::config::{Pair, Protocol};
use log::{debug, error, info};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream, UdpSocket};

pub async fn open_pipe(pair: Pair) -> io::Result<()> {
    match pair.protocol {
        Protocol::TCP => {
            let listener = TcpListener::bind(pair.in_sock).await?;
            loop {
                let (socket, addr) = listener.accept().await?;
                tokio::spawn(pipe_tcp_data(socket, pair.out_sock, addr));
            }
        }
        Protocol::DNS => loop {
            let in_sock = match UdpSocket::bind(pair.in_sock).await {
                Ok(s) => s,
                Err(e) => {
                    error!("error binding {}", e);
                    return Ok(());
                },
            };
            let _ = pipe_dns_data(in_sock, pair.out_sock).await;
        },
    }
}

async fn pipe_tcp_data(
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

async fn pipe_dns_data(sock: UdpSocket, upstream_addr: SocketAddr) -> io::Result<()> {
    info!("UDP sock created {}", sock.local_addr()?,);

    let mut queries_map: std::collections::HashMap<u16, SocketAddr> = HashMap::new();

    let mut buf = [0; 2000];
    loop {
        match sock.recv_from(&mut buf).await {
            Ok((num_bytes, from_addr)) => {
                debug!(
                    "UDP msg from {} >> {} >> {}",
                    from_addr,
                    sock.local_addr()?,
                    upstream_addr,
                );

                let msg = &buf[..num_bytes];
                if msg.len() < 2 {
                    error!("msg too short");
                    continue;
                }
                let query_id: u16 = (u16::from(msg[0]) << 8) + u16::from(msg[1]);

                if from_addr == upstream_addr {
                    // this is a response from upstream server, return it to the right requester
                    if let Some(requester_addr) = get_addr_count(&mut queries_map, query_id) {
                        sock.send_to(msg, requester_addr).await?;
                    } else {
                        continue;
                    }
                } else {
                    // this is a new request, send it upstream
                    add_addr_count(&mut queries_map, query_id, from_addr);
                    sock.send_to(msg, upstream_addr).await?;
                }
            }
            Err(e) => {
                error!("Error reading udp msg {}", e);
            }
        }
    }
}

fn get_addr_count(queries_map: &mut HashMap<u16, SocketAddr>, query_id: u16) -> Option<SocketAddr> {
    match queries_map.remove(&query_id) {
        Some(a) => Some(a),
        None => {
            error!("did not find addr for query {}", query_id);
            None
        }
    }
}

fn add_addr_count(
    queries_map: &mut HashMap<u16, SocketAddr>,
    query_id: u16,
    requester_addr: SocketAddr,
) {
    if queries_map.contains_key(&query_id) {
        error!(
            "duplicate query_id {} . did not insert {}",
            query_id, requester_addr
        );
        return;
    }
    debug!("queries_map added {:#?}", requester_addr);
    queries_map.insert(query_id, requester_addr);
}
