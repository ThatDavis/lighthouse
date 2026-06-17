use std::net::SocketAddr;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
    RequestControllerCount = 1,
    RequestControllerData = 2,
    SetClientName = 50,
    UpdateLeds = 1050,
    UpdateZoneLeds = 1051,
    UpdateSingleLed = 1052,
    UpdateMode = 1100,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u32)]
enum Mode {
    Begin = 0,
    Set = 1,
    End = 2,
}

#[derive(Debug, Clone)]
pub struct ZoneData {
    pub name: String,
    pub led_count: u32,
}

#[derive(Debug, Clone)]
pub struct ControllerData {
    pub zones: Vec<ZoneData>,
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

#[derive(Debug)]
#[allow(dead_code)]
struct Packet {
    command: u32,
    device_id: u32,
    mode: u32,
    data: Vec<u8>,
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

    async fn read_packet(&mut self) -> Result<Packet, OpenRgbError> {
        match self {
            Connection::DryRun => Err(OpenRgbError::DryRun),
            Connection::Real(stream) => {
                let mut header = [0u8; 16];
                stream.read_exact(&mut header).await?;
                let command = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
                let data_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
                let device_id = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
                let mode = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
                let mut data = vec![0u8; data_size as usize];
                if data_size > 0 {
                    stream.read_exact(&mut data).await?;
                }
                Ok(Packet {
                    command,
                    device_id,
                    mode,
                    data,
                })
            }
        }
    }

    pub async fn query_controller_data(
        &mut self,
        device_id: u32,
    ) -> Result<ControllerData, OpenRgbError> {
        self.send_command(
            Command::RequestControllerData,
            Mode::Begin,
            device_id,
            &OPENRGB_PROTOCOL_VERSION.to_le_bytes(),
        )
        .await?;
        let packet = self.read_packet().await?;
        if packet.command != Command::RequestControllerData as u32 {
            return Err(OpenRgbError::Protocol(format!(
                "expected controller data, got command {}",
                packet.command
            )));
        }
        parse_controller_data(&packet.data)
    }

    pub async fn set_device_mode(
        &mut self,
        device_id: u32,
        mode_index: u32,
    ) -> Result<(), OpenRgbError> {
        self.send_command(
            Command::UpdateMode,
            Mode::Set,
            device_id,
            &mode_index.to_le_bytes(),
        )
        .await
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

fn parse_controller_data(data: &[u8]) -> Result<ControllerData, OpenRgbError> {
    let mut offset = 0usize;

    // device_index and type
    offset = checked_add(offset, 8, data.len())?;

    // name, vendor, description, version, serial, location
    for _ in 0..6 {
        offset = skip_string(data, offset)?;
    }

    // modes
    let num_modes = read_u16(data, &mut offset)?;
    for _ in 0..num_modes {
        offset = skip_mode(data, offset)?;
    }

    // zones
    let num_zones = read_u16(data, &mut offset)?;
    let mut zones = Vec::with_capacity(num_zones as usize);
    for _ in 0..num_zones {
        let name = read_string(data, &mut offset)?;
        let _zone_type = read_u32(data, &mut offset)?;
        let _leds_min = read_u32(data, &mut offset)?;
        let _leds_max = read_u32(data, &mut offset)?;
        let leds_count = read_u32(data, &mut offset)?;
        let matrix_size = read_u32(data, &mut offset)?;
        offset = checked_add(offset, matrix_size as usize * 4, data.len())?;
        let _width = read_u32(data, &mut offset)?;
        let _height = read_u32(data, &mut offset)?;
        let _pad = read_u32(data, &mut offset)?;
        zones.push(ZoneData {
            name,
            led_count: leds_count,
        });
    }

    Ok(ControllerData { zones })
}

fn skip_mode(data: &[u8], mut offset: usize) -> Result<usize, OpenRgbError> {
    skip_string(data, offset)?;
    // value, flags, speed_min, speed_max, color_mode
    offset = checked_add(offset, 5 * 4 + 2, data.len())?;
    let num_colors = read_u16(data, &mut offset)?;
    offset = checked_add(offset, num_colors as usize * 3, data.len())?;
    // speed, direction
    offset = checked_add(offset, 2 * 4, data.len())?;
    Ok(offset)
}

fn read_string(data: &[u8], offset: &mut usize) -> Result<String, OpenRgbError> {
    let start = *offset;
    while *offset < data.len() && data[*offset] != 0 {
        *offset += 1;
    }
    if *offset >= data.len() {
        return Err(OpenRgbError::Protocol("unterminated string".into()));
    }
    let s = String::from_utf8_lossy(&data[start..*offset]).to_string();
    *offset += 1; // skip null
    Ok(s)
}

fn skip_string(data: &[u8], mut offset: usize) -> Result<usize, OpenRgbError> {
    while offset < data.len() && data[offset] != 0 {
        offset += 1;
    }
    if offset >= data.len() {
        return Err(OpenRgbError::Protocol("unterminated string".into()));
    }
    Ok(offset + 1)
}

fn read_u16(data: &[u8], offset: &mut usize) -> Result<u16, OpenRgbError> {
    if *offset + 2 > data.len() {
        return Err(OpenRgbError::Protocol("truncated u16".into()));
    }
    let val = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    Ok(val)
}

fn read_u32(data: &[u8], offset: &mut usize) -> Result<u32, OpenRgbError> {
    if *offset + 4 > data.len() {
        return Err(OpenRgbError::Protocol("truncated u32".into()));
    }
    let val = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    Ok(val)
}

fn checked_add(offset: usize, add: usize, len: usize) -> Result<usize, OpenRgbError> {
    let new = offset + add;
    if new > len {
        return Err(OpenRgbError::Protocol("truncated data".into()));
    }
    Ok(new)
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
