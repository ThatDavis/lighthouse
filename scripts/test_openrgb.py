#!/usr/bin/env python3
"""Minimal OpenRGB SDK test client.

Use this to verify that the OpenRGB server responds to commands the same
way Lighthouse expects, without rebuilding the Rust binary.
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

CMD_REQUEST_PROTOCOL_VERSION = 0
CMD_REQUEST_CONTROLLER_COUNT = 1
CMD_REQUEST_CONTROLLER_DATA = 2
CMD_SET_CLIENT_NAME = 50
CMD_UPDATE_LEDS = 1050
CMD_UPDATE_ZONE_LEDS = 1051
CMD_UPDATE_SINGLE_LED = 1052
CMD_UPDATE_MODE = 1100

MODE_BEGIN = 0
MODE_SET = 1
MODE_END = 2


def send_command(sock, command, mode, device_id, data):
    header = struct.pack("<IIII", command, len(data), device_id, mode)
    sock.sendall(header + data)
    print(f"--> command={command} mode={mode} device={device_id} len={len(data)}")


def recv_packet(sock):
    header = sock.recv(16)
    if len(header) != 16:
        raise RuntimeError(f"short header: {len(header)} bytes")
    command, data_size, device_id, mode = struct.unpack("<IIII", header)
    data = b""
    while len(data) < data_size:
        chunk = sock.recv(data_size - len(data))
        if not chunk:
            raise RuntimeError("connection closed while reading packet data")
        data += chunk
    print(f"<-- command={command} mode={mode} device={device_id} len={len(data)}")
    return command, device_id, mode, data


def read_string(data, offset):
    end = data.find(b"\0", offset)
    if end == -1:
        raise RuntimeError("unterminated string")
    value = data[offset:end].decode("utf-8", errors="replace")
    return value, end + 1


def read_u16(data, offset):
    return struct.unpack_from("<H", data, offset)[0], offset + 2


def read_u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0], offset + 4


def parse_controller_data(data):
    offset = 8  # device_index + type

    for _ in range(6):  # name, vendor, description, version, serial, location
        _, offset = read_string(data, offset)

    num_modes, offset = read_u16(data, offset)
    for _ in range(num_modes):
        _, offset = read_string(data, offset)
        # value, flags, speed_min, speed_max, color_mode
        offset += 5 * 4 + 2
        num_colors, offset = read_u16(data, offset)
        offset += num_colors * 3
        offset += 2 * 4  # speed, direction

    num_zones, offset = read_u16(data, offset)
    zones = []
    for _ in range(num_zones):
        name, offset = read_string(data, offset)
        zone_type, offset = read_u32(data, offset)
        leds_min, offset = read_u32(data, offset)
        leds_max, offset = read_u32(data, offset)
        leds_count, offset = read_u32(data, offset)
        matrix_size, offset = read_u32(data, offset)
        offset += matrix_size * 4
        width, offset = read_u32(data, offset)
        height, offset = read_u32(data, offset)
        pad, offset = read_u32(data, offset)
        zones.append({
            "name": name,
            "type": zone_type,
            "led_count": leds_count,
            "matrix_size": matrix_size,
            "width": width,
            "height": height,
        })

    return zones


def set_client_name(sock, name):
    send_command(sock, CMD_SET_CLIENT_NAME, MODE_BEGIN, 0, (name + "\0").encode())


def request_controller_count(sock):
    send_command(sock, CMD_REQUEST_CONTROLLER_COUNT, MODE_BEGIN, 0, b"")
    command, _, _, data = recv_packet(sock)
    if command != CMD_REQUEST_CONTROLLER_COUNT:
        raise RuntimeError(f"unexpected response command {command}")
    return struct.unpack("<I", data[:4])[0]


def request_controller_data(sock, device_id):
    send_command(
        sock,
        CMD_REQUEST_CONTROLLER_DATA,
        MODE_BEGIN,
        device_id,
        struct.pack("<I", PROTOCOL_VERSION),
    )
    command, _, _, data = recv_packet(sock)
    if command != CMD_REQUEST_CONTROLLER_DATA:
        raise RuntimeError(f"unexpected response command {command}")
    return parse_controller_data(data)


def set_device_mode(sock, device_id, mode_index):
    send_command(sock, CMD_UPDATE_MODE, MODE_SET, device_id, struct.pack("<I", mode_index))


def set_zone_color(sock, device_id, zone_id, led_count, color):
    data = struct.pack("<I", zone_id) + bytes(color) * led_count
    send_command(sock, CMD_UPDATE_ZONE_LEDS, MODE_SET, device_id, data)


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

    set_client_name(sock, "lighthouse-test")

    print("Requesting controller count ...")
    count = request_controller_count(sock)
    print(f"Controller count: {count}\n")

    print(f"Requesting controller data for device {args.device} ...")
    zones = request_controller_data(sock, args.device)
    print(f"Zones on device {args.device}:")
    for idx, zone in enumerate(zones):
        print(f"  zone {idx}: {zone['name']!r} ({zone['led_count']} leds)")
    print()

    print("Setting Direct mode (mode index 0) ...")
    set_device_mode(sock, args.device, 0)
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
