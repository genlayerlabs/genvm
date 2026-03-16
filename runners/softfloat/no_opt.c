#include <stdbool.h>
#include <stdint.h>

#include "internals.h"
#include "platform.h"
#include "softfloat.h"
#include "specialize.h"

export float32_t
f32_neg(float32_t a)
{
	return f32_from_bits(f32_bits(a) ^ 0x80000000u);
}

export float32_t
f32_abs(float32_t a)
{
	return f32_from_bits(f32_bits(a) & 0x7FFFFFFFu);
}
