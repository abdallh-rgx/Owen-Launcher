/**
 * owen-detect — curl_easy_setopt scanner (TypeScript port of the Rust v2 scan).
 *
 * The scanner locates `curl_easy_setopt` in a stripped ARM64 shared object
 * without any symbol table, using four structural layers:
 *
 *   1. `MOV W0, #0x2b`  — CURLE_BAD_FUNCTION_ARGUMENT (43), returned when
 *      the handle argument is NULL.
 *   2. `CBZ X0, +imm19` — the NULL-handle check branching to that MOV.
 *   3. `BL`             — a call to Curl_vsetopt between the CBZ and the MOV.
 *   4. Backward walk    — from the CBZ to the function prologue.
 *
 * v2 improvements over the reference algorithm:
 *   - closest-marker backward walk (instead of earliest forward marker)
 *   - widened `stp Xt1,Xt2,[sp,#-N]!` prologue markers
 *   - predecessor sanity check (instruction before a true start is almost
 *     always ret / b / bl / b.cond / ldp-post-index / nop)
 *   - variadic `va_start` fingerprint bonus (+3): the prologue of a variadic
 *     function spills q0-q7 as STP Q pairs before the CBZ
 */

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** `mov w0, #0x2b` instruction bytes (little-endian). */
export const MOV_W0_2B_BYTES: readonly [number, number, number, number] = [0x60, 0x05, 0x80, 0x52];

/** Maximum distance between CBZ and MOV for the pair to be considered. */
export const BL_MAX_GAP = 0x200;

/** Sensible gap range between CBZ and MOV (used by the scorer). */
export const CBZ_MOV_GAP_MIN = 0x30;
export const CBZ_MOV_GAP_MAX = 0xc0;

/** Backwards search window when locating the function prologue. */
export const FUNC_BACK_WINDOW = 0x100;

/** Maximum candidates returned per scan. */
export const MAX_CANDIDATES = 8;

// ---------------------------------------------------------------------------
// ELF64-LE program headers
// ---------------------------------------------------------------------------

export interface Segment {
  /** File offset of the segment. */
  offset: number;
  /** Virtual address of the segment. */
  vaddr: number;
  /** Size of the segment data in the file. */
  filesz: number;
}

function readU16(d: Uint8Array, off: number): number {
  return d[off] | (d[off + 1] << 8);
}

function readU32(d: Uint8Array, off: number): number {
  return (d[off] | (d[off + 1] << 8) | (d[off + 2] << 16) | (d[off + 3] << 24)) >>> 0;
}

function readU64(d: Uint8Array, off: number): number {
  // ELF64 fields we care about (vaddr/offset) fit in 53 bits in practice;
  // Number can hold them exactly up to 2^53.
  let lo = readU32(d, off);
  let hi = readU32(d, off + 4);
  return hi * 0x100000000 + lo;
}

/**
 * Read the PT_LOAD segments of an ELF64 little-endian image.
 * Returns null when the buffer is not ELF64-LE.
 */
export function readElfSegments(data: Uint8Array): Segment[] | null {
  if (data.length < 64) return null;
  if (data[0] !== 0x7f || data[1] !== 0x45 || data[2] !== 0x4c || data[3] !== 0x46) return null;
  if (data[4] !== 2 || data[5] !== 1) return null; // ELF64, little-endian

  const phoff = readU64(data, 0x20);
  const phentsize = readU16(data, 0x36);
  const phnum = readU16(data, 0x38);
  if (phentsize < 56 || phnum === 0) return null;

  const segments: Segment[] = [];
  for (let i = 0; i < phnum; i++) {
    const base = phoff + i * phentsize;
    if (base + 56 > data.length) break;
    const pType = readU32(data, base);
    if (pType !== 1) continue; // PT_LOAD only
    segments.push({
      offset: readU64(data, base + 8),
      vaddr: readU64(data, base + 16),
      filesz: readU64(data, base + 32),
    });
  }
  return segments;
}

