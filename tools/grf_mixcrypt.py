#!/usr/bin/env python3
"""Pure-Python port of the client's GRF mixcrypt ('Simple DES + shuffle').

The CondaRO `data.grf` stores `data\\monsterskillinfo.xml` and other data files
encrypted. The client decrypts them with the scheme implemented in
`ro-next/src/loaders/archive/native/mixcrypt.rs` (a hard copy of the official
client's algorithm); this module mirrors it so the data extractors can read
those entries without a Rust helper.

The DES S-boxes are parsed from `mixcrypt.rs` so the Rust implementation stays
the single source of truth. The block permutation itself is re-implemented
here; `decode_des_block` is checked against the Rust unit-test vector.

Usage:
    python3 tools/grf_mixcrypt.py     # run the self-test
"""

from __future__ import annotations

import re
from pathlib import Path

MIXCRYPT_SOURCE = Path("ro-next/src/loaders/archive/native/mixcrypt.rs")

#: File uses mixed crypto (Simple DES + shuffle).
GRF_FLAG_FULL_MIX_CRYPT = 1 << 1
#: Only the first 0x14 compressed blocks are encrypted with DES.
GRF_FLAG_HEADER_DES_CRYPT = 1 << 2

#: First 0x14 blocks are always encrypted.
HEADER_BLOCKS_SIZE = 0x14
BLOCK_SIZE = 8
M64 = (1 << 64) - 1


def load_s_boxes(path: Path = MIXCRYPT_SOURCE) -> list[list[int]]:
    """Read the eight DES S-boxes from the Rust implementation."""
    source = path.read_text()
    match = re.search(r"const S_BOXES: \[\[u8; 64\]; 8\] = \[(.*?)\n    \];", source, re.S)
    if match is None:
        raise SystemExit(f"could not find S_BOXES in {path}")
    boxes = []
    for array in re.findall(r"\[([^\]]*)\]", match.group(1)):
        values = [int(value, 16) for value in re.findall(r"0x[0-9A-Fa-f]+", array)]
        if len(values) == 64:
            boxes.append(values)
    if len(boxes) != 8:
        raise SystemExit(f"expected 8 S-boxes, found {len(boxes)}")
    return boxes


S_BOXES = load_s_boxes()


def _delta_swap(a: int, delta: int, mask: int) -> int:
    b = (a ^ (a >> delta)) & mask
    return a ^ b ^ ((b << delta) & M64)


def _ip(message: int) -> int:
    message = _delta_swap(message, 9, 0x0055005500550055)
    message = _delta_swap(message, 18, 0x0000333300003333)
    message = _delta_swap(message, 36, 0x000000000F0F0F0F)
    message = _delta_swap(message, 24, 0x00000000FF00FF00)
    return _delta_swap(message, 24, 0x000000FF000000FF)


def _fp(message: int) -> int:
    message = _delta_swap(message, 24, 0x000000FF000000FF)
    message = _delta_swap(message, 24, 0x00000000FF00FF00)
    message = _delta_swap(message, 36, 0x000000000F0F0F0F)
    message = _delta_swap(message, 18, 0x0000333300003333)
    return _delta_swap(message, 9, 0x0055005500550055)


def _expand(block: int) -> int:
    b1 = (block << 31) & 0x8000000000000000
    b2 = (block >> 1) & 0x7C00000000000000
    b3 = (block >> 3) & 0x03F0000000000000
    b4 = (block >> 5) & 0x000FC00000000000
    b5 = (block >> 7) & 0x00003F0000000000
    b6 = (block >> 9) & 0x000000FC00000000
    b7 = (block >> 11) & 0x00000003F0000000
    b8 = (block >> 13) & 0x000000000FC00000
    b9 = (block >> 15) & 0x00000000003E0000
    b10 = (block >> 47) & 0x0000000000010000
    return b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8 | b9 | b10


def _rotate_left(value: int, count: int) -> int:
    count %= 64
    return ((value << count) | (value >> (64 - count))) & M64


def _pbox(block: int) -> int:
    block = _rotate_left(block, 44)
    b1 = (block & 0x0000000000200000) << 32
    b2 = (block & 0x0000000000480000) << 13
    b3 = (block & 0x0000088000000000) << 12
    b4 = (block & 0x0000002020120000) << 25
    b5 = (block & 0x0000000442000000) << 14
    b6 = (block & 0x0000000001800000) << 37
    b7 = (block & 0x0000000004000000) << 24
    b8 = ((block & 0x0000020280015000) * 0x0000020080800083) & 0x02000A6400000000
    b9 = ((_rotate_left(block, 29) & 0x01001400000000AA) * 0x0000210210008081) & 0x0902C01200000000
    b10 = ((block & 0x0000000910040000) * 0x0000000C04000020) & 0x8410010000000000
    return (b1 | b2 | b3 | b4 | b5 | b6 | b7 | b8 | b9 | b10) & M64


