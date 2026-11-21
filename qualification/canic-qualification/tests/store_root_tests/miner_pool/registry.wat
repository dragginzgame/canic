;; Registry reply fixture: Ok { subnet_id = Some(principal) }.
(module
  (import "ic0" "msg_reply_data_append" (func $append (param i32 i32)))
  (import "ic0" "msg_reply" (func $reply))
  (import "ic0" "msg_arg_data_size" (func $size (result i32)))
  (import "ic0" "msg_arg_data_copy" (func $copy (param i32 i32 i32)))
  (memory 1)
  (global $foreign_size (mut i32) (i32.const 0))
  ;; Install with the exact Registry request for the candidate on the NNS subnet.
  (func (export "canister_init")
    (global.set $foreign_size (call $size))
    (if (i32.gt_u (global.get $foreign_size) (i32.const 256)) (then unreachable))
    (call $copy (i32.const 256) (i32.const 0) (global.get $foreign_size)))
  (data (i32.const 0) "\44\49\44\4c\03\6b\02\bc\8a\01\01\c5\fe\d2\01\71\6c\01\bd\86\9d\8b\04\02\6e\68\01\00\00\01\01\01\2a")
  (func (export "canister_query get_subnet_for_canister")
    (local $i i32)
    (block $local_subnet
      (br_if $local_subnet (i32.ne (call $size) (global.get $foreign_size)))
      (call $copy (i32.const 512) (i32.const 0) (call $size))
      (loop $compare
        (br_if $local_subnet
          (i32.ne (i32.load8_u offset=256 (local.get $i))
                  (i32.load8_u offset=512 (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br_if $compare (i32.lt_u (local.get $i) (call $size))))
      ;; Query writes are discarded. Only this reply names the second subnet.
      (i32.store8 (i32.const 32) (i32.const 43)))
    (call $append (i32.const 0) (i32.const 33))
    (call $reply)))
