#include <stdbool.h>
#include <stdint.h>

#include "platform.h"

#include "internals.h"
#include "softfloat.h"
#include "specialize.h"

/* ===== Bit manipulation helpers ===== */

static inline uint32_t
f32_bits(float32_t f)
{
	union ui32_f32 u = { .f = f };
	return u.ui;
}

static inline float32_t
f32_from_bits(uint32_t ui)
{
	union ui32_f32 u = { .ui = ui };
	return u.f;
}

static inline uint64_t
f64_bits(float64_t f)
{
	union ui64_f64 u = { .f = f };
	return u.ui;
}

static inline float64_t
f64_from_bits(uint64_t ui)
{
	union ui64_f64 u = { .ui = ui };
	return u.f;
}

bool
f32_is_nan(float32_t a);

bool
f32_i_is_nan(int32_t a);

bool
f64_is_nan(float64_t a);

bool
f64_i_is_nan(int64_t a);

static inline float32_t
f32_canon_nan(void)
{
	return f32_from_bits(defaultNaNF32UI);
}

static inline float64_t
f64_canon_nan(void)
{
	return f64_from_bits(defaultNaNF64UI);
}

/* ========================================================================
 * Scalar comparisons (gt/ge via swapped lt/le)
 * ======================================================================== */

export bool
f64_gt_quiet(float64_t l, float64_t r)
{
	return f64_lt_quiet(r, l);
}

export bool
f64_ge_quiet(float64_t l, float64_t r)
{
	return f64_le_quiet(r, l);
}

export bool
f32_ne(float32_t a, float32_t b)
{
	return !f32_eq(a, b);
}

export bool
f64_ne(float64_t a, float64_t b)
{
	return !f64_eq(a, b);
}

export bool
f32_gt_quiet(float32_t l, float32_t r)
{
	return f32_lt_quiet(r, l);
}

export bool
f32_ge_quiet(float32_t l, float32_t r)
{
	return f32_le_quiet(r, l);
}

/* ========================================================================
 * Scalar f32 ops not in berkeley-softfloat-3
 * ======================================================================== */

export float32_t
f32_min(float32_t a, float32_t b)
{
	uint32_t ab = f32_bits(a), bb = f32_bits(b);
	if (f32_i_is_nan(ab) || f32_i_is_nan(bb))
		return f32_canon_nan();
	if (f32_i_is_zero(ab) && f32_i_is_zero(bb))
		return f32_from_bits(ab | bb);
	return f32_lt_quiet(a, b) ? a : b;
}

export float32_t
f32_max(float32_t a, float32_t b)
{
	uint32_t ab = f32_bits(a), bb = f32_bits(b);
	if (f32_i_is_nan(ab) || f32_i_is_nan(bb))
		return f32_canon_nan();
	if (f32_i_is_zero(ab) && f32_i_is_zero(bb))
		return f32_from_bits(ab & bb);
	return f32_gt_quiet(a, b) ? a : b;
}

/* ========================================================================
 * Scalar f64 ops not in berkeley-softfloat-3
 * ======================================================================== */

export float64_t
f64_min(float64_t a, float64_t b)
{
	uint64_t ab = f64_bits(a), bb = f64_bits(b);
	if (f64_i_is_nan(ab) || f64_i_is_nan(bb))
		return f64_canon_nan();
	if (f64_i_is_zero(ab & UINT64_C(0x7FFFFFFFFFFFFFFF)) &&
			f64_i_is_zero(bb & UINT64_C(0x7FFFFFFFFFFFFFFF)))
		return f64_from_bits(ab | bb);
	return f64_lt_quiet(a, b) ? a : b;
}

export float64_t
f64_max(float64_t a, float64_t b)
{
	uint64_t ab = f64_bits(a), bb = f64_bits(b);
	if (f64_i_is_nan(ab) || f64_i_is_nan(bb))
		return f64_canon_nan();
	if (f64_i_is_zero(ab & UINT64_C(0x7FFFFFFFFFFFFFFF)) &&
			f64_i_is_zero(bb & UINT64_C(0x7FFFFFFFFFFFFFFF)))
		return f64_from_bits(ab & bb);
	return f64_gt_quiet(a, b) ? a : b;
}

