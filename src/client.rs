#![allow(dead_code)]        // unreferenced functions, structs, enums, etc.
#![allow(unused_imports)]   // imports that never get used
#![allow(unused_variables)] // local variables that never get read

use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};
use bytes::{BytesMut, Buf, BufMut};
use prost::Message;
use std::error::Error;


use cache_rs::{
    frame,
    uc::proto,
};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut socket = TcpStream::connect("127.0.0.1:50051").await?;

    // Prepare GET request
    let get_req = proto::GetRequest { cache_name: "default".into(), key: b"mykey".to_vec() };
    let envelope = proto::Envelope { request_id: 1, body: Some(proto::envelope::Body::Get(get_req)) };
    let mut payload = Vec::new(); envelope.encode(&mut payload)?;

    frame::write_frame(&mut socket, frame::MESSAGE_TYPE_ENVELOPE, &payload).await?;

    // Read and decode response
    let (resp_len, _, _, _) = frame::read_frame_header(&mut socket).await?;
    let mut resp_payload = vec![0u8; (resp_len - 4) as usize]; socket.read_exact(&mut resp_payload).await?;
    let resp_env = proto::EnvelopeResponse::decode(&resp_payload[..])?;
    if let Some(proto::envelope_response::Body::GetResponse(gr)) = resp_env.body {
        println!("Found: {}", gr.found);
    }
    Ok(())
}