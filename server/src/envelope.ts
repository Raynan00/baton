// Minimal protobuf reader for the outer Envelope (fields 1-8). Ciphertext is never inspected.
export interface EnvelopeHeader {
  v: number;
  msgId: Uint8Array;
  sender: Uint8Array;
  recipient: Uint8Array;
  sentAtMs: number;
  ttlS: number;
  push: number;
  collapseKey: Uint8Array;
}

export function parseEnvelope(buf: Uint8Array): EnvelopeHeader {
  const h: EnvelopeHeader = {
    v: 0, msgId: new Uint8Array(), sender: new Uint8Array(), recipient: new Uint8Array(),
    sentAtMs: 0, ttlS: 0, push: 0, collapseKey: new Uint8Array(),
  };
  let i = 0;
  const varint = (): number => {
    let r = 0, shift = 1;
    for (;;) {
      if (i >= buf.length) throw new Error("truncated");
      const b = buf[i++];
      r += (b & 0x7f) * shift;
      if (!(b & 0x80)) return r;
      shift *= 128;
    }
  };
  while (i < buf.length) {
    const tag = varint();
    const field = tag >>> 3, wire = tag & 7;
    if (wire === 0) {
      const v = varint();
      if (field === 1) h.v = v; else if (field === 5) h.sentAtMs = v;
      else if (field === 6) h.ttlS = v; else if (field === 7) h.push = v;
    } else if (wire === 2) {
      const len = varint();
      if (i + len > buf.length) throw new Error("truncated");
      const d = buf.subarray(i, i + len);
      i += len;
      if (field === 2) h.msgId = d; else if (field === 3) h.sender = d;
      else if (field === 4) h.recipient = d; else if (field === 8) h.collapseKey = d;
    } else {
      throw new Error("unsupported wire type");
    }
  }
  return h;
}
