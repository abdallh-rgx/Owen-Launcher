//! Minimal fixed-address AArch64 assembler — a direct port of the assembler
//! in the original `Owen.c` (Dobby), emitting the same instruction forms.

use crate::Addr;

pub(crate) const NOP: u32 = 0xD503_201F;
pub(crate) const RET: u32 = 0xD65F_03C0;
pub(crate) const B_: u32 = 0x1400_0000;
pub(crate) const BL: u32 = 0x9400_0000;
pub(crate) const BR: u32 = 0xD61F_0000;
pub(crate) const BLR: u32 = 0xD63F_0000;
pub(crate) const ADRP: u32 = 0x9000_0000;
pub(crate) const ADD_X_IMM: u32 = 0x9100_0000;
pub(crate) const LDR_X: u32 = 0xF940_0000;
pub(crate) const LDR_W: u32 = 0xB940_0000;
pub(crate) const STR_X: u32 = 0xF900_0000;
pub(crate) const ORR_X: u32 = 0xAA00_0000;
pub(crate) const MOVZ: u32 = 0x4000_0000;
pub(crate) const MOVK: u32 = 0x6000_0000;
pub(crate) const SIXTY_FOUR: u32 = 0x8000_0000;
pub(crate) const MOVE_WIDE_FIXED: u32 = 0x1280_0000;
pub(crate) const LDR_LITERAL_FIXED: u32 = 0x5800_0000;

/// Scratch register used by trampolines (x17, the inter-procedural scratch
/// register — safe because the ABI guarantees it is clobbered by calls).
pub(crate) const TMP: u32 = 17;

#[inline]
fn rd(v: u32) -> u32 {
    v & 0x1F
}
#[inline]
fn rn(v: u32) -> u32 {
    (v & 0x1F) << 5
}
#[inline]
fn rt(v: u32) -> u32 {
    v & 0x1F
}
#[inline]
fn rm(v: u32) -> u32 {
    (v & 0x1F) << 16
}

pub(crate) struct Assembler {
    buffer: Vec<u8>,
    /// Trailing literal pool: (data, list of (patch offsets)).
    literals: Vec<(u64, Vec<usize>)>,
}

impl Assembler {
    pub(crate) fn new() -> Self {
        Assembler {
            buffer: Vec::with_capacity(128),
            literals: Vec::new(),
        }
    }

    pub(crate) fn pc_offset(&self) -> usize {
        self.buffer.len()
    }

