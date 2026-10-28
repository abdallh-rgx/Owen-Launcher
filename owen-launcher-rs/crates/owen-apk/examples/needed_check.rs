fn main() {
    let path = std::env::args().nth(1).expect("usage: needed_check <so> [lib]");
    let lib: String = std::env::args()
        .nth(2)
        .unwrap_or_else(|| String::from("libowen.so"));
    let data = std::fs::read(&path).expect("read failed");
    let layout = owen_apk::needed::describe(&data);
    println!("layout: {}", layout.unwrap_or_else(|| String::from("n/a")));
    let outcome = owen_apk::needed::inject_needed(&data, &lib);
    match outcome {
        owen_apk::needed::InjectOutcome::Injected(new) => {
            println!("INJECTED: {} bytes (was {})", new.len(), data.len());
            let v = owen_apk::needed::describe(&new).unwrap_or_default();
            println!("verify: {v}");
        }
        owen_apk::needed::InjectOutcome::AlreadyPresent => println!("outcome: already present"),
        owen_apk::needed::InjectOutcome::NoDynamicSlot => println!("outcome: no spare DT_NULL slot"),
        owen_apk::needed::InjectOutcome::NoStringRoom => println!("outcome: no .dynstr room"),
        owen_apk::needed::InjectOutcome::NotAnElf => println!("outcome: not an elf"),
    }
}
