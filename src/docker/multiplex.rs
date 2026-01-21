use bytes::{BufMut, BytesMut};

#[allow(dead_code)]
/// Docker stream types for multiplexed streams
#[derive(Debug, Clone, Copy)]
pub enum StreamType {
    Stdout = 1,
    Stderr = 2,
}

/// Encode data with Docker's 8-byte header protocol
/// Format: [stream_type: 1 byte][reserved: 3 bytes][length: 4 bytes BE][data]
pub fn encode_log_frame(stream_type: StreamType, data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + data.len());
    
    // Stream type (1 byte)
    frame.push(stream_type as u8);
    
    // Reserved (3 bytes)
    frame.extend_from_slice(&[0, 0, 0]);
    
    // Length (4 bytes, big endian)
    frame.extend_from_slice(&(data.len() as u32).to_be_bytes());
    
    // Data
    frame.extend_from_slice(data);
    
    frame
}

/// Encode a log line with a newline if not present
pub fn encode_log_line(stream_type: StreamType, line: &str) -> Vec<u8> {
    let mut data = line.as_bytes().to_vec();
    if !data.ends_with(&[b'\n']) {
        data.push(b'\n');
    }
    encode_log_frame(stream_type, &data)
}

#[allow(dead_code)]
/// Create a BytesMut buffer with Docker multiplexed format
pub fn create_multiplexed_frame(stream_type: StreamType, data: &[u8]) -> BytesMut {
    let mut buf = BytesMut::with_capacity(8 + data.len());
    
    buf.put_u8(stream_type as u8);
    buf.put_slice(&[0, 0, 0]);
    buf.put_u32(data.len() as u32);
    buf.put_slice(data);
    
    buf
}

