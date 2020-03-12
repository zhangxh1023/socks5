use async_std::net::{ TcpListener, ToSocketAddrs, TcpStream };
use async_std::prelude::*;
use async_std::task;

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
  let response = [SOCKS5_VERSION,  SOCKS5_AUTH_METHOD_NONE];
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
  match addr_type[0] {
    SOCKS5_ADDR_TYPE_IPV4 => {
      unimplemented!()
    },
    SOCKS5_ADDR_TYPE_DOMAIN => {
      unimplemented!()
    },
    SOCKS5_ADDR_TYPE_IPV6 => {
      unimplemented!()
    },
    _ => Err("Unknown destination addr type")?
  };
  let mut port = [0u8; 2];
  stream.read_exact(&mut port).await?;

  unimplemented!()
}
