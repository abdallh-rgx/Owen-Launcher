//! AArch64 inline hooking engine.
//!
//! The only module in the whole Owen stack that touches raw memory.
//! Everything is kept in one place with explicit invariants:
//!
//! * [`install_inline_hook`] patches the first 16 bytes of a function with
//!   an absolute jump (`LDR X17, #8; BR X17; .quad target`) and builds a
//!   trampoline that executes the displaced instructions and jumps back.
//! * Displaced PC-relative instructions (ADRP / ADR / B / BL / B.cond /
//!   CBZ / TBZ / literal LDR) are relocated into the trampoline.
//! * Pages are made writable with `mprotect`, restored afterwards, and
//!   the instruction cache is flushed with `__clear_cache`.

#![allow(clippy::missing_safety_doc)]

use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct InlineHook {
    /// Address of the trampoline (call this to invoke the original).
    pub trampoline: usize,
    /// The original function address.
    pub target: usize,
    /// The original displaced bytes (for removal).
    original: [u8; DISPLACED],
}

static REGISTRY: Mutex<Option<BTreeMap<usize, InlineHook>>> = Mutex::new(None);

// -------------------------------------------------------------------------
// AArch64 instruction classification
// -------------------------------------------------------------------------

fn is_adrp(w: u32) -> bool {
    (w & 0x9F000000) == 0x90000000
}
fn is_adr(w: u32) -> bool {
    (w & 0x9F000000) == 0x10000000
}
fn is_b(w: u32) -> bool {
    (w & 0xFC000000) == 0x14000000
}
fn is_bl(w: u32) -> bool {
    (w & 0xFC000000) == 0x94000000
}
fn is_bcond(w: u32) -> bool {
    (w & 0xFF000010) == 0x54000000
}
fn is_cbz_cbnz(w: u32) -> bool {
    (w & 0x7E000000) == 0x34000000
}
fn is_tbz_tbnz(w: u32) -> bool {
    (w & 0x7E000000) == 0x36000000
}
fn is_ldr_literal(w: u32) -> bool {
    matches!(
        (w & 0xBF000000),
        0x18000000 | 0x1C000000 | 0x58000000 | 0x5C000000 | 0x98000000 | 0x9C000000 | 0xD8000000 | 0xDC000000
    )
}

fn sign_extend(v: u32, bits: u32) -> i64 {
    let sign = 1i64 << (bits - 1);
    ((v as i64) ^ sign) - sign
}

// -------------------------------------------------------------------------
// Page protection helper
// -------------------------------------------------------------------------

fn page_floor(addr: usize) -> usize {
    addr & !(0xFFF)
}

fn with_writable<T>(addr: usize, len: usize, f: impl FnOnce() -> T) -> T {
    let start = page_floor(addr);
    let end = page_floor(addr + len) + 0x1000;
    let size = end - start;
    unsafe {
        // .text is R+X; make it writable, patch, restore R+X.
        libc::mprotect(
            start as *const libc::c_void,
            size,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
        );
        let out = f();
        libc::mprotect(
            start as *const libc::c_void,
            size,
            libc::PROT_READ | libc::PROT_EXEC,
        );
        libc::__clear_cache(start as *const libc::c_void, end as *const libc::c_void);
        out
    }
}

// -------------------------------------------------------------------------
// Trampoline construction
// -------------------------------------------------------------------------

const DISPLACED: usize = 16; // bytes overwritten at the target entry

/// LDR X17, [PC+8] ; BR X17 ; .quad addr  (16-byte absolute jump)
fn absolute_jump(addr: u64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&0x58000051u32.to_le_bytes()); // LDR X17, [PC+8]
    out[4..8].copy_from_slice(&0xD61F0220u32.to_le_bytes()); // BR X17
    out[8..16].copy_from_slice(&addr.to_le_bytes());
    out
}

enum Reloc {
    Copy(u32),
    /// Absolute destination address of a branch.
    Branch { word: u32, dest: i64 },
    /// Absolute target page of an ADRP.
    AdrpPage { word: u32, dest_page: i64 },
    /// Absolute destination of an ADR.
    AdrDest { word: u32, dest: i64 },
    /// LDR literal: remember the original literal address.
    LdrLiteral { word: u32, lit_addr: i64 },
}

