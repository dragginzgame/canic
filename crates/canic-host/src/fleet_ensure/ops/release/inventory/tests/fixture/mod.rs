//! Minimal wire fixture: retain an opaque reply, without pretending to implement Fleet roles.

use std::cell::RefCell;

#[link(wasm_import_module = "ic0")]
unsafe extern "C" {
    fn msg_arg_data_size() -> i32;
    fn msg_arg_data_copy(dst: i32, offset: i32, size: i32);
    fn msg_reply_data_append(src: i32, size: i32);
    fn msg_reply();
}

thread_local! {
    static FUNDING: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static REPLY: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(export_name = "canister_init")]
pub extern "C" fn init() {
    let size = unsafe { msg_arg_data_size() };
    let mut bytes = vec![0; size as usize];
    unsafe { msg_arg_data_copy(bytes.as_mut_ptr() as i32, 0, size) };
    REPLY.with(|reply| reply.replace(bytes));
}

#[unsafe(export_name = "canister_update replace")]
pub extern "C" fn replace() {
    init();
    unsafe { msg_reply() };
}

fn reply() {
    REPLY.with(|reply| {
        let bytes = reply.borrow();
        unsafe {
            msg_reply_data_append(bytes.as_ptr() as i32, bytes.len() as i32);
            msg_reply();
        }
    });
}

#[unsafe(export_name = "canister_query canic_coordinator_registry")]
pub extern "C" fn registry() {
    reply();
}

#[unsafe(export_name = "canister_query canic_root_status")]
pub extern "C" fn pool() {
    reply();
}

#[unsafe(export_name = "canister_update replace_funding")]
pub extern "C" fn replace_funding() {
    let size = unsafe { msg_arg_data_size() };
    let mut bytes = vec![0; size as usize];
    unsafe { msg_arg_data_copy(bytes.as_mut_ptr() as i32, 0, size) };
    FUNDING.with(|reply| reply.replace(bytes));
    unsafe { msg_reply() };
}

#[unsafe(export_name = "canister_query canic_observability")]
pub extern "C" fn funding() {
    FUNDING.with(|reply| {
        let bytes = reply.borrow();
        unsafe {
            msg_reply_data_append(bytes.as_ptr() as i32, bytes.len() as i32);
            msg_reply();
        }
    });
}
