const _: () = {
    #[canic_memory::__reexports::ctor::ctor(
        anonymous,
        crate_path = canic_memory::__reexports::ctor
    )]
    fn __canic_reserve_template_runtime_memory_range() {
        canic_memory::ic_memory_range!(10, 12);
    }
};

pub mod storage;