fn build_trampoline(target: usize) -> Result<usize, String> {
    let orig: [u32; 4] = unsafe {
        let p = target as *const u32;
        [p.read(), p.add(1).read(), p.add(2).read(), p.add(3).read()]
    };

    let mut relocs: Vec<Reloc> = Vec::new();
    for (i, &w) in orig.iter().enumerate() {
        let pc = target as i64 + (i * 4) as i64;
        if is_adrp(w) {
            let immhi = ((w >> 5) & 0x7FFFF) as i64;
            let immlo = ((w >> 29) & 0x3) as i64;
            let imm = sign_extend(((immhi << 2) | immlo) as u32, 21) << 12;
            relocs.push(Reloc::AdrpPage {
                word: w,
                dest_page: (pc & !0xFFF) + imm,
            });
        } else if is_adr(w) {
            let immhi = ((w >> 5) & 0x7FFFF) as i64;
            let immlo = ((w >> 29) & 0x3) as i64;
            let imm = sign_extend(((immhi << 2) | immlo) as u32, 21);
            relocs.push(Reloc::AdrDest { word: w, dest: pc + imm });
        } else if is_b(w) || is_bl(w) {
            let imm = sign_extend(w & 0x03FF_FFFF, 26) << 2;
            relocs.push(Reloc::Branch { word: w, dest: pc + imm });
        } else if is_bcond(w) {
            let imm = sign_extend((w >> 5) & 0x7FFFF, 19) << 2;
            relocs.push(Reloc::Branch { word: w, dest: pc + imm });
        } else if is_cbz_cbnz(w) {
            let imm = sign_extend((w >> 5) & 0x7FFFF, 19) << 2;
            relocs.push(Reloc::Branch { word: w, dest: pc + imm });
        } else if is_tbz_tbnz(w) {
            let imm = sign_extend((w >> 5) & 0x3FFF, 14) << 2;
            relocs.push(Reloc::Branch { word: w, dest: pc + imm });
        } else if is_ldr_literal(w) {
            let imm = sign_extend((w >> 5) & 0x7FFFF, 19) << 2;
            relocs.push(Reloc::LdrLiteral { word: w, lit_addr: pc + imm });
        } else {
            relocs.push(Reloc::Copy(w));
        }
    }

    // Layout:
    //   [0 .. code_len)                      relocated instructions
    //   [code_len .. code_len+8)             LDR X17,[PC+8]; BR X17
    //   [code_len+8 .. code_len+16)          .quad target+16   (back jump)
    //   [code_len+16 .. code_len+16+lit_len) copied literals
    let mut code: Vec<u8> = Vec::with_capacity(128);
    let mut literal_words: Vec<u64> = Vec::new();
    // (index into code of the LDR instruction, original literal address)
    let mut fixups: Vec<(usize, u32, u64)> = Vec::new();

    for r in &relocs {
        let inst_at = code.len();
        let new_pc = inst_at as i64; // trampoline-relative PC
        let word: u32 = match r {
            Reloc::Copy(w) => *w,
            Reloc::Branch { word, dest } => {
                let delta = *dest - new_pc;
                let mut w = *word;
                if is_b(w) || is_bl(w) {
                    w = (w & !0x03FF_FFFF) | (((delta >> 2) as u32) & 0x03FF_FFFF);
                } else if is_bcond(w) {
                    w = (w & !(0x7FFFF << 5)) | ((((delta >> 2) as u32) & 0x7FFFF) << 5);
                } else if is_cbz_cbnz(w) {
                    w = (w & !(0x7FFFF << 5)) | ((((delta >> 2) as u32) & 0x7FFFF) << 5);
                } else {
                    w = (w & !(0x3FFF << 5)) | ((((delta >> 2) as u32) & 0x3FFF) << 5);
                }
                w
            }
            Reloc::AdrpPage { word, dest_page } => {
                let delta = *dest_page - (new_pc & !0xFFF);
                let imm = delta >> 12;
                let immlo = (imm as u32) & 0x3;
                let immhi = ((imm as u64) >> 2) as u32 & 0x7FFFF;
                let mut w = *word;
                w = (w & !(0x3 << 29)) | (immlo << 29);
                w = (w & !(0x7FFFF << 5)) | (immhi << 5);
                w
            }
            Reloc::AdrDest { word, dest } => {
                let delta = *dest - new_pc;
                let immlo = (delta as u32) & 0x3;
                let immhi = ((delta as u64) >> 2) as u32 & 0x7FFFF;
                let mut w = *word;
                w = (w & !(0x3 << 29)) | (immlo << 29);
                w = (w & !(0x7FFFF << 5)) | (immhi << 5);
                w
            }
            Reloc::LdrLiteral { word, lit_addr } => {
                fixups.push((inst_at, *word, *lit_addr as u64));
                *word
            }
        };
        code.extend_from_slice(&word.to_le_bytes());
    }

    let code_len = code.len();
    // Back jump: LDR X17,[PC+8]; BR X17 (literal placed right after).
    code.extend_from_slice(&0x58000051u32.to_le_bytes());
    code.extend_from_slice(&0xD61F0220u32.to_le_bytes());
    let back_jump_literal_pos = code_len + 8;

    // Copy literals; each 8 bytes, aligned after the back-jump literal.
    let mut lit_abs = back_jump_literal_pos + 8;
    for (inst_at, word, lit_addr) in &fixups {
        let lit: u64 = unsafe { std::ptr::read_unaligned(*lit_addr as *const u64) };
        literal_words.push(lit);
        // patch the LDR imm19 to point at lit_abs
        let delta = (lit_abs as i64) - (*inst_at as i64);
        let imm19 = ((delta >> 2) as u32) & 0x7FFFF;
        let w = (*word & !(0x7FFFF << 5)) | (imm19 << 5);
        code[*inst_at..*inst_at + 4].copy_from_slice(&w.to_le_bytes());
        lit_abs += 8;
    }

    let total = lit_abs.next_multiple_of(16);
    let tram = unsafe {
        let p = libc::mmap(
            std::ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if p == libc::MAP_FAILED {
            return Err("mmap trampoline failed".into());
        }
        p as usize
    };

    unsafe {
        let dst = tram as *mut u8;
        std::ptr::copy_nonoverlapping(code.as_ptr(), dst, code.len());
        std::ptr::write_unaligned(
            dst.add(back_jump_literal_pos) as *mut u64,
            (target + DISPLACED) as u64,
        );
        let mut off = back_jump_literal_pos + 8;
        for lit in &literal_words {
            std::ptr::write_unaligned(dst.add(off) as *mut u64, *lit);
            off += 8;
        }
        libc::__clear_cache(tram as *const libc::c_void, (tram + total) as *const libc::c_void);
    }

    Ok(tram)
}

/// Install an inline hook. The returned `trampoline` invokes the original
/// function.
pub fn install_inline_hook(target: usize, hook_fn: usize) -> Result<InlineHook, String> {
    if target == 0 || hook_fn == 0 {
        return Err("null address".into());
    }
    let mut reg = REGISTRY.lock().unwrap();
    if let Some(existing) = reg.as_ref().and_then(|r| r.get(&target)) {
        return Ok(existing.clone());
    }

    let trampoline = build_trampoline(target)?;
    // Save the pristine displaced bytes before patching.
    let original: [u8; DISPLACED] =
        unsafe { std::ptr::read_unaligned(target as *const [u8; DISPLACED]) };
    let jump = absolute_jump(hook_fn as u64);
    with_writable(target, DISPLACED, || unsafe {
        std::ptr::copy_nonoverlapping(jump.as_ptr(), target as *mut u8, DISPLACED);
    });

    let hook = InlineHook {
        trampoline,
        target,
        original,
    };
    reg.get_or_insert_with(BTreeMap::new)
        .insert(target, hook.clone());
    Ok(hook)
}

/// Remove an inline hook by restoring the saved original bytes.
pub fn remove_inline_hook(target: usize) -> Result<(), String> {
    let reg = REGISTRY.lock().unwrap();
    let Some(hook) = reg.as_ref().and_then(|r| r.get(&target)) else {
        return Ok(());
    };
    let original = hook.original;
    drop(reg);
    with_writable(target, DISPLACED, || unsafe {
        std::ptr::copy_nonoverlapping(original.as_ptr(), target as *mut u8, DISPLACED);
    });
    Ok(())
}
