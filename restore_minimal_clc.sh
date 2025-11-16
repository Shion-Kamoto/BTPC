#!/bin/bash
# Restore minimal clc.h that only includes type definitions
# Skip ALL broken Mesa library functions

set -e

echo "Creating minimal clc.h (types only, no functions)..."

sudo tee /usr/include/clc/clc.h > /dev/null << 'EOF'
#ifndef __CLC_H__
#define __CLC_H__

// Only include type definitions (no functions/math/workitem)
#include "clctypes.h"
#include "clcfunc.h"

#endif
EOF

echo "✅ Minimal clc.h installed (types only, no broken functions)"
echo "Test: cd test_opencl_diagnostic && cargo run"