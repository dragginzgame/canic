use canic::{Error, canic_query};

#[canic_query(composite)]
fn composite_probe() -> Result<(), Error> {
    Ok(())
}

#[test]
fn canic_query_accepts_composite_marker() {
    std::hint::black_box(composite_probe as fn() -> Result<(), Error>);
}
