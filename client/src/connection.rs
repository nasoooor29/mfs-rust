use std::{
    io::{self, Write},
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use maze_runner::protocol::{decode, encode, ClientMessage, ServerMessage, PROTOCOL_VERSION};

pub fn prompt(label: &str, default: &str) -> io::Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim();
    Ok(if value.is_empty() {
        default.into()
    } else {
        value.into()
    })
}

pub fn connect(
    server: SocketAddr,
    username: String,
) -> Result<(UdpSocket, ServerMessage), Box<dyn std::error::Error>> {
    let bind = if server.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = UdpSocket::bind(bind)?;
    socket.connect(server)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let request = encode(&ClientMessage::Connect {
        version: PROTOCOL_VERSION,
        username,
    })?;
    let mut buffer = [0u8; 65_507];
    for _ in 0..5 {
        socket.send(&request)?;
        if let Ok(length) = socket.recv(&mut buffer) {
            let message: ServerMessage = decode(&buffer[..length])?;
            match message {
                ServerMessage::Welcome { .. } => {
                    socket.set_read_timeout(None)?;
                    socket.set_nonblocking(true)?;
                    return Ok((socket, message));
                }
                ServerMessage::Error(reason) => return Err(reason.into()),
                _ => {}
            }
        }
    }
    Err("server did not respond after 5 attempts".into())
}

pub fn spawn_receiver(socket: &UdpSocket) -> io::Result<Receiver<ServerMessage>> {
    let (sender, receiver) = mpsc::channel();
    let receive_socket = socket.try_clone()?;
    std::thread::spawn(move || {
        let mut buffer = [0u8; 65_507];
        loop {
            match receive_socket.recv(&mut buffer) {
                Ok(length) => {
                    if let Ok(message) = decode(&buffer[..length]) {
                        if sender.send(message).is_err() {
                            break;
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(_) => break,
            }
        }
    });
    Ok(receiver)
}
