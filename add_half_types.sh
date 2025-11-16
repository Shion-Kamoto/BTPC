#!/bin/bash
# Add missing half (fp16) types to clctypes.h

sudo sed -i '/#endif/d' /usr/include/clc/clctypes.h

sudo tee -a /usr/include/clc/clctypes.h > /dev/null << 'EOF'

// Half precision (fp16) types
typedef __fp16 half;
typedef __fp16 __attribute__((ext_vector_type(2))) half2;
typedef __fp16 __attribute__((ext_vector_type(3))) half3;
typedef __fp16 __attribute__((ext_vector_type(4))) half4;
typedef __fp16 __attribute__((ext_vector_type(8))) half8;
typedef __fp16 __attribute__((ext_vector_type(16))) half16;

#endif
EOF

echo "✅ Added half types to clctypes.h"
echo "Test: cd test_opencl_diagnostic && cargo run"