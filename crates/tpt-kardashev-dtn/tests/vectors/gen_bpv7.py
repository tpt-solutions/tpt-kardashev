# SPDX-License-Identifier: MIT OR Apache-2.0
"""Independent encoder for the byte-exact BPv7 vectors in bpv7.txt.

Written from RFC 9171 (the CDDL of its Appendix B and the block and CRC rules of its section 4), RFC 8949
(CBOR shortest-form heads, indefinite-length arrays) and the CRC definitions (CRC-16/X-25 and CRC-32C).
It does not read or copy the crate's encoder: the Rust test decodes these bytes and must re-encode them to
the same bytes. Run from this directory:

    python gen_bpv7.py > bpv7.txt
"""

import sys

# --- CBOR (RFC 8949): shortest-form heads, definite and indefinite arrays ---------------------------------

MAJOR_UINT, MAJOR_BYTES, MAJOR_TEXT, MAJOR_ARRAY = 0, 2, 3, 4


class Indefinite(list):
    """An indefinite-length array (RFC 8949 section 3.2.2)."""


def head(major, arg):
    if arg < 24:
        return bytes([(major << 5) | arg])
    for width, marker in ((1, 24), (2, 25), (4, 26), (8, 27)):
        if arg < (1 << (8 * width)):
            return bytes([(major << 5) | marker]) + arg.to_bytes(width, "big")
    raise ValueError("argument too large")


def cbor(item):
    if item is None:
        return bytes([0xF6])
    if isinstance(item, bool):
        return bytes([0xF5 if item else 0xF4])
    if isinstance(item, int):
        return head(MAJOR_UINT, item)
    if isinstance(item, bytes):
        return head(MAJOR_BYTES, len(item)) + item
    if isinstance(item, str):
        raw = item.encode("utf-8")
        return head(MAJOR_TEXT, len(raw)) + raw
    if isinstance(item, Indefinite):
        return b"\x9f" + b"".join(cbor(x) for x in item) + b"\xff"
    if isinstance(item, list):
        return head(MAJOR_ARRAY, len(item)) + b"".join(cbor(x) for x in item)
    raise TypeError(type(item))


# --- CRCs, bit by bit from the definitions ---------------------------------------------------------------------


def crc16_x25(data):
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ 0x8408 if crc & 1 else crc >> 1
    return crc ^ 0xFFFF


def crc32c(data):
    crc = 0xFFFFFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ 0x82F63B78 if crc & 1 else crc >> 1
    return crc ^ 0xFFFFFFFF


CRC_NONE, CRC_16, CRC_32C = 0, 1, 2
CRC_SIZE = {CRC_NONE: 0, CRC_16: 2, CRC_32C: 4}


def crc_of(crc_type, data):
    if crc_type == CRC_16:
        return crc16_x25(data).to_bytes(2, "big")
    if crc_type == CRC_32C:
        return crc32c(data).to_bytes(4, "big")
    raise ValueError(crc_type)


# --- EIDs (RFC 9171 section 4.2.5.1): dtn scheme 1, ipn scheme 2 -----------------------------------------------


def dtn_eid(ssp):
    return [1, ssp]


def ipn_eid(node, service):
    return [2, [node, service]]


NULL_EID = [1, 0]


# --- Blocks: the CBOR encoding is computed with a zero CRC value, then the CRC is put in (RFC 9171 4.2.1) ------


def encode_with_crc(fields, crc_type):
    """fields is the block array without its CRC; the CRC value is appended and covers the zeroed form."""
    if crc_type == CRC_NONE:
        return cbor(fields)
    size = CRC_SIZE[crc_type]
    zeroed = cbor(fields + [bytes(size)])
    value = crc_of(crc_type, zeroed)
    return cbor(fields + [value])


def primary_block(flags, crc_type, dest, src, report_to, created, lifetime, fragment=None):
    fields = [7, flags, crc_type, dest, src, report_to, list(created), lifetime]
    if fragment is not None:
        fields += [fragment[0], fragment[1]]
    return encode_with_crc(fields, crc_type)


def canonical_block(block_type, number, flags, crc_type, data):
    return encode_with_crc([block_type, number, flags, crc_type, data], crc_type)


def cbor_bundle(primary, blocks):
    """Bundle = indefinite-length array of the primary block, the extension blocks and the payload block last
    (RFC 9171 section 4.1 and its CDDL). The blocks are already encoded."""
    return b"\x9f" + primary + b"".join(blocks) + b"\xff"


# --- Custody (private-use codes, this crate's choice; RFC 9171 section 9 reserves 192-255) ---------------------


