use icu::prelude::*;

pub static TEST: &str = "test";

//
// ICU
//

icu_start_root!(TEST);

#[update]
async fn init_async() {
    ::icu::log!(::icu::Log::Warn, "hello from init_async!!");
}

#[update]
async fn test_perf() {
    perf_start!();

    // Simulate some work before 'a'
    let mut acc: i32 = 0;
    for i in 0..500 {
        acc += i;
    }
    perf!("a");

    // More work before 'b'
    for i in 0..1000 {
        acc = acc.wrapping_add(i * 2);
    }
    perf!("b");

    // More work before 'c'
    for i in 0..2000 {
        acc = acc.wrapping_add(i * 3);
    }
    perf!("c");

    // Prevent optimization
    icu::ic::println!("acc: {}", acc);
}

export_candid!();
