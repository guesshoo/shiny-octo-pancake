#![allow(dead_code)]        // unreferenced functions, structs, enums, etc.
#![allow(unused_imports)]   // imports that never get used
#![allow(unused_variables)] // local variables that never get read

use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use std::error::Error;
use prost::Message;
use cache_rs::{
    frame,
    uc::proto,
};


use bytes::{BytesMut, Buf, BufMut};
use hex; // for encoding byte slices as hex strings


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
async fn handle_client(mut socket: TcpStream) -> Result<(), Box<dyn Error>> {
    loop {
        // Read and parse header
        let (frame_len, version, msg_type, _) = frame::read_frame_header(&mut socket).await?;

        if version != frame::PROTOCOL_VERSION {
            return Err("Unsupported protocol version".into());
        }

        // Read payload
        let payload_len = frame_len - 4;
        let mut buf = vec![0u8; payload_len as usize];
        socket.read_exact(&mut buf).await?;

        // Decode request
        let req = proto::Envelope::decode(&buf[..])?;

        // // If Disconnect, send ack and break
        // if let Some(proto::envelope::Body::Disconnect(_)) = req.body {
        //     let ack = proto::DisconnectResponse { message: "Goodbye".into() };
        //     let mut resp = proto::EnvelopeResponse { request_id: req.request_id, body: Some(proto::envelope_response::Body::DisconnectResponse(ack)) };
        //     let mut resp_buf = Vec::new();
        //     resp.encode(&mut resp_buf)?;
        //     write_frame(&mut socket, proto::MessageType::EnvelopeResponse as u8, &resp_buf).await?;
        //     println!("Client requested disconnect");
        //     return Ok(());
        // }

        // Process and respond
        let resp = process_request(req)?;
        let mut resp_buf = Vec::new();
        resp.encode(&mut resp_buf)?;
        frame::write_frame(&mut socket, frame::MESSAGE_TYPE_ENVELOPE_RESPONSE, &resp_buf).await?;
    }
}


fn process_request(req: proto::Envelope) -> Result<proto::EnvelopeResponse, Box<dyn Error>> {
    let mut resp = proto::EnvelopeResponse { request_id: req.request_id, body: None };
    use proto::envelope::Body;
    match req.body.unwrap() {
        Body::Put(put) => {
            println!("PUT {} bytes into {}", put.value.len(), put.cache_name);
            resp.body = Some(proto::envelope_response::Body::PutResponse(proto::PutResponse { success: true }));
        }

        Body::Get(get) => {
            println!("GET {:?}",  get);
            resp.body = Some(proto::envelope_response::Body::GetResponse(proto::GetResponse { found: false, value: vec![] }));
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