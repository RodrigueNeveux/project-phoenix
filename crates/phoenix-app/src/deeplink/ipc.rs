use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

use futures::channel::mpsc::UnboundedSender;

use super::Message;

const TIMEOUT: Duration = Duration::from_secs(2);
const MAX_LINE: u64 = 8192;

pub fn endpoint() -> Option<PathBuf> {
    let dir = crate::paths::config_dir()?;
    Some(dir.join(if cfg!(unix) { "instance.sock" } else { "instance.port" }))
}

fn read_message(stream: impl std::io::Read) -> Option<Message> {
    let mut line = String::new();
    BufReader::new(std::io::Read::take(stream, MAX_LINE)).read_line(&mut line).ok()?;
    Message::decode(&line)
}

#[cfg(unix)]
pub fn forward(msg: &Message) -> bool {
    use std::os::unix::net::UnixStream;
    let Some(path) = endpoint() else { return false };
    let Ok(mut s) = UnixStream::connect(&path) else { return false };
    let _ = s.set_write_timeout(Some(TIMEOUT));
    s.write_all(msg.encode().as_bytes()).and_then(|_| s.flush()).is_ok()
}

#[cfg(unix)]
pub fn listen(tx: UnboundedSender<Message>) -> bool {
    use std::os::unix::net::{UnixListener, UnixStream};
    let Some(path) = endpoint() else { return false };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if path.exists() && UnixStream::connect(&path).is_err() {
        let _ = std::fs::remove_file(&path);
    }
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("instance socket unavailable: {e}");
            return false;
        }
    };
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    std::thread::Builder::new()
        .name("phoenix-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(TIMEOUT));
                if let Some(m) = read_message(&stream)
                    && tx.unbounded_send(m).is_err()
                {
                    break;
                }
            }
        })
        .is_ok()
}

#[cfg(not(unix))]
pub fn forward(msg: &Message) -> bool {
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    let Some(path) = endpoint() else { return false };
    let Some(port) = std::fs::read_to_string(&path).ok().and_then(|s| s.trim().parse::<u16>().ok()) else { return false };
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, TIMEOUT) else { return false };
    let _ = s.set_write_timeout(Some(TIMEOUT));
    s.write_all(msg.encode().as_bytes()).and_then(|_| s.flush()).is_ok()
}

#[cfg(not(unix))]
pub fn listen(tx: UnboundedSender<Message>) -> bool {
    use std::net::{Ipv4Addr, TcpListener};
    let Some(path) = endpoint() else { return false };
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else { return false };
    let Ok(port) = listener.local_addr().map(|a| a.port()) else { return false };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::write(&path, port.to_string()).is_err() {
        return false;
    }
    std::thread::Builder::new()
        .name("phoenix-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(TIMEOUT));
                if let Some(m) = read_message(&stream)
                    && tx.unbounded_send(m).is_err()
                {
                    break;
                }
            }
        })
        .is_ok()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_second_launch_reaches_the_listening_instance() {
        let (tx, rx) = super::super::channel();
        assert!(!forward(&Message::default()));
        assert!(listen(tx));
        let msg = Message { token: None, url: Some("discord://-/channels/1/2".into()) };
        assert!(forward(&msg));
        let got = futures::executor::block_on(futures::StreamExt::into_future(rx)).0;
        assert_eq!(got, Some(msg));
    }
}
