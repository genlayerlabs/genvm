#include <stdbool.h>

#include "platform.h"
#include "softfloat.h"

export
bool f64_gt_quiet(float64_t l, float64_t r)
{
    return f64_lt_quiet(r, l);
}

export
bool f64_ge_quiet(float64_t l, float64_t r)
{
    return f64_le_quiet(r, l);
}
