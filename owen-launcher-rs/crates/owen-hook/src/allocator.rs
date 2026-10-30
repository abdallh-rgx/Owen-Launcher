//! Page allocation for trampolines and relocated code.
//!
//! Mirrors the original allocator: small (<= page) blocks are carved out of
//! page-sized pools. Code pages are mapped RW during writes and flipped back
//! to RX afterwards (W^X friendly).

use crate::Addr;
use std::sync::Mutex;

const MAX_POOLS: usize = 64;

#[repr(i32)]
pub(crate) enum Prot {
    NoAccess = 0,
    Read = 1,
    Write = 2,
    Exec = 4,
}

extern "C" {
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    fn mprotect(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
    ) -> i32;
    fn sysconf(name: i32) -> i64;
}

// from bionic: MAP_PRIVATE=0x02, MAP_ANONYMOUS=0x20, MAP_FIXED=0x10
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: usize = usize::MAX;
// _SC_PAGESIZE on Android/bionic is 30 (Linux generic).
const SC_PAGESIZE: i32 = 30;

pub(crate) fn page_size() -> usize {
    // SAFETY: sysconf(_SC_PAGESIZE) is a pure query with no preconditions.
    let v = unsafe { sysconf(SC_PAGESIZE) };
    if v <= 0 {
        4096
    } else {
        v as usize
    }
}

/// A bump allocator over one page.
struct Pool {
    base: Addr,
    cursor: usize,
    capacity: usize,
}

impl Pool {
    fn alloc(&mut self, size: usize, align: usize) -> Option<Addr> {
        let align = align.max(1);
        let cur = self.base + self.cursor;
        let aligned = (cur + align - 1) & !(align - 1);
        let start = aligned - self.base;
        if start + size > self.capacity {
            return None;
        }
        self.cursor = start + size;
        Some(aligned)
    }
}

struct Pools {
    code: Vec<Pool>,
    data: Vec<Pool>,
}

static POOLS: Mutex<Option<Pools>> = Mutex::new(None);

