# cleanup
cargo audit
cargo sort -w 1>/dev/null

# update last
cargo update --verbose
cargo sort-derives --check || cargo sort-derives
