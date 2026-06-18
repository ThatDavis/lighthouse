#!/usr/bin/env python3
"""Minimal OpenRGB SDK test client.

Use this to verify that the OpenRGB server responds to commands the same
way Lighthouse expects, without rebuilding the Rust binary.

This follows the real OpenRGB SDK protocol:
  * every packet is prefixed with the magic bytes b"ORGB"
  * packet header layout: magic | device_id | command | data_size
  * strings are length-prefixed (u16)
  * LED update payloads are size-prefixed
"""

import argparse
import socket
import struct
import sys
import time

HOST = "127.0.0.1"
PORT = 6742
PROTOCOL_VERSION = 4
TIMEOUT = 5

MAGIC = b"ORGB"

CMD_REQUEST_CONTROLLER_COUNT = 0
CMD_REQUEST_CONTROLLER_DATA = 1
CMD_REQUEST_PROTOCOL_VERSION = 40
CMD_SET_CLIENT_NAME = 50
CMD_UPDATE_LEDS = 1050
CMD_UPDATE_ZONE_LEDS = 1051
CMD_UPDATE_SINGLE_LED = 1052
CMD_UPDATE_MODE = 1101


def send_command(sock, command, device_id, data):
    header = MAGIC + struct.pack("<III", device_id, command, len(data))
    sock.sendall(header + data)
    print(f"--> command={command} device={device_id} len={len(data)}")


def recv_packet(sock):
    header = b""
    while len(header) < 16:
        chunk = sock.recv(16 - len(header))
        if not chunk:
            raise RuntimeError("connection closed while reading header")
        header += chunk
    _, _, command, data_size = struct.unpack("<4sIII", header)
    data = b""
    while len(data) < data_size:
        chunk = sock.recv(data_size - len(data))
        if not chunk:
            raise RuntimeError("connection closed while reading packet data")
        data += chunk
    print(f"<-- command={command} len={len(data)}")
    return command, data


def read_string(data, offset):
    length = struct.unpack_from("<H", data, offset)[0]
    offset += 2
    end = offset + length
    if end > len(data):
        raise RuntimeError("string exceeds data bounds")
    value = data[offset:end - 1].decode("utf-8", errors="replace")
    return value, end


def read_u16(data, offset):
    return struct.unpack_from("<H", data, offset)[0], offset + 2


def read_u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0], offset + 4


def skip_string(data, offset):
    length = struct.unpack_from("<H", data, offset)[0]
    return offset + 2 + length


def write_string(value):
    encoded = (value + "\0").encode("utf-8")
    return struct.pack("<H", len(encoded)) + encoded


def parse_controller_data(data, protocol_version):
    offset = 4  # data_size

    # device type
    offset += 4

    # name
    offset = skip_string(data, offset)

    # vendor (protocol >= 1)
    if protocol_version >= 1:
        offset = skip_string(data, offset)

    # description, version, serial, location
    for _ in range(4):
        offset = skip_string(data, offset)

    # modes
    num_modes, offset = read_u16(data, offset)
    active_mode, offset = read_u32(data, offset)
    modes = []
    for _ in range(num_modes):
        name, offset = read_string(data, offset)
        value, offset = read_u32(data, offset)
        flags, offset = read_u32(data, offset)
        speed_min, offset = read_u32(data, offset)
        speed_max, offset = read_u32(data, offset)
        brightness_min = 0
        brightness_max = 0
        brightness = 0
        if protocol_version >= 3:
            brightness_min, offset = read_u32(data, offset)
            brightness_max, offset = read_u32(data, offset)
        colors_min, offset = read_u32(data, offset)
        colors_max, offset = read_u32(data, offset)
        speed, offset = read_u32(data, offset)
        if protocol_version >= 3:
            brightness, offset = read_u32(data, offset)
        direction, offset = read_u32(data, offset)
        color_mode, offset = read_u32(data, offset)
        num_colors, offset = read_u16(data, offset)
        colors = []
        for _ in range(num_colors):
            c, offset = read_u32(data, offset)
            colors.append(c)
        modes.append({
            "name": name,
            "value": value,
            "flags": flags,
            "speed_min": speed_min,
            "speed_max": speed_max,
            "brightness_min": brightness_min,
            "brightness_max": brightness_max,
            "colors_min": colors_min,
            "colors_max": colors_max,
            "speed": speed,
            "brightness": brightness,
            "direction": direction,
            "color_mode": color_mode,
            "colors": colors,
        })

    # zones
    num_zones, offset = read_u16(data, offset)
    zones = []
    for _ in range(num_zones):
        name, offset = read_string(data, offset)
        zone_type, offset = read_u32(data, offset)
        leds_min, offset = read_u32(data, offset)
        leds_max, offset = read_u32(data, offset)
        leds_count, offset = read_u32(data, offset)
        matrix_len, offset = read_u16(data, offset)
        offset += matrix_len
        if protocol_version >= 4:
            num_segments, offset = read_u16(data, offset)
            for _ in range(num_segments):
                offset = skip_string(data, offset)
                offset += 3 * 4
        if protocol_version >= 5:
            offset += 4  # zone flags
        zones.append({
            "name": name,
            "type": zone_type,
            "led_count": leds_count,
        })

    return {"active_mode": active_mode, "modes": modes, "zones": zones}


def find_direct_mode(modes):
    for candidate in ("Direct", "Custom", "Static"):
        for idx, mode in enumerate(modes):
            if mode["name"] == candidate:
                return idx
    return None