    fn emit_u32(&mut self, v: u32) {
        self.buffer.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_u64(&mut self, v: u64) {
        self.buffer.extend_from_slice(&v.to_le_bytes());
    }

    fn rewrite_inst(&mut self, off: usize, v: u32) {
        self.buffer[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn load_inst(&self, off: usize) -> u32 {
        let mut b = [0u8; 4];
        b.copy_from_slice(&self.buffer[off..off + 4]);
        u32::from_le_bytes(b)
    }

    pub(crate) fn nop(&mut self) {
        self.emit_u32(NOP);
    }

    /// Emit one raw pre-encoded instruction word (used by the relocator).
    pub(crate) fn push_word(&mut self, v: u32) {
        self.emit_u32(v);
    }
    pub(crate) fn ret(&mut self) {
        self.emit_u32(RET);
    }
    pub(crate) fn br(&mut self, reg: u32) {
        self.emit_u32(BR | rn(reg));
    }
    pub(crate) fn blr(&mut self, reg: u32) {
        self.emit_u32(BLR | rn(reg));
    }

    pub(crate) fn b_imm(&mut self, imm: i64) {
        let imm26 = ((imm >> 2) as u32) & 0x03FF_FFFF;
        self.emit_u32(B_ | imm26);
    }

    pub(crate) fn adrp(&mut self, r: u32, imm: i64) {
        let immlo = (((imm >> 12) as u32) & 0x3) << 29;
        let immhi = ((((imm >> 12) as u64) >> 2) as u32 & 0x7FFFF) << 5;
        self.emit_u32(ADRP | rd(r) | immlo | immhi);
    }

    pub(crate) fn add_imm_x(&mut self, r: u32, n: u32, imm: i64) {
        let imm12 = ((imm as u32) & 0xFFF) << 10;
        self.emit_u32(ADD_X_IMM | rd(r) | rn(n) | imm12);
    }

    pub(crate) fn adrp_add(&mut self, r: u32, from: Addr, to: Addr) {
        let from_page = from & !0xFFF;
        let to_page = to & !0xFFF;
        let to_off = to & 0xFFF;
        self.adrp(r, (to_page as i64) - (from_page as i64));
        self.add_imm_x(r, r, to_off as i64);
    }

    pub(crate) fn ldr_literal_x(&mut self, r: u32, imm: i64) {
        let enc = ((((imm >> 2) as u32) & 0x7FFFF) << 5) | rt(r);
        self.emit_u32(LDR_LITERAL_FIXED | enc);
    }

    pub(crate) fn ldr_unsigned_x(&mut self, r: u32, n: u32, offset: i64) {
        let imm12 = ((offset >> 3) as u32) & 0xFFF;
        self.emit_u32(0x3900_0000 | LDR_X | (imm12 << 10) | rn(n) | rt(r));
    }

    pub(crate) fn ldr_unsigned_w(&mut self, r: u32, n: u32, offset: i64) {
        let imm12 = ((offset >> 2) as u32) & 0xFFF;
        self.emit_u32(0x3900_0000 | LDR_W | (imm12 << 10) | rn(n) | rt(r));
    }

    pub(crate) fn movz(&mut self, r: u32, imm: u64, shift: u32) {
        let shift = if shift > 0 { shift / 16 } else { 0 };
        let imm16 = ((imm as u32) & 0xFFFF) << 5;
        self.emit_u32(MOVE_WIDE_FIXED | MOVZ | SIXTY_FOUR | (shift << 21) | imm16 | rd(r));
    }

    pub(crate) fn movk(&mut self, r: u32, imm: u64, shift: u32) {
        let shift = if shift > 0 { shift / 16 } else { 0 };
        let imm16 = ((imm as u32) & 0xFFFF) << 5;
        self.emit_u32(MOVE_WIDE_FIXED | MOVK | SIXTY_FOUR | (shift << 21) | imm16 | rd(r));
    }

    pub(crate) fn mov_reg(&mut self, r: u32, n: u32) {
        self.emit_u32(0x0A00_0000 | ORR_X | rm(31) | rn(n) | rd(r));
    }

    pub(crate) fn mov_imm64(&mut self, r: u32, imm: u64) {
        let h0 = (imm & 0xFFFF) as u64;
        let h1 = (imm >> 16) & 0xFFFF;
        let h2 = (imm >> 32) & 0xFFFF;
        let h3 = (imm >> 48) & 0xFFFF;
        self.movz(r, h0, 0);
        self.movk(r, h1, 16);
        self.movk(r, h2, 32);
        self.movk(r, h3, 48);
    }

    /// Record a literal load at the current position; the 64-bit value is
    /// appended to the trailing literal pool and patched on finalize.
    pub(crate) fn ldr_label(&mut self, r: u32, data: u64) {
        let at = self.pc_offset();
        self.ldr_literal_x(r, 0);
        if let Some(l) = self.literals.iter_mut().find(|(d, _)| *d == data) {
            l.1.push(at);
        } else {
            self.literals.push((data, vec![at]));
        }
    }

    /// `ldr x17, [pc, #8]; br x17; .quad target` — far trampoline.
    pub(crate) fn literal_ldr_branch(&mut self, target: Addr) {
        self.ldr_label(TMP, target as u64);
        self.br(TMP);
    }

    pub(crate) fn finalize_bytes(&mut self) -> Vec<u8> {
        // Append literals and patch the loads.
        let mut pool_at = self.buffer.len();
        let literals = std::mem::take(&mut self.literals);
        let mut patches: Vec<(usize, usize)> = Vec::new(); // (inst offset, literal offset)
        for (data, ats) in literals.iter() {
            let literal_offset = pool_at;
            self.emit_u64(*data);
            for at in ats {
                patches.push((*at, literal_offset));
            }
            pool_at = self.buffer.len();
        }
        for (inst_off, lit_off) in patches {
            let diff = lit_off as i64 - inst_off as i64;
            let inst = self.load_inst(inst_off);
            let imm19 = ((diff >> 2) as u32) & 0x7FFFF;
            let new_inst = (inst & !(0x7FFFF << 5)) | (imm19 << 5);
            self.rewrite_inst(inst_off, new_inst);
        }
        std::mem::take(&mut self.buffer)
    }
}
