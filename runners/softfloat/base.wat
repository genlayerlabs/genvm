(module
  (func $f32_abs (export "f32_abs") (param $a f32) (result f32)
    (f32.reinterpret_i32 (i32.and (i32.reinterpret_f32 (local.get $a)) (i32.const 0x7FFFFFFF))))

  (func $f32_neg (export "f32_neg") (param $a f32) (result f32)
    (f32.reinterpret_i32 (i32.xor (i32.reinterpret_f32 (local.get $a)) (i32.const 0x80000000))))

  (func $f32_copysign (export "f32_copysign") (param $a f32) (param $b f32) (result f32)
    (f32.reinterpret_i32
      (i32.or
        (i32.and (i32.reinterpret_f32 (local.get $a)) (i32.const 0x7FFFFFFF))
        (i32.and (i32.reinterpret_f32 (local.get $b)) (i32.const 0x80000000)))))

  (func $f64_abs (export "f64_abs") (param $a f64) (result f64)
    (f64.reinterpret_i64 (i64.and (i64.reinterpret_f64 (local.get $a)) (i64.const 0x7FFFFFFFFFFFFFFF))))

  (func $f64_neg (export "f64_neg") (param $a f64) (result f64)
    (f64.reinterpret_i64 (i64.xor (i64.reinterpret_f64 (local.get $a)) (i64.const 0x8000000000000000))))

  (func $f64_copysign (export "f64_copysign") (param $a f64) (param $b f64) (result f64)
    (f64.reinterpret_i64
      (i64.or
        (i64.and (i64.reinterpret_f64 (local.get $a)) (i64.const 0x7FFFFFFFFFFFFFFF))
        (i64.and (i64.reinterpret_f64 (local.get $b)) (i64.const 0x8000000000000000)))))

  (func $f32_i_is_nan (export "f32_i_is_nan") (param $a i32) (result i32)
    (i32.and
      (i32.eq (i32.and (local.get $a) (i32.const 0x7F800000)) (i32.const 0x7F800000))
      (i32.ne (i32.and (local.get $a) (i32.const 0x007FFFFF)) (i32.const 0))))

  (func $f32_is_nan (export "f32_is_nan") (param $a f32) (result i32)
    (call $f32_i_is_nan (i32.reinterpret_f32 (local.get $a))))

  (func $f64_i_is_nan (export "f64_i_is_nan") (param $a i64) (result i32)
    (i32.and
      (i64.eq (i64.and (local.get $a) (i64.const 0x7FF0000000000000)) (i64.const 0x7FF0000000000000))
      (i64.ne (i64.and (local.get $a) (i64.const 0x000FFFFFFFFFFFFF)) (i64.const 0))))

  (func $f64_is_nan (export "f64_is_nan") (param $a f64) (result i32)
    (call $f64_i_is_nan (i64.reinterpret_f64 (local.get $a))))

  (func $f32_i_is_zero (export "f32_i_is_zero") (param $a i32) (result i32)
    (i32.eqz (i32.and (local.get $a) (i32.const 0x7FFFFFFF))))

  (func $f32_is_zero (export "f32_is_zero") (param $a f32) (result i32)
    (call $f32_i_is_zero (i32.reinterpret_f32 (local.get $a))))

  (func $f64_i_is_zero (export "f64_i_is_zero") (param $a i64) (result i32)
    (i64.eqz (i64.and (local.get $a) (i64.const 0x7FFFFFFFFFFFFFFF))))

  (func $f64_is_zero (export "f64_is_zero") (param $a f64) (result i32)
    (call $f64_i_is_zero (i64.reinterpret_f64 (local.get $a))))

  (func $f32_i_not_zero (export "f32_i_not_zero") (param $a i32) (result i32)
    (i32.eqz (call $f32_i_is_zero (local.get $a))))

  (func $f32_not_zero (export "f32_not_zero") (param $a f32) (result i32)
    (i32.eqz (call $f32_is_zero (local.get $a))))

  (func $f64_i_not_zero (export "f64_i_not_zero") (param $a i64) (result i32)
    (i32.eqz (call $f64_i_is_zero (local.get $a))))

  (func $f64_not_zero (export "f64_not_zero") (param $a f64) (result i32)
    (i32.eqz (call $f64_is_zero (local.get $a))))
)