/* ========================================================================
 * Truncations (WASM trapping float-to-int, minMag rounding, non-exact)
 * ======================================================================== */

export int32_t
f32_to_i32_trunc(float32_t a)
{
	return f32_to_i32(a, softfloat_round_minMag, false);
}

export uint32_t
f32_to_ui32_trunc(float32_t a)
{
	return f32_to_ui32(a, softfloat_round_minMag, false);
}

export int64_t
f32_to_i64_trunc(float32_t a)
{
	return f32_to_i64(a, softfloat_round_minMag, false);
}

export uint64_t
f32_to_ui64_trunc(float32_t a)
{
	return f32_to_ui64(a, softfloat_round_minMag, false);
}

export int32_t
f64_to_i32_trunc(float64_t a)
{
	return f64_to_i32(a, softfloat_round_minMag, false);
}

export uint32_t
f64_to_ui32_trunc(float64_t a)
{
	return f64_to_ui32(a, softfloat_round_minMag, false);
}

export int64_t
f64_to_i64_trunc(float64_t a)
{
	return f64_to_i64(a, softfloat_round_minMag, false);
}

export uint64_t
f64_to_ui64_trunc(float64_t a)
{
	return f64_to_ui64(a, softfloat_round_minMag, false);
}

/* ========================================================================
 * Round-to-integer-as-float wrappers
 * ======================================================================== */

export float32_t
f32_floor(float32_t a)
{
	return f32_roundToInt(a, softfloat_round_min, false);
}
export float32_t
f32_ceil(float32_t a)
{
	return f32_roundToInt(a, softfloat_round_max, false);
}
export float32_t
f32_trunc(float32_t a)
{
	return f32_roundToInt(a, softfloat_round_minMag, false);
}
export float32_t
f32_nearest(float32_t a)
{
	return f32_roundToInt(a, softfloat_round_near_even, false);
}

export float64_t
f64_floor(float64_t a)
{
	return f64_roundToInt(a, softfloat_round_min, false);
}
export float64_t
f64_ceil(float64_t a)
{
	return f64_roundToInt(a, softfloat_round_max, false);
}
export float64_t
f64_trunc(float64_t a)
{
	return f64_roundToInt(a, softfloat_round_minMag, false);
}
export float64_t
f64_nearest(float64_t a)
{
	return f64_roundToInt(a, softfloat_round_near_even, false);
}

/* ========================================================================
 * Saturating truncations (WASM nontrapping float-to-int)
 *
 * softfloat _r_minMag already returns clamped values on overflow matching
 * WASM sat semantics, but returns fromNaN != 0 for NaN inputs.
 * ======================================================================== */

export int32_t
f32_to_i32_sat(float32_t a)
{
	if (f32_is_nan(a))
		return 0;
	return (int32_t)f32_to_i32_r_minMag(a, false);
}

export uint32_t
f32_to_ui32_sat(float32_t a)
{
	if (f32_is_nan(a))
		return 0;
	return (uint32_t)f32_to_ui32_r_minMag(a, false);
}

export int64_t
f32_to_i64_sat(float32_t a)
{
	if (f32_is_nan(a))
		return 0;
	return (int64_t)f32_to_i64_r_minMag(a, false);
}

export uint64_t
f32_to_ui64_sat(float32_t a)
{
	if (f32_is_nan(a))
		return 0;
	return (uint64_t)f32_to_ui64_r_minMag(a, false);
}

export int32_t
f64_to_i32_sat(float64_t a)
{
	if (f64_is_nan(a))
		return 0;
	return (int32_t)f64_to_i32_r_minMag(a, false);
}

export uint32_t
f64_to_ui32_sat(float64_t a)
{
	if (f64_is_nan(a))
		return 0;
	return (uint32_t)f64_to_ui32_r_minMag(a, false);
}

export int64_t
f64_to_i64_sat(float64_t a)
{
	if (f64_is_nan(a))
		return 0;
	return (int64_t)f64_to_i64_r_minMag(a, false);
}

export uint64_t
f64_to_ui64_sat(float64_t a)
{
	if (f64_is_nan(a))
		return 0;
	return (uint64_t)f64_to_ui64_r_minMag(a, false);
}
