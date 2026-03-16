(module
  ;; ===== Imports: scalar softfloat functions =====

  ;; f32 arithmetic
  (import "softfloat" "f32_add" (func $f32_add (param f32 f32) (result f32)))
  (import "softfloat" "f32_sub" (func $f32_sub (param f32 f32) (result f32)))
  (import "softfloat" "f32_mul" (func $f32_mul (param f32 f32) (result f32)))
  (import "softfloat" "f32_div" (func $f32_div (param f32 f32) (result f32)))
  (import "softfloat" "f32_sqrt" (func $f32_sqrt (param f32) (result f32)))

  ;; f32 min/max
  (import "softfloat" "f32_min" (func $f32_min (param f32 f32) (result f32)))
  (import "softfloat" "f32_max" (func $f32_max (param f32 f32) (result f32)))

  ;; f32 comparison
  (import "softfloat" "f32_eq" (func $f32_eq (param f32 f32) (result i32)))
  (import "softfloat" "f32_lt_quiet" (func $f32_lt_quiet (param f32 f32) (result i32)))
  (import "softfloat" "f32_le_quiet" (func $f32_le_quiet (param f32 f32) (result i32)))

  ;; f32 rounding: (value, rounding_mode, exact_flag) -> value
  (import "softfloat" "f32_roundToInt" (func $f32_roundToInt (param f32 i32 i32) (result f32)))

  ;; f32 conversion
  (import "softfloat" "i32_to_f32" (func $i32_to_f32 (param i32) (result f32)))
  (import "softfloat" "ui32_to_f32" (func $ui32_to_f32 (param i32) (result f32)))
  (import "softfloat" "f64_to_f32" (func $f64_to_f32 (param f64) (result f32)))

  ;; f32 saturating truncation
  (import "softfloat" "f32_to_i32_sat" (func $f32_to_i32_sat (param f32) (result i32)))
  (import "softfloat" "f32_to_ui32_sat" (func $f32_to_ui32_sat (param f32) (result i32)))

  ;; f64 arithmetic
  (import "softfloat" "f64_add" (func $f64_add (param f64 f64) (result f64)))
  (import "softfloat" "f64_sub" (func $f64_sub (param f64 f64) (result f64)))
  (import "softfloat" "f64_mul" (func $f64_mul (param f64 f64) (result f64)))
  (import "softfloat" "f64_div" (func $f64_div (param f64 f64) (result f64)))
  (import "softfloat" "f64_sqrt" (func $f64_sqrt (param f64) (result f64)))

  ;; f64 min/max
  (import "softfloat" "f64_min" (func $f64_min (param f64 f64) (result f64)))
  (import "softfloat" "f64_max" (func $f64_max (param f64 f64) (result f64)))

  ;; f64 comparison
  (import "softfloat" "f64_eq" (func $f64_eq (param f64 f64) (result i32)))
  (import "softfloat" "f64_lt_quiet" (func $f64_lt_quiet (param f64 f64) (result i32)))
  (import "softfloat" "f64_le_quiet" (func $f64_le_quiet (param f64 f64) (result i32)))

  ;; f64 rounding
  (import "softfloat" "f64_roundToInt" (func $f64_roundToInt (param f64 i32 i32) (result f64)))

  ;; f64 conversion
  (import "softfloat" "i32_to_f64" (func $i32_to_f64 (param i32) (result f64)))
  (import "softfloat" "ui32_to_f64" (func $ui32_to_f64 (param i32) (result f64)))
  (import "softfloat" "f32_to_f64" (func $f32_to_f64 (param f32) (result f64)))

  ;; f64 saturating truncation
  (import "softfloat" "f64_to_i32_sat" (func $f64_to_i32_sat (param f64) (result i32)))
  (import "softfloat" "f64_to_ui32_sat" (func $f64_to_ui32_sat (param f64) (result i32)))

  ;; =====================================================================
  ;; f32x4 arithmetic
  ;; =====================================================================

  (func $f32x4_add (export "f32x4_add") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_add (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_add (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_add (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_add (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  (func $f32x4_sub (export "f32x4_sub") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_sub (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_sub (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_sub (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_sub (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  (func $f32x4_mul (export "f32x4_mul") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_mul (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_mul (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_mul (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_mul (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  (func $f32x4_div (export "f32x4_div") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_div (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_div (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_div (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_div (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  (func $f32x4_sqrt (export "f32x4_sqrt") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_sqrt (f32x4.extract_lane 0 (local.get $a))))
          (call $f32_sqrt (f32x4.extract_lane 1 (local.get $a))))
        (call $f32_sqrt (f32x4.extract_lane 2 (local.get $a))))
      (call $f32_sqrt (f32x4.extract_lane 3 (local.get $a)))))

  ;; neg/abs are bitwise - no need to call scalar softfloat
  (func $f32x4_neg (export "f32x4_neg") (param $a v128) (result v128)
    (v128.xor (local.get $a) (v128.const i32x4 0x80000000 0x80000000 0x80000000 0x80000000)))

  (func $f32x4_abs (export "f32x4_abs") (param $a v128) (result v128)
    (v128.and (local.get $a) (v128.const i32x4 0x7FFFFFFF 0x7FFFFFFF 0x7FFFFFFF 0x7FFFFFFF)))

  ;; =====================================================================
  ;; f32x4 min/max
  ;; =====================================================================

  (func $f32x4_min (export "f32x4_min") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_min (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_min (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_min (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_min (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  (func $f32x4_max (export "f32x4_max") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_max (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))))
          (call $f32_max (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))))
        (call $f32_max (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))))
      (call $f32_max (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b)))))

  ;; pmin(a, b) = b < a ? b : a  (C-style min, propagates NaN from a)
  (func $f32x4_pmin (export "f32x4_pmin") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (select (f32x4.extract_lane 0 (local.get $b)) (f32x4.extract_lane 0 (local.get $a))
              (call $f32_lt_quiet (f32x4.extract_lane 0 (local.get $b)) (f32x4.extract_lane 0 (local.get $a)))))
          (select (f32x4.extract_lane 1 (local.get $b)) (f32x4.extract_lane 1 (local.get $a))
            (call $f32_lt_quiet (f32x4.extract_lane 1 (local.get $b)) (f32x4.extract_lane 1 (local.get $a)))))
        (select (f32x4.extract_lane 2 (local.get $b)) (f32x4.extract_lane 2 (local.get $a))
          (call $f32_lt_quiet (f32x4.extract_lane 2 (local.get $b)) (f32x4.extract_lane 2 (local.get $a)))))
      (select (f32x4.extract_lane 3 (local.get $b)) (f32x4.extract_lane 3 (local.get $a))
        (call $f32_lt_quiet (f32x4.extract_lane 3 (local.get $b)) (f32x4.extract_lane 3 (local.get $a))))))

  ;; pmax(a, b) = a < b ? b : a  (C-style max, propagates NaN from a)
  (func $f32x4_pmax (export "f32x4_pmax") (param $a v128) (param $b v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (select (f32x4.extract_lane 0 (local.get $b)) (f32x4.extract_lane 0 (local.get $a))
              (call $f32_lt_quiet (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b)))))
          (select (f32x4.extract_lane 1 (local.get $b)) (f32x4.extract_lane 1 (local.get $a))
            (call $f32_lt_quiet (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b)))))
        (select (f32x4.extract_lane 2 (local.get $b)) (f32x4.extract_lane 2 (local.get $a))
          (call $f32_lt_quiet (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b)))))
      (select (f32x4.extract_lane 3 (local.get $b)) (f32x4.extract_lane 3 (local.get $a))
        (call $f32_lt_quiet (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b))))))

  ;; =====================================================================
  ;; f32x4 rounding
  ;; rounding modes: 0=near_even, 1=minMag(trunc), 2=min(floor), 3=max(ceil)
  ;; =====================================================================

  (func $f32x4_ceil (export "f32x4_ceil") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_roundToInt (f32x4.extract_lane 0 (local.get $a)) (i32.const 3) (i32.const 0)))
          (call $f32_roundToInt (f32x4.extract_lane 1 (local.get $a)) (i32.const 3) (i32.const 0)))
        (call $f32_roundToInt (f32x4.extract_lane 2 (local.get $a)) (i32.const 3) (i32.const 0)))
      (call $f32_roundToInt (f32x4.extract_lane 3 (local.get $a)) (i32.const 3) (i32.const 0))))

  (func $f32x4_floor (export "f32x4_floor") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_roundToInt (f32x4.extract_lane 0 (local.get $a)) (i32.const 2) (i32.const 0)))
          (call $f32_roundToInt (f32x4.extract_lane 1 (local.get $a)) (i32.const 2) (i32.const 0)))
        (call $f32_roundToInt (f32x4.extract_lane 2 (local.get $a)) (i32.const 2) (i32.const 0)))
      (call $f32_roundToInt (f32x4.extract_lane 3 (local.get $a)) (i32.const 2) (i32.const 0))))

  (func $f32x4_trunc (export "f32x4_trunc") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_roundToInt (f32x4.extract_lane 0 (local.get $a)) (i32.const 1) (i32.const 0)))
          (call $f32_roundToInt (f32x4.extract_lane 1 (local.get $a)) (i32.const 1) (i32.const 0)))
        (call $f32_roundToInt (f32x4.extract_lane 2 (local.get $a)) (i32.const 1) (i32.const 0)))
      (call $f32_roundToInt (f32x4.extract_lane 3 (local.get $a)) (i32.const 1) (i32.const 0))))

  (func $f32x4_nearest (export "f32x4_nearest") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $f32_roundToInt (f32x4.extract_lane 0 (local.get $a)) (i32.const 0) (i32.const 0)))
          (call $f32_roundToInt (f32x4.extract_lane 1 (local.get $a)) (i32.const 0) (i32.const 0)))
        (call $f32_roundToInt (f32x4.extract_lane 2 (local.get $a)) (i32.const 0) (i32.const 0)))
      (call $f32_roundToInt (f32x4.extract_lane 3 (local.get $a)) (i32.const 0) (i32.const 0))))

  ;; =====================================================================
  ;; f32x4 comparisons — result is i32x4 mask (0x00000000 or 0xFFFFFFFF)
  ;; scalar comparisons return i32 0/1; negate to get 0/0xFFFFFFFF
  ;; =====================================================================

  (func $f32x4_eq (export "f32x4_eq") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (i32.const 0) (call $f32_eq (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b)))))
          (i32.sub (i32.const 0) (call $f32_eq (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b)))))
        (i32.sub (i32.const 0) (call $f32_eq (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b)))))
      (i32.sub (i32.const 0) (call $f32_eq (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b))))))

  (func $f32x4_ne (export "f32x4_ne") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (call $f32_eq (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b))) (i32.const 1)))
          (i32.sub (call $f32_eq (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b))) (i32.const 1)))
        (i32.sub (call $f32_eq (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b))) (i32.const 1)))
      (i32.sub (call $f32_eq (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b))) (i32.const 1))))

  (func $f32x4_lt (export "f32x4_lt") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b)))))
          (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b)))))
        (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b)))))
      (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b))))))

  (func $f32x4_gt (export "f32x4_gt") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 0 (local.get $b)) (f32x4.extract_lane 0 (local.get $a)))))
          (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 1 (local.get $b)) (f32x4.extract_lane 1 (local.get $a)))))
        (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 2 (local.get $b)) (f32x4.extract_lane 2 (local.get $a)))))
      (i32.sub (i32.const 0) (call $f32_lt_quiet (f32x4.extract_lane 3 (local.get $b)) (f32x4.extract_lane 3 (local.get $a))))))

  (func $f32x4_le (export "f32x4_le") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 0 (local.get $a)) (f32x4.extract_lane 0 (local.get $b)))))
          (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 1 (local.get $a)) (f32x4.extract_lane 1 (local.get $b)))))
        (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 2 (local.get $a)) (f32x4.extract_lane 2 (local.get $b)))))
      (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 3 (local.get $a)) (f32x4.extract_lane 3 (local.get $b))))))

  (func $f32x4_ge (export "f32x4_ge") (param $a v128) (param $b v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 0 (local.get $b)) (f32x4.extract_lane 0 (local.get $a)))))
          (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 1 (local.get $b)) (f32x4.extract_lane 1 (local.get $a)))))
        (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 2 (local.get $b)) (f32x4.extract_lane 2 (local.get $a)))))
      (i32.sub (i32.const 0) (call $f32_le_quiet (f32x4.extract_lane 3 (local.get $b)) (f32x4.extract_lane 3 (local.get $a))))))

  ;; =====================================================================
  ;; f32x4 conversions
  ;; =====================================================================

  (func $f32x4_convert_i32x4_s (export "f32x4_convert_i32x4_s") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $i32_to_f32 (i32x4.extract_lane 0 (local.get $a))))
          (call $i32_to_f32 (i32x4.extract_lane 1 (local.get $a))))
        (call $i32_to_f32 (i32x4.extract_lane 2 (local.get $a))))
      (call $i32_to_f32 (i32x4.extract_lane 3 (local.get $a)))))

  (func $f32x4_convert_i32x4_u (export "f32x4_convert_i32x4_u") (param $a v128) (result v128)
    (f32x4.replace_lane 3
      (f32x4.replace_lane 2
        (f32x4.replace_lane 1
          (f32x4.splat
            (call $ui32_to_f32 (i32x4.extract_lane 0 (local.get $a))))
          (call $ui32_to_f32 (i32x4.extract_lane 1 (local.get $a))))
        (call $ui32_to_f32 (i32x4.extract_lane 2 (local.get $a))))
      (call $ui32_to_f32 (i32x4.extract_lane 3 (local.get $a)))))

  ;; demote 2 f64 lanes to f32, zero-fill lanes 2-3
  (func $f32x4_demote_f64x2_zero (export "f32x4_demote_f64x2_zero") (param $a v128) (result v128)
    (f32x4.replace_lane 1
      (f32x4.replace_lane 0
        (v128.const i32x4 0 0 0 0)
        (call $f64_to_f32 (f64x2.extract_lane 0 (local.get $a))))
      (call $f64_to_f32 (f64x2.extract_lane 1 (local.get $a)))))

  ;; =====================================================================
  ;; f64x2 arithmetic
  ;; =====================================================================

  (func $f64x2_add (export "f64x2_add") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_add (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_add (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_sub (export "f64x2_sub") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_sub (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_sub (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_mul (export "f64x2_mul") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_mul (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_mul (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_div (export "f64x2_div") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_div (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_div (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_sqrt (export "f64x2_sqrt") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_sqrt (f64x2.extract_lane 0 (local.get $a))))
      (call $f64_sqrt (f64x2.extract_lane 1 (local.get $a)))))

  (func $f64x2_neg (export "f64x2_neg") (param $a v128) (result v128)
    (v128.xor (local.get $a) (v128.const i64x2 0x8000000000000000 0x8000000000000000)))

  (func $f64x2_abs (export "f64x2_abs") (param $a v128) (result v128)
    (v128.and (local.get $a) (v128.const i64x2 0x7FFFFFFFFFFFFFFF 0x7FFFFFFFFFFFFFFF)))

  ;; =====================================================================
  ;; f64x2 min/max
  ;; =====================================================================

  (func $f64x2_min (export "f64x2_min") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_min (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_min (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_max (export "f64x2_max") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_max (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))
      (call $f64_max (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))

  (func $f64x2_pmin (export "f64x2_pmin") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (select (f64x2.extract_lane 0 (local.get $b)) (f64x2.extract_lane 0 (local.get $a))
          (call $f64_lt_quiet (f64x2.extract_lane 0 (local.get $b)) (f64x2.extract_lane 0 (local.get $a)))))
      (select (f64x2.extract_lane 1 (local.get $b)) (f64x2.extract_lane 1 (local.get $a))
        (call $f64_lt_quiet (f64x2.extract_lane 1 (local.get $b)) (f64x2.extract_lane 1 (local.get $a))))))

  (func $f64x2_pmax (export "f64x2_pmax") (param $a v128) (param $b v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (select (f64x2.extract_lane 0 (local.get $b)) (f64x2.extract_lane 0 (local.get $a))
          (call $f64_lt_quiet (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b)))))
      (select (f64x2.extract_lane 1 (local.get $b)) (f64x2.extract_lane 1 (local.get $a))
        (call $f64_lt_quiet (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b))))))

  ;; =====================================================================
  ;; f64x2 rounding
  ;; =====================================================================

  (func $f64x2_ceil (export "f64x2_ceil") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_roundToInt (f64x2.extract_lane 0 (local.get $a)) (i32.const 3) (i32.const 0)))
      (call $f64_roundToInt (f64x2.extract_lane 1 (local.get $a)) (i32.const 3) (i32.const 0))))

  (func $f64x2_floor (export "f64x2_floor") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_roundToInt (f64x2.extract_lane 0 (local.get $a)) (i32.const 2) (i32.const 0)))
      (call $f64_roundToInt (f64x2.extract_lane 1 (local.get $a)) (i32.const 2) (i32.const 0))))

  (func $f64x2_trunc (export "f64x2_trunc") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_roundToInt (f64x2.extract_lane 0 (local.get $a)) (i32.const 1) (i32.const 0)))
      (call $f64_roundToInt (f64x2.extract_lane 1 (local.get $a)) (i32.const 1) (i32.const 0))))

  (func $f64x2_nearest (export "f64x2_nearest") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f64_roundToInt (f64x2.extract_lane 0 (local.get $a)) (i32.const 0) (i32.const 0)))
      (call $f64_roundToInt (f64x2.extract_lane 1 (local.get $a)) (i32.const 0) (i32.const 0))))

  ;; =====================================================================
  ;; f64x2 comparisons — result is i64x2 mask (0 or 0xFFFFFFFFFFFFFFFF)
  ;; scalar returns i32 0/1; negate then sign-extend to i64
  ;; =====================================================================

  (func $f64x2_eq (export "f64x2_eq") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub (i32.const 0)
          (call $f64_eq (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))))
      (i64.extend_i32_s (i32.sub (i32.const 0)
        (call $f64_eq (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))))

  (func $f64x2_ne (export "f64x2_ne") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub
          (call $f64_eq (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b)))
          (i32.const 1))))
      (i64.extend_i32_s (i32.sub
        (call $f64_eq (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))
        (i32.const 1)))))

  (func $f64x2_lt (export "f64x2_lt") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub (i32.const 0)
          (call $f64_lt_quiet (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))))
      (i64.extend_i32_s (i32.sub (i32.const 0)
        (call $f64_lt_quiet (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))))

  (func $f64x2_gt (export "f64x2_gt") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub (i32.const 0)
          (call $f64_lt_quiet (f64x2.extract_lane 0 (local.get $b)) (f64x2.extract_lane 0 (local.get $a))))))
      (i64.extend_i32_s (i32.sub (i32.const 0)
        (call $f64_lt_quiet (f64x2.extract_lane 1 (local.get $b)) (f64x2.extract_lane 1 (local.get $a)))))))

  (func $f64x2_le (export "f64x2_le") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub (i32.const 0)
          (call $f64_le_quiet (f64x2.extract_lane 0 (local.get $a)) (f64x2.extract_lane 0 (local.get $b))))))
      (i64.extend_i32_s (i32.sub (i32.const 0)
        (call $f64_le_quiet (f64x2.extract_lane 1 (local.get $a)) (f64x2.extract_lane 1 (local.get $b)))))))

  (func $f64x2_ge (export "f64x2_ge") (param $a v128) (param $b v128) (result v128)
    (i64x2.replace_lane 1
      (i64x2.splat
        (i64.extend_i32_s (i32.sub (i32.const 0)
          (call $f64_le_quiet (f64x2.extract_lane 0 (local.get $b)) (f64x2.extract_lane 0 (local.get $a))))))
      (i64.extend_i32_s (i32.sub (i32.const 0)
        (call $f64_le_quiet (f64x2.extract_lane 1 (local.get $b)) (f64x2.extract_lane 1 (local.get $a)))))))

  ;; =====================================================================
  ;; f64x2 conversions
  ;; =====================================================================

  ;; convert low 2 signed i32 lanes to f64
  (func $f64x2_convert_low_i32x4_s (export "f64x2_convert_low_i32x4_s") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $i32_to_f64 (i32x4.extract_lane 0 (local.get $a))))
      (call $i32_to_f64 (i32x4.extract_lane 1 (local.get $a)))))

  ;; convert low 2 unsigned i32 lanes to f64
  (func $f64x2_convert_low_i32x4_u (export "f64x2_convert_low_i32x4_u") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $ui32_to_f64 (i32x4.extract_lane 0 (local.get $a))))
      (call $ui32_to_f64 (i32x4.extract_lane 1 (local.get $a)))))

  ;; promote low 2 f32 lanes to f64
  (func $f64x2_promote_low_f32x4 (export "f64x2_promote_low_f32x4") (param $a v128) (result v128)
    (f64x2.replace_lane 1
      (f64x2.splat
        (call $f32_to_f64 (f32x4.extract_lane 0 (local.get $a))))
      (call $f32_to_f64 (f32x4.extract_lane 1 (local.get $a)))))

  ;; =====================================================================
  ;; i32x4 saturating truncation
  ;; =====================================================================

  (func $i32x4_trunc_sat_f32x4_s (export "i32x4_trunc_sat_f32x4_s") (param $a v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (call $f32_to_i32_sat (f32x4.extract_lane 0 (local.get $a))))
          (call $f32_to_i32_sat (f32x4.extract_lane 1 (local.get $a))))
        (call $f32_to_i32_sat (f32x4.extract_lane 2 (local.get $a))))
      (call $f32_to_i32_sat (f32x4.extract_lane 3 (local.get $a)))))

  (func $i32x4_trunc_sat_f32x4_u (export "i32x4_trunc_sat_f32x4_u") (param $a v128) (result v128)
    (i32x4.replace_lane 3
      (i32x4.replace_lane 2
        (i32x4.replace_lane 1
          (i32x4.splat
            (call $f32_to_ui32_sat (f32x4.extract_lane 0 (local.get $a))))
          (call $f32_to_ui32_sat (f32x4.extract_lane 1 (local.get $a))))
        (call $f32_to_ui32_sat (f32x4.extract_lane 2 (local.get $a))))
      (call $f32_to_ui32_sat (f32x4.extract_lane 3 (local.get $a)))))

  ;; truncate 2 f64 lanes to i32, zero-fill lanes 2-3
  (func $i32x4_trunc_sat_f64x2_s_zero (export "i32x4_trunc_sat_f64x2_s_zero") (param $a v128) (result v128)
    (i32x4.replace_lane 1
      (i32x4.replace_lane 0
        (v128.const i32x4 0 0 0 0)
        (call $f64_to_i32_sat (f64x2.extract_lane 0 (local.get $a))))
      (call $f64_to_i32_sat (f64x2.extract_lane 1 (local.get $a)))))

  (func $i32x4_trunc_sat_f64x2_u_zero (export "i32x4_trunc_sat_f64x2_u_zero") (param $a v128) (result v128)
    (i32x4.replace_lane 1
      (i32x4.replace_lane 0
        (v128.const i32x4 0 0 0 0)
        (call $f64_to_ui32_sat (f64x2.extract_lane 0 (local.get $a))))
      (call $f64_to_ui32_sat (f64x2.extract_lane 1 (local.get $a)))))
)
