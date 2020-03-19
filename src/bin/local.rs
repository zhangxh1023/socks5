use async_std::net::{ TcpListener, ToSocketAddrs, TcpStream, Shutdown };
use async_std::task;
use async_std::prelude::*;
use async_std::io;

const SOCKS5_VERSION: u8 = 0x05;
const SOCKS5_AUTH_METHOD_NONE: u8 = 0x00;
const SOCKS5_CMD_TCP_CONNECT: u8 = 0x01;
const SOCKS5_RSV: u8 = 0x00;
const SOCKS5_ADDR_TYPE_IPV4: u8 = 0x01;
const SOCKS5_ADDR_TYPE_DOMAIN: u8 = 0x03;
const SOCKS5_ADDR_TYPE_IPV6: u8 = 0x04;
const SOCKS5_REPLY_SUCCEEDED: u8 = 0x00;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
  run()
}

fn run() -> Result<()> {
  let socks5_fut = socks5_accept_loop("127.0.0.1:1186");
  println!("listening 127.0.0.1:1186");
  let http_fut = http_accept_loop("127.0.0.1:1187");
  println!("listening 127.0.0.1:1187");
  task::spawn(async {
    task::block_on(http_fut)
  });
  task::block_on(socks5_fut)
}

async fn socks5_accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
  let listener = TcpListener::bind(addr).await?;
  let mut incoming = listener.incoming();
  while let Some(stream) = incoming.next().await {
    let stream = stream?;
    println!("Accept from {}", stream.peer_addr()?);
    spawn_and_log_error(socks5_handle(stream));
  }
  Ok(())
}

async fn http_accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
  let listener = TcpListener::bind(addr).await?;
  let mut incoming = listener.incoming();
  while let Some(stream) = incoming.next().await {
    let stream = stream?;
    println!("Accept from {}", stream.peer_addr()?);
    spawn_and_log_error(http_handle(stream));
  }
  Ok(())
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
  F: Future<Output = Result<()>> + Send + 'static,
{
  task::spawn(async move {
    if let Err(e) = fut.await {
      eprintln!("{}", e)
    }
  })
}

async fn socks5_handle(mut stream: TcpStream) -> Result<()> {
  let mut dest_stream = TcpStream::connect("127.0.0.1:8080").await?;

  let mut stream_reader = stream.clone();
  let mut dest_stream_writer = dest_stream.clone();
  task::spawn(async move {
    if let Err(e) = io::copy(&mut stream_reader, &mut dest_stream_writer).await {
      println!("{}", e);
    };
    if let Err(e) = stream_reader.shutdown(Shutdown::Read) {
      println!("{}", e);
    };
    if let Err(e) = dest_stream_writer.shutdown(Shutdown::Write) {
      println!("{}", e);
    };
  });

  io::copy(&mut dest_stream, &mut stream).await?;
  if let Err(e) = stream.shutdown(Shutdown::Write) {
    println!("{}", e);
  };
  if let Err(e) = dest_stream.shutdown(Shutdown::Read) {
    println!("{}", e);
  };

  Ok(())
}

async fn http_handle(mut stream: TcpStream) -> Result<()> {



  unimplemented!()
}
