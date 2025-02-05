#include <stdbool.h>

#include "platform.h"
#include "softfloat.h"

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
f32_gt_quiet(float32_t l, float32_t r)
{
	return f32_lt_quiet(r, l);
}

export bool
f32_ge_quiet(float32_t l, float32_t r)
{
	return f32_le_quiet(r, l);
}