def build_mode_description(mode_idx, mode, protocol_version):
    body = struct.pack("<I", mode_idx)
    body += write_string(mode["name"])
    body += struct.pack(
        "<IIII",
        mode["value"],
        mode["flags"],
        mode["speed_min"],
        mode["speed_max"],
    )
    if protocol_version >= 3:
        body += struct.pack("<II", mode["brightness_min"], mode["brightness_max"])
    body += struct.pack("<II", mode["colors_min"], mode["colors_max"])
    body += struct.pack("<I", mode["speed"])
    if protocol_version >= 3:
        body += struct.pack("<I", mode["brightness"])
    body += struct.pack("<II", mode["direction"], mode["color_mode"])
    body += struct.pack("<H", len(mode["colors"]))
    for c in mode["colors"]:
        body += struct.pack("<I", c)
    return struct.pack("<I", 4 + len(body)) + body


def set_client_name(sock, name):
    send_command(sock, CMD_SET_CLIENT_NAME, 0, (name + "\0").encode())


def request_protocol_version(sock):
    send_command(sock, CMD_REQUEST_PROTOCOL_VERSION, 0, struct.pack("<I", PROTOCOL_VERSION))
    command, data = recv_packet(sock)
    if command != CMD_REQUEST_PROTOCOL_VERSION:
        raise RuntimeError(f"unexpected response command {command}")
    return struct.unpack("<I", data[:4])[0]


def request_controller_count(sock):
    send_command(sock, CMD_REQUEST_CONTROLLER_COUNT, 0, b"")
    command, data = recv_packet(sock)
    if command != CMD_REQUEST_CONTROLLER_COUNT:
        raise RuntimeError(f"unexpected response command {command}")
    return struct.unpack("<I", data[:4])[0]


def request_controller_data(sock, device_id, protocol_version):
    send_command(
        sock,
        CMD_REQUEST_CONTROLLER_DATA,
        device_id,
        struct.pack("<I", protocol_version),
    )
    command, data = recv_packet(sock)
    if command != CMD_REQUEST_CONTROLLER_DATA:
        raise RuntimeError(f"unexpected response command {command}")
    return parse_controller_data(data, protocol_version)


def set_mode(sock, device_id, mode_data, protocol_version):
    send_command(sock, CMD_UPDATE_MODE, device_id, mode_data)


def rgbcolor(color):
    r, g, b = color
    return struct.pack("<I", (r << 16) | (g << 8) | b)


def set_zone_color(sock, device_id, zone_id, led_count, color):
    num_colors = led_count
    data_size = 4 + 4 + 2 + num_colors * 4
    data = struct.pack("<IIH", data_size, zone_id, num_colors)
    data += rgbcolor(color) * num_colors
    send_command(sock, CMD_UPDATE_ZONE_LEDS, device_id, data)


def color_cycle():
    while True:
        for color in [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 0),
            (0, 255, 255),
            (255, 0, 255),
            (255, 255, 255),
        ]:
            yield color


def main():
    parser = argparse.ArgumentParser(description="Test OpenRGB SDK protocol")
    parser.add_argument("--host", default=HOST, help="OpenRGB server host")
    parser.add_argument("--port", type=int, default=PORT, help="OpenRGB server port")
    parser.add_argument("--device", type=int, default=0, help="Device ID")
    parser.add_argument(
        "--zones", type=int, nargs="+", default=None, help="Zone IDs to update"
    )
    parser.add_argument(
        "--cycle", action="store_true", help="Cycle zone colors after querying"
    )
    parser.add_argument(
        "--delay", type=float, default=2.0, help="Seconds between cycle colors"
    )
    args = parser.parse_args()

    print(f"Connecting to {args.host}:{args.port} ...")
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(TIMEOUT)
    try:
        sock.connect((args.host, args.port))
    except socket.error as e:
        print(f"Connection failed: {e}", file=sys.stderr)
        return 1

    print("Connected.\n")

    print("Negotiating protocol version ...")
    negotiated = request_protocol_version(sock)
    print(f"Server protocol version: {negotiated}\n")

    set_client_name(sock, "lighthouse-test")

    print("Requesting controller count ...")
    count = request_controller_count(sock)
    print(f"Controller count: {count}\n")

    if args.device >= count:
        print(f"Device {args.device} not found (count={count})", file=sys.stderr)
        return 1

    print(f"Requesting controller data for device {args.device} ...")
    controller = request_controller_data(sock, args.device, negotiated)
    zones = controller["zones"]
    print(f"Zones on device {args.device}:")
    for idx, zone in enumerate(zones):
        print(f"  zone {idx}: {zone['name']!r} ({zone['led_count']} leds)")
    print()

    direct_idx = find_direct_mode(controller["modes"])
    if direct_idx is None:
        print("No Direct/Custom/Static mode found; aborting", file=sys.stderr)
        return 1

    print(f"Switching to mode {direct_idx}: {controller['modes'][direct_idx]['name']!r} ...")
    mode_data = build_mode_description(direct_idx, controller["modes"][direct_idx], negotiated)
    set_mode(sock, args.device, mode_data, negotiated)
    print()

    zone_ids = args.zones
    if zone_ids is None:
        zone_ids = list(range(len(zones)))

    if args.cycle:
        print(f"Cycling colors on zones {zone_ids} (Ctrl+C to stop) ...\n")
        for color in color_cycle():
            print(f"Setting color rgb{color}")
            for zone_id in zone_ids:
                led_count = zones[zone_id]["led_count"] if zone_id < len(zones) else 1
                if led_count == 0:
                    led_count = 1
                set_zone_color(sock, args.device, zone_id, led_count, color)
            time.sleep(args.delay)

    return 0


if __name__ == "__main__":
    sys.exit(main())
