// src/mux/frame.rs
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Wire format (big-endian / network byte order):
/// [stream_id: u32][flags: u8][length: u32][payload: length bytes]
///
/// Header = 9 bytes total.
pub const HEADER_LEN: usize = 9;

/// Reasonable default to prevent memory blow-ups. Tune as needed.
pub const MAX_FRAME_LEN: usize = 64 * 1024; // 64 KiB

/// Control and data flags. Combinable where it makes sense.
pub const FLAG_OPEN: u8 = 0x01; // Peer is opening a new stream (SYN)
pub const FLAG_FIN:  u8 = 0x02; // Half-close (EOF) for this direction
pub const FLAG_RST:  u8 = 0x04; // Abort/reset this stream
pub const FLAG_DATA: u8 = 0x08; // Frame carries data

/// A single multiplexing frame attributed to a logical stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub stream_id: u32,
    pub flags: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    #[inline]
    pub fn new(stream_id: u32, flags: u8, payload: Vec<u8>) -> Self {
        Self { stream_id, flags, payload }
    }

    #[inline]
    pub fn new_open(stream_id: u32) -> Self {
        Self { stream_id, flags: FLAG_OPEN, payload: Vec::new() }
    }

    #[inline]
    pub fn new_fin(stream_id: u32) -> Self {
        Self { stream_id, flags: FLAG_FIN, payload: Vec::new() }
    }

    #[inline]
    pub fn new_rst(stream_id: u32) -> Self {
        Self { stream_id, flags: FLAG_RST, payload: Vec::new() }
    }

    #[inline]
    pub fn new_data(stream_id: u32, data: Vec<u8>) -> Self {
        Self { stream_id, flags: FLAG_DATA, payload: data }
    }

    #[inline]
    pub fn payload_len(&self) -> u32 {
        self.payload.len() as u32
    }

    /// Serialize and write the frame to an async writer.
    /// Header fields are encoded in big-endian.
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, w: &mut W) -> io::Result<()> {
        // Sanity check payload length fits u32
        if self.payload.len() > u32::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "payload too large"));
        }

        // Header
        w.write_all(&self.stream_id.to_be_bytes()).await?;
        w.write_all(&[self.flags]).await?;
        w.write_all(&(self.payload.len() as u32).to_be_bytes()).await?;

        // Body
        if !self.payload.is_empty() {
            w.write_all(&self.payload).await?;
        }
        w.flush().await?;
        Ok(())
        }

    /// Read and deserialize a frame from an async reader.
    /// Enforces `MAX_FRAME_LEN` to resist memory DoS.
    pub async fn read_from<R: AsyncRead + Unpin>(r: &mut R) -> io::Result<Self> {
        let mut header = [0u8; HEADER_LEN];
        r.read_exact(&mut header).await?;

        let stream_id = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let flags = header[4];
        let len = u32::from_be_bytes([header[5], header[6], header[7], header[8]]) as usize;

        if len > MAX_FRAME_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("frame length {} exceeds limit {}", len, MAX_FRAME_LEN),
            ));
        }

        let mut payload = vec![0u8; len];
        if len > 0 {
            r.read_exact(&mut payload).await?;
        }

        Ok(Self { stream_id, flags, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn frame_roundtrip_data() {
        let (mut a, mut b) = duplex(8 * 1024);

        let f_out = Frame::new_data(42, b"hello world".to_vec());

        // writer
        let w = async {
            f_out.write_to(&mut a).await.unwrap();
        };

        // reader
        let r = async {
            let f_in = Frame::read_from(&mut b).await.unwrap();
            assert_eq!(f_in.stream_id, 42);
            assert_eq!(f_in.flags, FLAG_DATA);
            assert_eq!(f_in.payload, b"hello world");
        };

        tokio::join!(w, r);
    }

    #[tokio::test]
    async fn frame_roundtrip_control() {
        let (mut a, mut b) = duplex(1024);

        let open = Frame::new_open(1);
        let fin  = Frame::new_fin(1);
        let rst  = Frame::new_rst(2);

        let writer = async {
            open.write_to(&mut a).await.unwrap();
            fin.write_to(&mut a).await.unwrap();
            rst.write_to(&mut a).await.unwrap();
        };

        let reader = async {
            let f1 = Frame::read_from(&mut b).await.unwrap();
            assert_eq!((f1.stream_id, f1.flags, f1.payload.len()), (1, FLAG_OPEN, 0));

            let f2 = Frame::read_from(&mut b).await.unwrap();
            assert_eq!((f2.stream_id, f2.flags, f2.payload.len()), (1, FLAG_FIN, 0));

            let f3 = Frame::read_from(&mut b).await.unwrap();
            assert_eq!((f3.stream_id, f3.flags, f3.payload.len()), (2, FLAG_RST, 0));
        };

        tokio::join!(writer, reader);
    }

    #[tokio::test]
    async fn frame_enforces_max_length() {
        let (mut a, mut b) = duplex(1024);

        // Manually craft a header advertising an absurd length.
        let stream_id = 7u32.to_be_bytes();
        let flags = [FLAG_DATA];
        let len = (MAX_FRAME_LEN as u32 + 1).to_be_bytes();

        let writer = async {
            a.write_all(&stream_id).await.unwrap();
            a.write_all(&flags).await.unwrap();
            a.write_all(&len).await.unwrap();
            // no payload written
            a.flush().await.unwrap();
        };

        let reader = async {
            let err = Frame::read_from(&mut b).await.unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        };

        tokio::join!(writer, reader);
    }
}
