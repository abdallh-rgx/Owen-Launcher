// Quick DT_NEEDED injection test against the real 13.40 libUE4.so (309MB).
use owen_patcher::needed::inject_needed;

fn main() {
    let src = "/home/z/owen-test/a1340/lib/arm64-v8a/libUE4.so";
    let data = std::fs::read(src).expect("read libUE4.so");
    println!("loaded {} ({} MB)", src, data.len() / 1_048_576);

    let t0 = std::time::Instant::now();
    let (patched, injected) = inject_needed(&data, "libOwen.so").expect("inject");
    println!(
        "injected={} in {:?} ({} -> {} bytes)",
        injected,
        t0.elapsed(),
        data.len(),
        patched.len()
    );

    // Write the patched copy and verify with external tools afterwards.
    std::fs::write("/tmp/libUE4-patched.so", &patched).unwrap();

    // Also verify our own parse sees it.
    let elf = owen_core::elf::ElfImage::parse(&patched).unwrap();
    let needed = elf.needed_libraries_from(&patched);
    println!("DT_NEEDED entries: {needed:?}");
    assert!(needed.iter().any(|n| n == "libOwen.so"), "libOwen.so missing!");
    println!("VERIFIED: libOwen.so is in the DT_NEEDED list");
}
