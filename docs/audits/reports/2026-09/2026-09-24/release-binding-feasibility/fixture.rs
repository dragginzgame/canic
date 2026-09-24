//! Audit-only executable binding slot; not a production Canic entrypoint.
#![no_std]

#[used]
#[unsafe(no_mangle)]
pub static mut CANIC_BINDING_SLOT_V1: [u8; 64] = [b'?'; 64];

#[link(wasm_import_module = "ic0")]
unsafe extern "C" {
    fn msg_arg_data_size() -> i32;
    fn msg_arg_data_copy(dst: i32, offset: i32, size: i32);
    fn msg_reply_data_append(src: i32, size: i32);
    fn msg_reply();
    fn trap(src: i32, size: i32) -> !;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    fail()
}

fn fail() -> ! {
    let text = b"release binding rejected";
    unsafe { trap(text.as_ptr() as i32, text.len() as i32) }
}

fn binding() -> [u8; 64] {
    let mut bytes = [0; 64];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe {
            core::ptr::read_volatile(
                core::ptr::addr_of!(CANIC_BINDING_SLOT_V1)
                    .cast::<u8>()
                    .add(index),
            )
        };
        if !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte) {
            fail();
        }
    }
    bytes
}

fn admit() {
    let expected = binding();
    let mut argument = [0_u8; 64];
    unsafe {
        if msg_arg_data_size() != 64 {
            fail();
        }
        msg_arg_data_copy(argument.as_mut_ptr() as i32, 0, 64);
    }
    if argument != expected {
        fail();
    }
}

#[unsafe(export_name = "canister_init")]
pub extern "C" fn init() {
    admit();
}

#[unsafe(export_name = "canister_post_upgrade")]
pub extern "C" fn restore() {
    admit();
}

#[unsafe(export_name = "canister_query binding")]
pub extern "C" fn query() {
    let bytes = binding();
    unsafe {
        msg_reply_data_append(bytes.as_ptr() as i32, 64);
        msg_reply();
    }
}