def custody_signal_record(succeeded, reason, signal_time, source, ctime, cseq, fragment_offset):
    status = (0x80 if succeeded else 0) | (reason & 0x7F)
    frag = 0 if fragment_offset is None else 1
    return [192, [status, signal_time, source, ctime, cseq, frag, fragment_offset or 0]]  # [type, content]


# --- The vectors -------------------------------------------------------------------------------------------------


def vectors():
    out = []

    # 1. Minimal bundle, CRC-32C everywhere, ipn endpoints.
    p = primary_block(0, CRC_32C, ipn_eid(2, 1), ipn_eid(1, 1), NULL_EID, (1000, 1), 60000)
    b = canonical_block(1, 1, 0, CRC_32C, b"hello")
    out.append(("min_ipn_crc32c", cbor_bundle(p, [b])))

    # 2. dtn endpoints with CRC-16/X-25 and a longer payload (a two-byte byte-string length).
    p = primary_block(
        0, CRC_16, dtn_eid("//node2/inbox"), dtn_eid("//node1/"), dtn_eid("//node1/reports"),
        (2000, 7), 3600000,
    )
    payload = bytes((i * 37) % 256 for i in range(300))
    b = canonical_block(1, 1, 0, CRC_16, payload)
    out.append(("dtn_crc16_long_payload", cbor_bundle(p, [b])))

    # 3. No CRC at all: the CRC-type fields are zero and no CRC values appear.
    p = primary_block(0, CRC_NONE, ipn_eid(4, 2), ipn_eid(3, 2), NULL_EID, (5, 3), 1000)
    b = canonical_block(1, 1, 0, CRC_NONE, b"nocrc")
    out.append(("no_crc", cbor_bundle(p, [b])))

    # 4. A fragment: the bundle-is-a-fragment flag (bit 0) and the two fragment fields.
    p = primary_block(
        1, CRC_32C, ipn_eid(2, 1), ipn_eid(1, 1), NULL_EID, (1000, 9), 60000, fragment=(100, 1000),
    )
    b = canonical_block(1, 1, 0, CRC_32C, bytes(range(50)))
    out.append(("fragment_offset_100_of_1000", cbor_bundle(p, [b])))

    # 5. Extension blocks: Previous Node (6), Bundle Age (7), Hop Count (10), before the payload.
    p = primary_block(0, CRC_32C, ipn_eid(2, 1), ipn_eid(1, 1), NULL_EID, (1000, 2), 60000)
    prev = canonical_block(6, 2, 0, CRC_32C, cbor(ipn_eid(5, 1)))  # block data is a bstr (RFC 9171 CDDL)
    age = canonical_block(7, 3, 0, CRC_32C, cbor(4000))
    hops = canonical_block(10, 4, 0, CRC_32C, cbor([16, 3]))
    b = canonical_block(1, 1, 0, CRC_32C, b"with extensions")
    out.append(("extension_blocks", cbor_bundle(p, [prev, age, hops, b])))

    # 6. An administrative record (bit 1) whose payload is a custody signal (this crate's record type 192).
    p = primary_block(2, CRC_32C, ipn_eid(1, 1), ipn_eid(2, 1), NULL_EID, (1000, 11), 600000)
    rec = custody_signal_record(True, 0, 500, 2, 1000, 9, None)
    b = canonical_block(1, 1, 0, CRC_32C, cbor(rec))
    out.append(("custody_signal_accepted", cbor_bundle(p, [b])))

    # 7. A refused custody signal for a fragment (depleted storage, reason 4).
    p = primary_block(2, CRC_32C, ipn_eid(1, 1), ipn_eid(3, 1), NULL_EID, (1000, 12), 600000)
    rec = custody_signal_record(False, 4, 777, 3, 1000, 9, 500)
    b = canonical_block(1, 1, 0, CRC_32C, cbor(rec))
    out.append(("custody_signal_refused_fragment", cbor_bundle(p, [b])))

    # 8. A custody request block (type 192) naming custodian node 2, with a CRC-16 payload.
    p = primary_block(0, CRC_16, ipn_eid(4, 1), ipn_eid(1, 1), NULL_EID, (1000, 13), 60000)
    req = canonical_block(192, 2, 0, CRC_16, cbor([2]))
    b = canonical_block(1, 1, 0, CRC_16, b"custody")
    out.append(("custody_request_block", cbor_bundle(p, [req, b])))

    return out


def main():
    lines = [
        "# BPv7 byte-exact vectors, generated by gen_bpv7.py (an independent encoder; see its docstring).",
        "# One vector per line: name, then the bundle bytes in hex.",
    ]
    for name, data in vectors():
        lines.append(f"{name} {data.hex()}")
    sys.stdout.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
