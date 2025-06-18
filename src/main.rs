
use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use bytes::{BytesMut, Buf, BufMut};
use prost::Message;
use std::error::Error;
use cache_rs::frame;
use cache_rs::uc::proto;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:50051").await?;
    println!("UltraCache server listening on 127.0.0.1:50051");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New client: {}", addr);
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket).await {
                eprintln!("Error handling {}: {}", addr, e);
            }
        });
    }
}

/// Handles a single client connection by continuously reading and responding to requests.
async fn handle_client( _socket: TcpStream) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn process_request(req: proto::Envelope) -> Result<proto::EnvelopeResponse, Box<dyn Error>> {
    let mut resp = proto::EnvelopeResponse { request_id: req.request_id, body: None };
    use proto::envelope::Body;
    match req.body.unwrap() {
        Body::Get(get) => resp.body = Some(proto::envelope_response::Body::GetResponse(proto::GetResponse { found: false, value: vec![] })),
        Body::Put(put) => {
            println!("PUT {} bytes into {}", put.value.len(), put.cache_name);
            resp.body = Some(proto::envelope_response::Body::PutResponse(proto::PutResponse { success: true }));
        }
        Body::Evict(ev) => {
            println!("EVICT {} from {}", hex::encode(&ev.key), ev.cache_name);
            resp.body = Some(proto::envelope_response::Body::EvictResponse(proto::EvictResponse { success: true }));
        }
        Body::Stats(st) => {
            println!("STATS for {}", st.cache_name);
            resp.body = Some(proto::envelope_response::Body::StatsResponse(proto::StatsResponse { hits: 0, misses: 0, entry_count: 0 }));
        }
        _ => {}
    }
    Ok(resp)
}