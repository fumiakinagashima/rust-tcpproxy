use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;

const PEEK_BUF_SIZE: usize = 4096;
const PEEK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

pub fn build_sni_routes() -> HashMap<String, SocketAddr> {
    let mut routes = HashMap::new();
    routes.insert("a.example.com".to_string(), "127.0.0.1:9001".parse().unwrap());
    routes.insert("b.example.com".to_string(), "127.0.0.1:9002".parse().unwrap());
    routes
}

pub async fn peek_sni(inbound: &TcpStream, timeout: Duration) -> Option<String> {
    let mut buf = vec![0u8; PEEK_BUF_SIZE];
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let n = inbound.peek(&mut buf).await.ok()?;
        if let Some(name) = parse_client_hello_sni(&buf[..n]) {
            return Some(name);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(PEEK_RETRY_INTERVAL).await;
    }
}

fn parse_client_hello_sni(buf: &[u8]) -> Option<String> {
    if buf.len() < 5 || buf[0] != 0x16 {
        return None;
    }
    let record_len = u16::from_be_bytes([buf[3], buf[4]]) as usize;
    let record_end = 5 + record_len;
    if buf.len() < record_end {
        return None;
    }

    let handshake = &buf[5..record_end];
    if handshake.len() < 4 || handshake[0] != 0x01 {
        return None;
    }

    let mut pos = 4;
    pos += 2;
    pos += 32;
    if pos >= handshake.len() {
        return None;
    }

    let session_id_len = handshake[pos] as usize;
    pos += 1 + session_id_len;
    if pos + 2 > handshake.len() {
        return None;
    }

    let cipher_suites_len = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]) as usize;
    pos += 2 + cipher_suites_len;
    if pos + 1 > handshake.len() {
        return None;
    }

    let compression_methods_len = handshake[pos] as usize;
    pos += 1 + compression_methods_len;
    if pos + 2 > handshake.len() {
        return None;
    }

    let extensions_len = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]) as usize;
    pos += 2;
    let extensions_end = (pos + extensions_len).min(handshake.len());

    while pos + 4 <= extensions_end {
        let ext_type = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]);
        let ext_len = u16::from_be_bytes([handshake[pos + 2], handshake[pos + 3]]) as usize;
        let ext_start = pos + 4;
        let ext_end = ext_start + ext_len;
        if ext_end > extensions_end {
            return None;
        }
        if ext_type == 0x0000 {
            return parse_server_name(&handshake[ext_start..ext_end]);
        }
        pos = ext_end;
    }
    None
}

fn parse_server_name(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
    let list_end = (2 + list_len).min(data.len());
    let mut pos = 2;
    
    while pos + 3 <= list_end {
        let name_type = data[pos];
        let name_len = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
        let name_start = pos + 3;
        let name_end = name_start + name_len;
        if name_end > list_end {
            return None;
        }
        if name_type == 0 {
            return String::from_utf8(data[name_start..name_end].to_vec()).ok();
        }
        pos = name_end;
    }
    None
}
