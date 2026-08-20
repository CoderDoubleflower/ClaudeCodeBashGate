#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    let _ = bashgate::parse(input);
});