/// SAFETY (caller): returned memory is freshly mapped anonymous memory.
unsafe fn map_page(prot: i32) -> Option<Addr> {
    let p = page_size();
    // SAFETY: mmap with MAP_ANONYMOUS and a null hint is always valid.
    let ptr = unsafe {
        mmap(
            core::ptr::null_mut(),
            p,
            prot,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if ptr as usize == MAP_FAILED {
        return None;
    }
    Some(ptr as Addr)
}

pub(crate) fn alloc_code(size: usize) -> Option<Addr> {
    alloc_from(size, true)
}

pub(crate) fn alloc_data(size: usize) -> Option<Addr> {
    alloc_from(size, false)
}

/// The reach of a direct AArch64 `b` instruction (26-bit signed word offset).
pub(crate) const DIRECT_B_REACH: usize = 120 * 1024 * 1024; // ±120 MiB (margin)

/// Allocate an executable block **near** `hint` (within `DIRECT_B_REACH`).
///
/// The BTI-safe jump-back at the end of a relocated block is a **direct**
/// `b target+N` — direct branches are never BTI-gated, but their range is
/// ±128 MiB, so the block must live close to the hooked function. Linux
/// honors a non-null `mmap` hint whenever the page is free, so we probe a
/// handful of candidate pages around the target module before falling back
/// to the normal allocator.
pub(crate) fn alloc_code_near(size: usize, hint: Addr) -> Option<Addr> {
    if size == 0 || size > page_size() {
        return None;
    }
    let ps = page_size();
    let stride = 2 * 1024 * 1024usize; // one probe every 2 MiB
    for k in 0..24u32 {
        let delta = (k as usize / 2 + 1) * stride;
        let candidate = if k % 2 == 0 {
            hint.saturating_add(delta)
        } else {
            hint.saturating_sub(delta)
        } & !(ps - 1);
        // SAFETY: mmap with MAP_ANONYMOUS, a page-aligned hint and no
        // MAP_FIXED either returns usable fresh memory or fails — it can
        // never clobber an existing mapping.
        let ptr = unsafe {
            mmap(
                candidate as *mut _,
                ps,
                Prot::Read as i32 | Prot::Exec as i32,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if ptr as usize == MAP_FAILED {
            continue;
        }
        let got = ptr as Addr;
        // The kernel may ignore the hint — only keep the page when it is
        // actually within direct-branch reach of the target.
        if got.abs_diff(hint) <= DIRECT_B_REACH && got % ps == 0 {
            let mut guard = POOLS.lock().ok()?;
            let pools = guard.get_or_insert_with(|| Pools {
                code: Vec::new(),
                data: Vec::new(),
            });
            let mut pool = Pool {
                base: got,
                cursor: 0,
                capacity: ps,
            };
            let a = pool.alloc(size, 8)?;
            pools.code.push(pool);
            return Some(a);
        }
        // Wrong neighborhood — give the page back and try the next probe.
        // SAFETY: exactly the page mmap just returned, full length.
        unsafe { munmap(ptr, ps) };
    }
    None
}

fn alloc_from(size: usize, code: bool) -> Option<Addr> {
    if size == 0 || size > page_size() {
        return None;
    }
    let mut guard = POOLS.lock().ok()?;
    let pools = guard.get_or_insert_with(|| Pools {
        code: Vec::new(),
        data: Vec::new(),
    });
    let list = if code { &mut pools.code } else { &mut pools.data };
    for pool in list.iter_mut() {
        if let Some(a) = pool.alloc(size, 8) {
            return Some(a);
        }
    }
    if list.len() >= MAX_POOLS {
        return None;
    }
    // SAFETY: fresh anonymous mapping with the right protection.
    let base = unsafe { map_page(if code { Prot::Read as i32 | Prot::Exec as i32 } else { Prot::Read as i32 | Prot::Write as i32 }) }?;
    let capacity = page_size();
    let mut pool = Pool {
        base,
        cursor: 0,
        capacity,
    };
    let a = pool.alloc(size, 8)?;
    list.push(pool);
    Some(a)
}

/// Write `bytes` to `addr` in a page that may currently be read+execute.
///
/// SAFETY (caller): `addr..addr+len` must point to a mapped page that this
/// process owns (allocator pool or a hook target). The function restores the
/// original protection afterwards.
pub(crate) unsafe fn write_code(addr: Addr, bytes: &[u8]) -> bool {
    let ps = page_size();
    let page = addr & !(ps - 1);
    let end_page = (addr + bytes.len()) & !(ps - 1);
    // SAFETY: mprotect on a page-aligned range of our own memory.
    unsafe {
        if mprotect(page as *mut _, ps, Prot::Read as i32 | Prot::Write as i32 | Prot::Exec as i32) != 0 {
            return false;
        }
        if end_page != page
            && mprotect(end_page as *mut _, ps, Prot::Read as i32 | Prot::Write as i32 | Prot::Exec as i32) != 0
        {
            return false;
        }
        let dst = addr as *mut u8;
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        if mprotect(page as *mut _, ps, Prot::Read as i32 | Prot::Exec as i32) != 0 {
            return false;
        }
        if end_page != page && mprotect(end_page as *mut _, ps, Prot::Read as i32 | Prot::Exec as i32) != 0 {
            return false;
        }
    }
    flush_instruction_cache(addr, bytes.len());
    true
}

/// Instruction + data cache maintenance (equivalent to `__clear_cache`).
///
/// SAFETY (caller): the range must be mapped readable.
pub(crate) unsafe fn flush_instruction_cache(start: Addr, len: usize) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut ctr_el0: u64;
        core::arch::asm!("mrs {0}, ctr_el0", out(reg) ctr_el0);
        let min_line = 4usize;
        let dcache = ((ctr_el0 >> 16) & 0xF) as usize;
        let icache = (ctr_el0 & 0xF) as usize;
        let dline = min_line << dcache;
        let iline = min_line << icache;
        let dmask = !(dline - 1);
        let imask = !(iline - 1);
        let end = start + len;
        if ((ctr_el0 >> 28) & 0x1) == 0 {
            // data cache clean to point of unification
            let mut a = start & dmask;
            while a < end {
                core::arch::asm!("dc cvau, {0}", in(reg) a);
                a += dline;
            }
        }
        core::arch::asm!("dsb ish");
        if ((ctr_el0 >> 29) & 0x1) == 0 {
            let mut a = start & imask;
            while a < end {
                core::arch::asm!("ic ivau, {0}", in(reg) a);
                a += iline;
            }
            core::arch::asm!("dsb ish");
        }
        core::arch::asm!("isb sy");
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        let _ = (start, len);
    }
}
