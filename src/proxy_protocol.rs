use std::net::SocketAddr;

pub fn v1_header(client_addr: SocketAddr, proxy_addr: SocketAddr) -> String {
    let inet = match (client_addr, proxy_addr) {
        (SocketAddr::V4(_), SocketAddr::V4(_)) => "TCP4",
        _ => "TCP6",
    };
    format!(
        "PROXY {inet} {} {} {} {}\r\n",
        client_addr.ip(),
        proxy_addr.ip(),
        client_addr.port(),
        proxy_addr.port(),
    )
}

pub fn v2_header(client_addr: SocketAddr, proxy_addr: SocketAddr) -> Vec<u8> {
    const SIGNATURE: [u8; 12] = *b"\r\n\r\n\x00\r\nQUIT\n";
    const VER_CMD: u8 = 0x21;

    let (fam_proto, addr_bytes): (u8, Vec<u8>) = match (client_addr, proxy_addr) {
        (SocketAddr::V4(client), SocketAddr::V4(proxy)) => {
            let mut bytes = Vec::with_capacity(12);
            bytes.extend_from_slice(&client.ip().octets());
            bytes.extend_from_slice(&proxy.ip().octets());
            bytes.extend_from_slice(&client.port().to_be_bytes());
            bytes.extend_from_slice(&proxy.port().to_be_bytes());
            (0x11, bytes)
        }
        (SocketAddr::V6(client), SocketAddr::V6(proxy)) => {
            let mut bytes = Vec::with_capacity(36);
            bytes.extend_from_slice(&client.ip().octets());
            bytes.extend_from_slice(&proxy.ip().octets());
            bytes.extend_from_slice(&client.port().to_be_bytes());
            bytes.extend_from_slice(&proxy.port().to_be_bytes());
            (0x21, bytes)
        }
        _ => unreachable!("client and proxy addresses must share the same IP version"),
    };

    let mut header = Vec::with_capacity(16 + addr_bytes.len());
    header.extend_from_slice(&SIGNATURE);
    header.push(VER_CMD);
    header.push(fam_proto);
    header.extend_from_slice(&(addr_bytes.len() as u16).to_be_bytes());
    header.extend_from_slice(&addr_bytes);
    header    
}