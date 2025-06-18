use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{ BytesMut, Buf, BufMut};

pub const PROTOCOL_VERSION: u8 = 1;

pub type MessageType = u8;

pub const MESSAGE_TYPE_UNSPECIFIED:        MessageType = 0;
pub const MESSAGE_TYPE_ENVELOPE:           MessageType = 1;
pub const MESSAGE_TYPE_ENVELOPE_RESPONSE:  MessageType = 2;
pub const MESSAGE_TYPE_DISCONNECT:         MessageType = 3;

/// Read and parse the 8-byte frame header from any AsyncRead source.
#[allow(dead_code)]
pub async fn read_frame_header<R>(reader: &mut R) -> Result<(u32, u8, u8, u16), Box<dyn Error>>
where
    R: AsyncReadExt + Unpin,
{
    let mut header = [0u8; 8];
    reader.read_exact(&mut header).await?;
    let mut buf = BytesMut::from(&header[..]);
    let frame_len = buf.get_u32();
    let version = buf.get_u8();
    let msg_type = buf.get_u8();
    let flags = buf.get_u16();
    Ok((frame_len, version, msg_type, flags))
}

/// Write a framed message: header + payload.
pub async fn write_frame<W>(writer: &mut W, msg_type: u8, payload: &[u8]) -> Result<(), Box<dyn Error>>
where
    W: AsyncWriteExt + Unpin,
{
    // 1 byte version + 1 byte type + 2 bytes flags + payload
    let frame_len = (1 + 1 + 2 + payload.len()) as u32;
    let mut header = BytesMut::with_capacity(8);
    header.put_u32(frame_len);
    header.put_u8(PROTOCOL_VERSION);
    header.put_u8(msg_type);
    header.put_u16(0); // flags

    writer.write_all(&header).await?;
    writer.write_all(payload).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use bytes::BufMut;

    #[tokio::test]
    async fn test_read_frame_header() {
        // Construct a header: frame_len=10, version=1, msg_type=2, flags=0xAABB
        let mut header_buf = Vec::with_capacity(8);
        header_buf.put_u32(10);
        header_buf.put_u8(1);
        header_buf.put_u8(2);
        header_buf.put_u16(0xAABB);

        let mut cursor = Cursor::new(header_buf.to_vec());
        let (frame_len, version, msg_type, flags) = read_frame_header(&mut cursor).await.unwrap();

        assert_eq!(frame_len, 10);
        assert_eq!(version, 1);
        assert_eq!(msg_type, 2);
        assert_eq!(flags, 0xAABB);
    }

     #[tokio::test]
    async fn test_write_frame() {
        let payload = b"hello";
        let mut buf = Vec::new();
        write_frame(&mut buf, 0x42, payload).await.unwrap();
        // Header: len=1+1+2+5=9 -> [0,0,0,9], version=1, type=0x42, flags=0, payload="hello"
        let expected = [&9u32.to_be_bytes()[..], &[1], &[0x42], &0u16.to_be_bytes()[..], b"hello"].concat();
        assert_eq!(buf, expected);
    }

}