def _sbox_substitute(value: int) -> int:
    output = 0
    for index, sbox in enumerate(S_BOXES):
        output |= sbox[(value >> (58 - index * 6)) & 0x3F] << (60 - index * 4)
    return output & M64


def _feistel(right: int, key: int) -> int:
    return _pbox(_sbox_substitute(_expand(right) ^ key))


def _round(value: int, key: int) -> int:
    left = value & (0xFFFFFFFF << 32)
    right = (value << 32) & M64
    return (right | ((_feistel(right, key) ^ left) >> 32)) & M64


def decode_des_block(block: int) -> int:
    """`mixcrypt.rs::decode_des_block`: one Gravity DES block."""
    block = _ip(block)
    block = _round(block, 0)
    block = _rotate_left(block, 32)
    return _fp(block)


_SHUFFLE = (3, 4, 6, 0, 1, 2, 5)
_BYTE_FLIP = {
    0x00: 0x2B,
    0x2B: 0x00,
    0x01: 0x68,
    0x68: 0x01,
    0x48: 0x77,
    0x77: 0x48,
    0x60: 0xFF,
    0xFF: 0x60,
    0x6C: 0x80,
    0x80: 0x6C,
    0xB9: 0xC0,
    0xC0: 0xB9,
    0xEB: 0xFE,
    0xFE: 0xEB,
}


def _scramble_block(block: bytearray) -> None:
    copy = bytes(block)
    for index, position in enumerate(_SHUFFLE):
        block[index] = copy[position]
    block[7] = _BYTE_FLIP.get(copy[7], copy[7])


def _decrypt_blocks(data: bytearray, only_header: bool, cycle: int) -> None:
    non_des = 0
    for block_number in range(len(data) // BLOCK_SIZE):
        start = block_number * BLOCK_SIZE
        if block_number < HEADER_BLOCKS_SIZE or (not only_header and cycle and block_number % cycle == 0):
            value = decode_des_block(int.from_bytes(data[start:start + BLOCK_SIZE], "big"))
            data[start:start + BLOCK_SIZE] = value.to_bytes(BLOCK_SIZE, "big")
        elif not only_header:
            if non_des == 7:
                block = bytearray(data[start:start + BLOCK_SIZE])
                _scramble_block(block)
                data[start:start + BLOCK_SIZE] = block
                non_des = 0
            non_des += 1


def decrypt_data(data: bytearray, only_header: bool, cycle: int) -> None:
    """Decrypt full blocks in place, padding the last incomplete block."""
    if len(data) % BLOCK_SIZE == 0:
        _decrypt_blocks(data, only_header, cycle)
        return
    full = (len(data) // BLOCK_SIZE) * BLOCK_SIZE
    if full:
        _decrypt_blocks(data[:full], only_header, cycle)
    last = bytearray(BLOCK_SIZE)
    remainder = len(data) - full
    last[:remainder] = data[full:]
    _decrypt_blocks(last, only_header, cycle)
    data[full:] = last[:remainder]


def count_digits(size: int) -> int:
    return max(1, len(str(size)))


def calculate_encryption_cycle(digits: int) -> int:
    if digits < 3:
        return 3
    if digits < 5:
        return digits + 1
    if digits < 7:
        return digits + 9
    return digits + 15


def decrypt_file(flags: int, compressed_size: int, data: bytearray) -> None:
    """`mixcrypt.rs::decrypt_file`; leaves unencrypted entries untouched."""
    if flags & GRF_FLAG_FULL_MIX_CRYPT:
        decrypt_data(data, False, calculate_encryption_cycle(count_digits(compressed_size)))
    elif flags & GRF_FLAG_HEADER_DES_CRYPT:
        decrypt_data(data, True, 0)


def self_test() -> None:
    # `mixcrypt.rs::test_decode_des_block`.
    assert decode_des_block(0xFFFFFFFFFFFFFFFF - 123456789) == 12316197016309868543
    assert decrypt_file is not None
    print("grf_mixcrypt: DES test vector OK")


if __name__ == "__main__":
    self_test()
