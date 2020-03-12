use async_std::net::{ TcpListener, TcpStream, ToSocketAddrs, SocketAddr, IpAddr, Ipv4Addr };
use async_std::prelude::*;
use async_std::task;
use std::str;
use async_std::io::copy;

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS5_AUTH_METHOD_NONE: u8 = 0x00;
const SOCKS5_CMD_TCP_CONNECT: u8 = 0x01;
const _SOCKS5_RSV: u8 = 0x00;
const SOCKS5_ADDR_TYPE_IPV4: u8 = 0x01;
const SOCKS5_ADDR_TYPE_DOMAIN: u8 = 0x03;
const SOCKS5_ADDR_TYPE_IPV6: u8 = 0x04;
const SOCKS5_REPLY_SUCCEEDED: u8 = 0x00;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  run()
}

fn run() -> Result<()> {
  let fut = accept_loop("127.0.0.1:8080");
  task::block_on(fut)
}

async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
  let listener = TcpListener::bind(addr).await?;
  let mut incoming = listener.incoming();
  while let Some(stream) = incoming.next().await {
    let stream = stream?;
    println!("Accept from {}", stream.peer_addr()?);
    let _handle = task::spawn(connection_loop(stream));
  }
  unimplemented!()
}

async fn connection_loop(mut stream: TcpStream) -> Result<()> {
  // +----+----------+----------+
  // |VER | NMETHODS | METHODS  |
  // +----+----------+----------+
  // | 1  |    1     | 1 to 255 |
  // +----+----------+----------+
  let mut version = [0u8; 1];
  stream.read_exact(&mut version).await?;
  if version[0] != SOCKS5_VERSION {
    Err("Only support socks 5 version")?
  }
  let mut _methods_count = [0u8; 1];
  stream.read_exact(&mut _methods_count).await?;
  let mut methods = vec![0u8; 255];
  let size = stream.read(&mut methods).await?;
  methods.truncate(size);
  let mut has_methods = false;
  for method in methods {
    if method == SOCKS5_AUTH_METHOD_NONE {
      has_methods = true;
      break;
    }
  }
  if !has_methods {
    Err("Haven't connect method")?
  }

  // +----+--------+
  // |VER | METHOD |
  // +----+--------+
  // | 1  |   1    |
  // +----+--------+
  let response = [SOCKS5_VERSION, SOCKS5_AUTH_METHOD_NONE];
  stream.write(&response).await?;

  // +----+-----+-------+------+----------+----------+
  // |VER | CMD |  RSV  | ATYP | DST.ADDR | DST.PORT |
  // +----+-----+-------+------+----------+----------+
  // | 1  |  1  | X'00' |  1   | Variable |    2     |
  // +----+-----+-------+------+----------+----------+
  let mut version = [0u8; 1];
  stream.read_exact(&mut version).await?;
  if version[0] != SOCKS5_VERSION {
    Err("Only support socks 5 version")?
  }
  let mut cmd = [0u8; 2];
  stream.read_exact(&mut cmd).await?;
  if cmd[0] != SOCKS5_CMD_TCP_CONNECT {
    Err("Only support TCP connection")?
  }
  let mut addr_type = [0u8; 1];
  stream.read_exact(&mut addr_type).await?;
  enum DestAddrType {
    Ipv4([u8; 4]),
    Domain(Vec<u8>),
    Unknown,
  }
  let dest_addr_type = match addr_type[0] {
    SOCKS5_ADDR_TYPE_IPV4 => {
      let mut ipv4_addr = [0u8; 4];
      stream.read_exact(&mut ipv4_addr).await?;
      DestAddrType::Ipv4(ipv4_addr)
    }
    SOCKS5_ADDR_TYPE_DOMAIN => {
      let mut domain_size = [0u8; 1];
      stream.read_exact(&mut domain_size).await?;
      let domain_size = domain_size[0] as usize;
      let mut domain_addr = vec![0u8; domain_size];
      stream.read_exact(&mut domain_addr).await?;
      DestAddrType::Domain(domain_addr)
    }
    SOCKS5_ADDR_TYPE_IPV6 => DestAddrType::Unknown,
    _ => DestAddrType::Unknown,
  };
  let mut dest_addr_port = [0u8; 2];
  stream.read_exact(&mut dest_addr_port).await?;
  let dest_addr_port = dest_addr_port[0] as u16 * 256 + dest_addr_port[1] as u16;
  //
  // @todo reply client
  //
  enum DestStream {
    DestStream(TcpStream),
    Unknown(String)
  }
  let stream = match dest_addr_type {
    DestAddrType::Ipv4(ipv4_addr) => {
      let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(ipv4_addr[0], ipv4_addr[1], ipv4_addr[2], ipv4_addr[3])), dest_addr_port);
      let dest_stream = TcpStream::connect(socket_addr).await?;
      DestStream::DestStream(dest_stream)
    },
    DestAddrType::Domain(domain_addr) => {
      let domain = str::from_utf8(&domain_addr)?;
      let dest_stream = TcpStream::connect(format!("{}{}", domain, dest_addr_port)).await?;
      DestStream::DestStream(dest_stream)
    },
    _ => DestStream::Unknown("Unknown destination addr type".to_string())
  };
  match stream {
    DestStream::Unknown(s) => {
      Err(s)?
    },
    DestStream::DestStream(dest_stream) => {
      //
      // @todo copy stream
      //
      unimplemented!()
    } 
  }

  unimplemented!()
}
