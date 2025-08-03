# update last
cargo update --verbose
cargo sort-derives --check || cargo sort-derives
