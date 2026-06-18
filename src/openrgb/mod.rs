use std::net::SocketAddr;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

const OPENRGB_PROTOCOL_VERSION: u32 = 4;
const OPENRGB_MAGIC: &[u8; 4] = b"ORGB";

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
    RequestControllerCount = 0,
    RequestControllerData = 1,
    RequestProtocolVersion = 40,
    SetClientName = 50,
    UpdateLeds = 1050,
    UpdateZoneLeds = 1051,
    UpdateSingleLed = 1052,
    UpdateMode = 1101,
}

#[derive(Debug, Clone)]
pub struct ZoneData {
    pub name: String,
    pub led_count: u32,
}

#[derive(Debug, Clone)]
pub struct ModeData {
    pub name: String,
    pub value: u32,
    pub flags: u32,
    pub speed_min: u32,
    pub speed_max: u32,
    pub brightness_min: u32,
    pub brightness_max: u32,
    pub colors_min: u32,
    pub colors_max: u32,
    pub speed: u32,
    pub brightness: u32,
    pub direction: u32,
    pub color_mode: u32,
    pub colors: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct ControllerData {
    pub active_mode: u32,
    pub modes: Vec<ModeData>,
    pub zones: Vec<ZoneData>,
}

impl ControllerData {
    pub fn direct_mode(&self) -> Option<(usize, &ModeData)> {
        for candidate in ["Direct", "Custom", "Static"] {
            for (idx, mode) in self.modes.iter().enumerate() {
                if mode.name == candidate {
                    return Some((idx, mode));
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct OpenRgbClient {
    address: SocketAddr,
    dry_run: bool,
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
            return Ok(Connection::new_dry_run());
        }

        let stream = TcpStream::connect(self.address).await?;
        info!("connected to OpenRGB server at {}", self.address);
        let mut conn = Connection::new_real(stream);
        let protocol_version = conn.request_protocol_version().await?;
        conn.protocol_version = protocol_version;
        conn.set_client_name("lighthouse").await?;
        Ok(conn)
    }
}

#[derive(Debug)]
struct Packet {
    command: u32,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Connection {
    inner: ConnectionInner,
    protocol_version: u32,
}

#[derive(Debug)]
enum ConnectionInner {
    Real(TcpStream),
    DryRun,
}

impl Connection {
    fn new_real(stream: TcpStream) -> Self {
        Self {
            inner: ConnectionInner::Real(stream),
            protocol_version: OPENRGB_PROTOCOL_VERSION,
        }
    }

    fn new_dry_run() -> Self {
        Self {
            inner: ConnectionInner::DryRun,
            protocol_version: OPENRGB_PROTOCOL_VERSION,
        }
    }

    async fn send_command(&mut self, command: Command, device_id: u32, data: &[u8]) -> Result<(), OpenRgbError> {
        match &mut self.inner {
            ConnectionInner::DryRun => {
                info!(
                    "dry-run: command={:?} device={} len={}",
                    command,
                    device_id,
                    data.len()
                );
                Ok(())
            }
            ConnectionInner::Real(stream) => {
                let header = build_header(command as u32, device_id, data.len() as u32);
                stream.write_all(&header).await?;
                stream.write_all(data).await?;
                stream.flush().await?;
                Ok(())
            }
        }
    }

    async fn read_packet(&mut self) -> Result<Packet, OpenRgbError> {
        match &mut self.inner {
            ConnectionInner::DryRun => Err(OpenRgbError::DryRun),
            ConnectionInner::Real(stream) => {
                let mut header = [0u8; 16];
                stream.read_exact(&mut header).await?;

                if &header[0..4] != OPENRGB_MAGIC {
                    return Err(OpenRgbError::Protocol(
                        "invalid magic in response header".into(),
                    ));
                }

                let _device_id = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
                let command = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
                let data_size = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);

                let mut data = vec![0u8; data_size as usize];
                if data_size > 0 {
                    stream.read_exact(&mut data).await?;
                }

                Ok(Packet { command, data })
            }
        }
    }

    async fn request_protocol_version(&mut self) -> Result<u32, OpenRgbError> {
        self.send_command(
            Command::RequestProtocolVersion,
            0,
            &OPENRGB_PROTOCOL_VERSION.to_le_bytes(),
        )
        .await?;
        let packet = self.read_packet().await?;
        if packet.command != Command::RequestProtocolVersion as u32 {
            return Err(OpenRgbError::Protocol(format!(
                "expected protocol version, got command {}",
                packet.command
            )));
        }
        Ok(u32::from_le_bytes([
            packet.data[0], packet.data[1], packet.data[2], packet.data[3],
        ]))
    }

    async fn set_client_name(&mut self, name: &str) -> Result<(), OpenRgbError> {
        self.send_command(Command::SetClientName, 0, &(name.to_string() + "\0").into_bytes())
            .await
    }

    pub async fn request_controller_count(&mut self) -> Result<u32, OpenRgbError> {
        self.send_command(Command::RequestControllerCount, 0, &[])
            .await?;
        let packet = self.read_packet().await?;
        if packet.command != Command::RequestControllerCount as u32 {
            return Err(OpenRgbError::Protocol(format!(
                "expected controller count, got command {}",
                packet.command
            )));
        }
        Ok(u32::from_le_bytes([
            packet.data[0], packet.data[1], packet.data[2], packet.data[3],
        ]))
    }

    pub async fn query_controller_data(
        &mut self,
        device_id: u32,
    ) -> Result<ControllerData, OpenRgbError> {
        self.send_command(
            Command::RequestControllerData,
            device_id,
            &self.protocol_version.to_le_bytes(),
        )
        .await?;
        let packet = self.read_packet().await?;
        if packet.command != Command::RequestControllerData as u32 {
            return Err(OpenRgbError::Protocol(format!(
                "expected controller data, got command {}",
                packet.command
            )));
        }
        parse_controller_data(&packet.data, self.protocol_version)
    }

    pub async fn set_direct_mode(&mut self, device_id: u32, controller: &ControllerData) -> Result<(), OpenRgbError> {
        let (mode_idx, mode) = controller
            .direct_mode()
            .ok_or_else(|| OpenRgbError::Protocol("no Direct/Custom/Static mode found".into()))?;
        self.set_mode(device_id, mode_idx as u32, mode).await
    }

    pub async fn set_mode(
        &mut self,
        device_id: u32,
        mode_idx: u32,
        mode: &ModeData,
    ) -> Result<(), OpenRgbError> {
        let data = build_mode_description(mode_idx, mode, self.protocol_version);
        self.send_command(Command::UpdateMode, device_id, &data).await
    }

    pub async fn set_zone_color(
        &mut self,
        device_id: u32,
        zone_id: u32,
        led_count: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let num_colors = led_count;
        let data_size = 4 + 4 + 2 + num_colors as usize * 4;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(data_size as u32).to_le_bytes());
        data.extend_from_slice(&zone_id.to_le_bytes());
        data.extend_from_slice(&(num_colors as u16).to_le_bytes());
        let rgb = rgb_to_u32(color);
        for _ in 0..num_colors {
            data.extend_from_slice(&rgb.to_le_bytes());
        }
        self.send_command(Command::UpdateZoneLeds, device_id, &data)
            .await
    }

    pub async fn set_all_color(
        &mut self,
        device_id: u32,
        led_count: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let num_colors = led_count;
        let data_size = 4 + 2 + num_colors as usize * 4;
        let mut data = Vec::with_capacity(data_size);
        data.extend_from_slice(&(data_size as u32).to_le_bytes());
        data.extend_from_slice(&(num_colors as u16).to_le_bytes());
        let rgb = rgb_to_u32(color);
        for _ in 0..num_colors {
            data.extend_from_slice(&rgb.to_le_bytes());
        }
        self.send_command(Command::UpdateLeds, device_id, &data)
            .await
    }

    pub async fn set_single_color(
        &mut self,
        device_id: u32,
        led_index: u32,
        color: [u8; 3],
    ) -> Result<(), OpenRgbError> {
        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&led_index.to_le_bytes());
        data.extend_from_slice(&rgb_to_u32(color).to_le_bytes());
        self.send_command(Command::UpdateSingleLed, device_id, &data)
            .await
    }
}

fn build_header(command: u32, device_id: u32, data_size: u32) -> Vec<u8> {
    let mut header = Vec::with_capacity(16);
    header.extend_from_slice(OPENRGB_MAGIC);
    header.extend_from_slice(&device_id.to_le_bytes());
    header.extend_from_slice(&command.to_le_bytes());
    header.extend_from_slice(&data_size.to_le_bytes());
    header
}

fn rgb_to_u32(color: [u8; 3]) -> u32 {
    ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32)
}

fn read_string(data: &[u8], offset: &mut usize) -> Result<String, OpenRgbError> {
    let len = read_u16(data, offset)? as usize;
    if *offset + len > data.len() {
        return Err(OpenRgbError::Protocol("string exceeds data bounds".into()));
    }
    let value = String::from_utf8_lossy(&data[*offset..*offset + len - 1]).to_string();
    *offset += len;
    Ok(value)
}

fn skip_string(data: &[u8], offset: &mut usize) -> Result<(), OpenRgbError> {
    let len = read_u16(data, offset)? as usize;
    if *offset + len > data.len() {
        return Err(OpenRgbError::Protocol("string exceeds data bounds".into()));
    }
    *offset += len;
    Ok(())
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

fn parse_controller_data(
    data: &[u8],
    protocol_version: u32,
) -> Result<ControllerData, OpenRgbError> {
    let mut offset = 4usize; // data_size

    // device type
    offset += 4;

    // name
    skip_string(data, &mut offset)?;

    // vendor (protocol >= 1)
    if protocol_version >= 1 {
        skip_string(data, &mut offset)?;
    }

    // description, version, serial, location
    for _ in 0..4 {
        skip_string(data, &mut offset)?;
    }

    // modes
    let num_modes = read_u16(data, &mut offset)?;
    let active_mode = read_u32(data, &mut offset)?;
    let mut modes = Vec::with_capacity(num_modes as usize);
    for _ in 0..num_modes {
        let name = read_string(data, &mut offset)?;
        let value = read_u32(data, &mut offset)?;
        let flags = read_u32(data, &mut offset)?;
        let speed_min = read_u32(data, &mut offset)?;
        let speed_max = read_u32(data, &mut offset)?;
        let (brightness_min, brightness_max) = if protocol_version >= 3 {
            (
                read_u32(data, &mut offset)?,
                read_u32(data, &mut offset)?,
            )
        } else {
            (0, 0)
        };
        let colors_min = read_u32(data, &mut offset)?;
        let colors_max = read_u32(data, &mut offset)?;
        let speed = read_u32(data, &mut offset)?;
        let brightness = if protocol_version >= 3 {
            read_u32(data, &mut offset)?
        } else {
            0
        };
        let direction = read_u32(data, &mut offset)?;
        let color_mode = read_u32(data, &mut offset)?;
        let num_colors = read_u16(data, &mut offset)?;
        let mut colors = Vec::with_capacity(num_colors as usize);
        for _ in 0..num_colors {
            colors.push(read_u32(data, &mut offset)?);
        }
        modes.push(ModeData {
            name,
            value,
            flags,
            speed_min,
            speed_max,
            brightness_min,
            brightness_max,
            colors_min,
            colors_max,
            speed,
            brightness,
            direction,
            color_mode,
            colors,
        });
    }

    // zones
    let num_zones = read_u16(data, &mut offset)?;
    let mut zones = Vec::with_capacity(num_zones as usize);
    for _ in 0..num_zones {
        let name = read_string(data, &mut offset)?;
        let _zone_type = read_u32(data, &mut offset)?;
        let _leds_min = read_u32(data, &mut offset)?;
        let _leds_max = read_u32(data, &mut offset)?;
        let led_count = read_u32(data, &mut offset)?;
        let matrix_len = read_u16(data, &mut offset)? as usize;
        offset += matrix_len;
        if protocol_version >= 4 {
            let num_segments = read_u16(data, &mut offset)?;
            for _ in 0..num_segments {
                skip_string(data, &mut offset)?;
                offset += 3 * 4;
            }
        }
        if protocol_version >= 5 {
            offset += 4;
        }
        zones.push(ZoneData { name, led_count });
    }

    // leds
    let num_leds = read_u16(data, &mut offset)?;
    for _ in 0..num_leds {
        skip_string(data, &mut offset)?;
        offset += 4;
    }

    if protocol_version >= 5 {
        let num_alt_names = read_u16(data, &mut offset)?;
        for _ in 0..num_alt_names {
            skip_string(data, &mut offset)?;
        }
        offset += 4; // controller flags
    }

    // colors
    let num_colors = read_u16(data, &mut offset)?;
    let _ = num_colors;

    Ok(ControllerData {
        active_mode,
        modes,
        zones,
    })
}

fn build_mode_description(mode_idx: u32, mode: &ModeData, protocol_version: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&mode_idx.to_le_bytes());
    body.extend_from_slice(&string_to_length_prefixed(&mode.name));
    body.extend_from_slice(&mode.value.to_le_bytes());
    body.extend_from_slice(&mode.flags.to_le_bytes());
    body.extend_from_slice(&mode.speed_min.to_le_bytes());
    body.extend_from_slice(&mode.speed_max.to_le_bytes());
    if protocol_version >= 3 {
        body.extend_from_slice(&mode.brightness_min.to_le_bytes());
        body.extend_from_slice(&mode.brightness_max.to_le_bytes());
    }
    body.extend_from_slice(&mode.colors_min.to_le_bytes());
    body.extend_from_slice(&mode.colors_max.to_le_bytes());
    body.extend_from_slice(&mode.speed.to_le_bytes());
    if protocol_version >= 3 {
        body.extend_from_slice(&mode.brightness.to_le_bytes());
    }
    body.extend_from_slice(&mode.direction.to_le_bytes());
    body.extend_from_slice(&mode.color_mode.to_le_bytes());
    body.extend_from_slice(&(mode.colors.len() as u16).to_le_bytes());
    for c in &mode.colors {
        body.extend_from_slice(&c.to_le_bytes());
    }

    let mut data = Vec::with_capacity(4 + body.len());
    data.extend_from_slice(&(4 + body.len() as u32).to_le_bytes());
    data.extend_from_slice(&body);
    data
}

fn string_to_length_prefixed(value: &str) -> Vec<u8> {
    let bytes = (value.to_string() + "\0").into_bytes();
    let mut out = Vec::with_capacity(2 + bytes.len());
    out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    out.extend_from_slice(&bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_format() {
        let h = build_header(1051, 0, 30);
        assert_eq!(h.len(), 16);
        assert_eq!(&h[0..4], b"ORGB");
        assert_eq!(&h[4..8], &[0, 0, 0, 0]); // device_id
        assert_eq!(&h[8..12], &[0x1b, 0x04, 0x00, 0x00]); // command 1051
        assert_eq!(&h[12..16], &[30, 0, 0, 0]); // data_size
    }

    #[test]
    fn set_zone_color_payload() {
        let mut conn = Connection::new_dry_run();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            conn.set_zone_color(0, 1, 4, [255, 0, 0]).await.unwrap();
        });
    }

    #[test]
    fn set_all_color_payload() {
        let mut conn = Connection::new_dry_run();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            conn.set_all_color(0, 2, [255, 0, 0]).await.unwrap();
        });
    }
}
