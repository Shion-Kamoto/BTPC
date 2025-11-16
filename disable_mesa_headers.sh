#!/bin/bash
# Temporarily disable Mesa's auto-included headers
# Allows self-contained OpenCL kernels to compile

set -e

echo "Disabling Mesa libclc auto-include..."

# Backup clc.h
if [ ! -f /usr/include/clc/clc.h.bak ]; then
    sudo cp /usr/include/clc/clc.h /usr/include/clc/clc.h.bak
    echo "✅ Backed up clc.h"
fi

# Replace clc.h with minimal stub (empty, just guard)
sudo tee /usr/include/clc/clc.h > /dev/null << 'EOF'
#ifndef __CLC_H__
#define __CLC_H__
/* Stub header - Mesa auto-includes disabled for self-contained kernels */
#endif
EOF

echo "✅ Mesa headers disabled (clc.h stubbed)"
echo ""
echo "Test: cd test_opencl_diagnostic && cargo run"
echo ""
echo "To restore: sudo mv /usr/include/clc/clc.h.bak /usr/include/clc/clc.h"