/** Convert a file offset to a virtual address using the PT_LOAD mapping. */
export function offToVa(segs: Segment[], off: number): number | null {
  for (const s of segs) {
    if (s.offset <= off && off < s.offset + s.filesz) {
      return off - s.offset + s.vaddr;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// ARM64 instruction decoding
// ---------------------------------------------------------------------------

/** Read a little-endian 32-bit instruction at `off` (0 when out of bounds). */
function wordAt(d: Uint8Array, off: number): number {
  if (off < 0 || off + 4 > d.length) return 0;
  return (d[off] | (d[off + 1] << 8) | (d[off + 2] << 16) | (d[off + 3] << 24)) >>> 0;
}

/** Sign-extend the low `bits` bits of `value`. */
function signExtend(value: number, bits: number): number {
  const sign = 1 << (bits - 1);
  return ((value ^ sign) - sign) | 0;
}

/**
 * Decode the branch target of a `CBZ X0, ...` instruction.
 * Returns null when the word is not a CBZ on X0.
 */
function cbzX0Target(word: number, pc: number): number | null {
  if ((word >>> 24) !== 0xb4) return null;
  if ((word & 0x1f) !== 0) return null;
  const imm19 = (word >>> 5) & 0x7ffff;
  const offset = signExtend(imm19, 19) << 2;
  return pc + offset;
}

/** Decode the branch target of a `BL` instruction (null when not a BL). */
function blTarget(word: number, pc: number): number | null {
  if ((word >>> 26) !== 0x25) return null; // 0b100101
  const imm26 = word & 0x03ffffff;
  const offset = signExtend(imm26, 26) << 2;
  return pc + offset;
}

/**
 * `stp x29, x30, [sp, #+N]` (signed offset, non-writeback) — used by the
 * EOSSDK/GCC compiler flavour for `stp x29,x30,[sp,#0x10]`-style prologues.
 */
function isStpFpLrSignedOffset(word: number): boolean {
  return (
    (word & 0xffc00000) >>> 0 === 0xa9000000 &&
    (word & 0x1f) === 29 &&
    ((word >>> 10) & 0x1f) === 30
  );
}

/** `sub sp, sp, #imm` */
function isSubSpSp(word: number): boolean {
  return (word & 0xff8003ff) >>> 0 === 0xd10003ff;
}

/** Any `stp Xt1, Xt2, [sp, #-N]!` (pre-index writeback). */
function isStpPreindexSp(word: number): boolean {
  return (word & 0xffc00000) >>> 0 === 0xa9800000 && ((word >>> 5) & 0x1f) === 31;
}

/** `stp qQt1, qQt2, [sp, #imm]` — 128-bit SIMD pair store. */
function isStpQPair(word: number): boolean {
  const base = (word & 0xffc00000) >>> 0;
  return base === 0xad000000 || base === 0xad800000;
}

/** `pacibsp` */
function isPacibsp(word: number): boolean {
  return word === 0xd503237f;
}

/** Branch target identification markers. */
function isBti(word: number): boolean {
  return (
    word === 0xd503241f ||
    word === 0xd503245f ||
    word === 0xd503249f ||
    word === 0xd50324df
  );
}

/** v2 prologue marker set. */
function isPrologueMarkerV2(word: number): boolean {
  return (
    isSubSpSp(word) ||
    isStpPreindexSp(word) ||
    isStpFpLrSignedOffset(word) ||
    isPacibsp(word) ||
    isBti(word)
  );
}

// Function-end markers (v2 predecessor sanity check)

/** `ret` (covers `retaa`/`retab` too via the mask). */
function isRet(word: number): boolean {
  return (word & 0xffffffe0) >>> 0 === 0xd65f03c0;
}

/** Unconditional `b imm` */
function isB(word: number): boolean {
  return (word & 0xfc000000) >>> 0 === 0x14000000;
}

/** `bl imm` */
function isBlWord(word: number): boolean {
  return (word & 0xfc000000) >>> 0 === 0x94000000;
}

/** Conditional branch `b.cond` */
function isBCond(word: number): boolean {
  return (word & 0xff000010) >>> 0 === 0x54000000;
}

/** `ldp Xt1, Xt2, [sp], #imm` post-index epilogue. */
function isLdpPostindexSp(word: number): boolean {
  return (word & 0xffc00000) >>> 0 === 0xa8c00000 && ((word >>> 5) & 0x1f) === 31;
}

/**
 * A word that can plausibly terminate the previous function or act as
 * inter-function padding.
 */
function isFunctionEnd(word: number): boolean {
  return (
    isRet(word) ||
    isB(word) ||
    isBlWord(word) ||
    isBCond(word) ||
    isLdpPostindexSp(word) ||
    word === 0xd503201f || // nop
    word === 0xd4200000 // brk
  );
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

export interface Candidate {
  /** File offset of the function prologue, when located. */
  funcStart: number | null;
  /** File offset of the `CBZ X0` instruction. */
  cbzOff: number;
  /** File offset of the `BL` (Curl_vsetopt call). */
  blOff: number | null;
  /** File offset of the BL target (Curl_vsetopt), when decodable. */
  blTarget: number | null;
  /** File offset of the `MOV W0, #0x2b`. */
  movOff: number;
  /** Distance in bytes between CBZ and MOV. */
  gap: number;
  /** Confidence score out of 10 plus the variadic bonus. */
  score: number;
  /** The `va_start` spill fingerprint. */
  variadic: boolean;
}

/**
 * v2 back-walk: scan backwards from the CBZ. The closest marker whose
 * predecessor looks like the end of the previous function wins. If no marker
 * passes the predecessor check, the closest raw marker is used.
 */
function findFunctionStartV2(data: Uint8Array, cbzOff: number): number | null {
  const lo = Math.max(0, cbzOff - FUNC_BACK_WINDOW) & ~3;
  let off = (cbzOff - 4) & ~3;
  let fallback: number | null = null;
  while (off + 4 <= cbzOff && off >= lo) {
    const word = wordAt(data, off);
    if (isPrologueMarkerV2(word)) {
      if (fallback === null) fallback = off;
      if (off >= 4) {
        const prev = wordAt(data, off - 4);
        if (isFunctionEnd(prev)) return off;
      } else {
        return off; // image start
      }
    }
    if (off < 4) break;
    off -= 4;
  }
  return fallback;
}

/**
 * Count the FP varargs spill instructions between two offsets.
 * `curl_easy_setopt` is variadic; its prologue spills q0-q7 (and typically
 * x2-x7) right after the stack setup, before the `cbz x0` NULL check.
 * Two or more STP Q-pair spills is the va_start fingerprint.
 */
function countVariadicSpill(data: Uint8Array, from: number, to: number): boolean {
  let qPairs = 0;
  let off = Math.max(0, from) & ~3;
  while (off + 4 <= to) {
    if (isStpQPair(wordAt(data, off))) qPairs++;
    off += 4;
  }
  return qPairs >= 2;
}

/** Reference scoring (0-10) plus the v2 variadic bonus (+3). */
function scoreCandidate(c: Candidate): number {
  let s = 0;
  if (c.gap >= CBZ_MOV_GAP_MIN && c.gap <= CBZ_MOV_GAP_MAX) s += 2;
  if (c.funcStart !== null) {
    s += 3;
    const dist = c.cbzOff - c.funcStart;
    if (dist >= 0x10 && dist <= 0x80) s += 2;
  }
  if (c.blTarget !== null) {
    s += 2;
    if (c.movOff > (c.blOff ?? 0) && c.movOff - (c.blOff ?? 0) < 0x40) s += 1;
  }
  if (c.variadic) s += 3;
  return s;
}

// ---------------------------------------------------------------------------
// CBZ X0 index
// ---------------------------------------------------------------------------

function buildCbzIndex(data: Uint8Array, index: Map<number, number[]>): void {
  const n = data.length;
  if (n < 4) return;
  // Scan every byte position p where data[p+3] === 0xB4 (potential CBZ X0
  // high byte), then validate the full instruction word at p.
  for (let p = 0; p + 4 <= n; p++) {
    if (data[p + 3] !== 0xb4) continue;
    const off = p;
    if (off % 4 !== 0) continue;
    if ((data[off] & 0x1f) !== 0) continue; // Rt must be X0
    const word = wordAt(data, off);
    if ((word >>> 24) !== 0xb4) continue;
    const target = cbzX0Target(word, off);
    if (target === null || target < 0) continue;
    let list = index.get(target);
    if (list === undefined) {
      list = [];
      index.set(target, list);
    }
    list.push(off);
  }
}

// ---------------------------------------------------------------------------
// Top-level scan
// ---------------------------------------------------------------------------

export interface ScanResult {
  candidates: Candidate[];
  best: Candidate | null;
  elapsedMs: number;
}

/**
 * Scan an ELF64-LE ARM64 image for `curl_easy_setopt` candidates.
 * Candidates are sorted by score (descending), then CBZ offset (ascending).
 */
export function scanImage(data: Uint8Array): ScanResult {
  const started = Date.now();
  const n = data.length;
  const out: Candidate[] = [];

  if (n >= 8) {
    // Index every CBZ X0 as { target_offset: [cbz_off, ...] }.
    const cbzIndex = new Map<number, number[]>();
    buildCbzIndex(data, cbzIndex);

    // Find every occurrence of the MOV W0,#0x2b bytes.
    const b0 = MOV_W0_2B_BYTES[0];
    const b1 = MOV_W0_2B_BYTES[1];
    const b2 = MOV_W0_2B_BYTES[2];
    const b3 = MOV_W0_2B_BYTES[3];
    let movOff = data.indexOf(b0);
    let cursor = 0;
    movOff = -1;
    while (cursor + 4 <= n) {
      // Fast skip using indexOf on the first byte.
      let p = data.indexOf(b0, cursor);
      if (p === -1 || p + 4 > n) break;
      if (data[p + 1] === b1 && data[p + 2] === b2 && data[p + 3] === b3) {
        movOff = p;
        if (movOff % 4 === 0) {
          const cbzs = cbzIndex.get(movOff);
          if (cbzs !== undefined) {
            for (const cbzOff of cbzs) {
              const gap = movOff - cbzOff;
              if (cbzOff >= movOff || gap === 0 || gap > BL_MAX_GAP) continue;

              // Layer 3: first BL strictly between CBZ and MOV.
              let blOff: number | null = null;
              let blDst: number | null = null;
              for (let j = cbzOff; j + 4 <= movOff; j += 4) {
                const b = data[j + 3];
                if (b >= 0x94 && b <= 0x97) {
                  const word = wordAt(data, j);
                  const dst = blTarget(word, j);
                  if (dst !== null) {
                    blOff = j;
                    blDst = dst;
                    break;
                  }
                }
              }
              if (blOff === null) continue;

              // Layer 4: locate the prologue (v2 backward walk).
              const funcStart = findFunctionStartV2(data, cbzOff);

              // v2: the va_start fingerprint.
              const from = funcStart ?? Math.max(0, cbzOff - 0x40);
              const variadic =
                countVariadicSpill(data, from, cbzOff) ||
                countVariadicSpill(data, cbzOff, movOff);

              const c: Candidate = {
                funcStart,
                cbzOff,
                blOff,
                blTarget: blDst,
                movOff,
                gap,
                score: 0,
                variadic,
              };
              c.score = scoreCandidate(c);
              out.push(c);
              break;
            }
          }
        }
      }
      cursor = p + 1;
    }
  }

  out.sort((a, b) => (b.score - a.score) || (a.cbzOff - b.cbzOff));

  return {
    candidates: out.slice(0, MAX_CANDIDATES),
    best: out.length > 0 ? out[0] : null,
    elapsedMs: Date.now() - started,
  };
}

/** Recommended hook address: function start when known, else the CBZ. */
export function hookOffset(c: Candidate): number {
  return c.funcStart ?? c.cbzOff;
}
