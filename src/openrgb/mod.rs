use std::net::SocketAddr;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::info;

#[allow(dead_code)]
const OPENRGB_PROTOCOL_VERSION: u32 = 4;

#[derive(Debug, Clone)]
pub struct OpenRgbClient {
    address: SocketAddr,
    dry_run: bool,
}

#[derive(Debug, Error)]
pub enum OpenRgbError {
    #[error("network error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("dry run mode active")]
    DryRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u32)]
enum Command {
    RequestProtocolVersion = 0,
    SetClientName = 50,
    UpdateLeds = 1050,
    UpdateZoneLeds = 1051,
    UpdateSingleLed = 1052,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u32)]
enum Mode {
    Begin = 0,
    Set = 1,
    End = 2,
}

impl OpenRgbClient {
    pub fn new(host: &str, port: u16, dry_run: bool) -> Self {
        let address = format!("{}:{}", host, port)
            .parse()
            .expect("invalid OpenRGB address");
        Self { address, dry_run }
    }

    pub async fn connect(&self) -> Result<Connection, OpenRgbError> {
        if self.dry_run {
            info!("dry-run: skipping OpenRGB connection to {}", self.address);
            return Ok(Connection::DryRun);
        }

        let stream = TcpStream::connect(self.address).await?;
        info!("connected to OpenRGB server at {}", self.address);
        let mut conn = Connection::Real(stream);
        conn.send_command(Command::SetClientName, Mode::Begin, 0, b"lighthouse\0")
            .await?;
        Ok(conn)
    }
}

pub enum Connection {
    Real(TcpStream),
    DryRun,
}

impl Connection {
    async fn send_command(
        &mut self,
        command: Command,
        mode: Mode,
        device_id: u32,
        data: &[u8],
    ) -> Result<(), OpenRgbError> {
        match self {
            Connection::DryRun => {
                info!(
                    "dry-run: command={:?} mode={:?} device={} len={}",
                    command,
                    mode,
                    device_id,
                    data.len()
                );
                Ok(())
            }
            Connection::Real(stream) => {
                let header =
                    build_header(command as u32, mode as u32, device_id, data.len() as u32);
                stream.write_all(&header).await?;
                stream.write_all(data).await?;
                stream.flush().await?;
                Ok(())
            }
        }
    }

    pub async fn set_zone_color(
        &mut self,
        device_id: u32,
        zone_id: u32,
        led_count: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let mut data = Vec::with_capacity((4 + led_count * 3) as usize);
        data.extend_from_slice(&zone_id.to_le_bytes());
        for _ in 0..led_count {
            data.extend_from_slice(&color);
        }
        self.send_command(Command::UpdateZoneLeds, Mode::Set, device_id, &data)
            .await
    }

    pub async fn set_all_color(
        &mut self,
        device_id: u32,
        led_count: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let mut data = Vec::with_capacity((led_count * 4) as usize);
        for i in 0..led_count {
            data.extend_from_slice(&i.to_le_bytes());
            data.extend_from_slice(&color);
        }
        self.send_command(Command::UpdateLeds, Mode::Set, device_id, &data)
            .await
    }

    pub async fn set_single_color(
        &mut self,
        device_id: u32,
        led_index: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let mut data = Vec::with_capacity(7);
        data.extend_from_slice(&led_index.to_le_bytes());
        data.extend_from_slice(&color);
        self.send_command(Command::UpdateSingleLed, Mode::Set, device_id, &data)
            .await
    }
}

fn build_header(command: u32, mode: u32, device_id: u32, data_size: u32) -> Vec<u8> {
    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(&command.to_le_bytes());
    header.extend_from_slice(&data_size.to_le_bytes());
    header.extend_from_slice(&device_id.to_le_bytes());
    header.extend_from_slice(&mode.to_le_bytes());
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_format() {
        let h = build_header(1050, 1, 0, 3);
        assert_eq!(h.len(), 16);
        assert_eq!(&h[0..4], &[0x1a, 0x04, 0x00, 0x00]);
    }

    #[test]
    fn set_zone_color_payload() {
        let mut conn = Connection::DryRun;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            conn.set_zone_color(0, 1, 4, [255, 0, 0]).await.unwrap();
        });
    }

    #[test]
    fn set_all_color_payload() {
        let mut conn = Connection::DryRun;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            conn.set_all_color(0, 2, [255, 0, 0]).await.unwrap();
        });
    }
}
