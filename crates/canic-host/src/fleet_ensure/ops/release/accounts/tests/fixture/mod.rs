//! Wire-only Ledger fixture rejects any argument other than the exact expected account.

use std::cell::RefCell;

#[link(wasm_import_module = "ic0")]
unsafe extern "C" {
    fn msg_arg_data_size() -> i32;
    fn msg_arg_data_copy(dst: i32, offset: i32, size: i32);
    fn msg_reply_data_append(src: i32, size: i32);
    fn msg_reply();
    fn msg_reject(src: i32, size: i32);
}

thread_local! {
    static DATA: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn argument() -> Vec<u8> {
    let size = unsafe { msg_arg_data_size() };
    let mut bytes = vec![0; size as usize];
    unsafe { msg_arg_data_copy(bytes.as_mut_ptr() as i32, 0, size) };
    bytes
}

#[unsafe(export_name = "canister_init")]
pub extern "C" fn init() {
    DATA.with(|data| data.replace(argument()));
}

#[unsafe(export_name = "canister_update replace")]
pub extern "C" fn replace() {
    init();
    unsafe { msg_reply() };
}

#[unsafe(export_name = "canister_query icrc1_balance_of")]
pub extern "C" fn balance() {
    DATA.with(|data| {
        let bytes = data.borrow();
        let length = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        if argument() != bytes[4..4 + length] {
            let error = b"wrong account";
            unsafe { msg_reject(error.as_ptr() as i32, error.len() as i32) };
            return;
        }
        let reply = &bytes[4 + length..];
        unsafe {
            msg_reply_data_append(reply.as_ptr() as i32, reply.len() as i32);
            msg_reply();
        }
    });
}
