#!/bin/bash
# Install stub headers for libclc-20-dev Ubuntu packaging bug workaround
# See: https://github.com/llvm/llvm-project/issues/119967
# Date: 2025-11-09
# Feature: 009-integrate-gpu-mining

set -e

echo "Installing stub headers for libclc OpenCL compilation..."

# Create clcfunc.h
sudo tee /usr/include/clc/clcfunc.h > /dev/null << 'EOF'
#ifndef __CLC_CLCFUNC_H__
#define __CLC_CLCFUNC_H__
#define _CLC_OVERLOAD __attribute__((overloadable))
#define _CLC_DECL
#define _CLC_DEF
#define _CLC_INLINE __attribute__((always_inline)) inline
#define _CLC_CONVERGENT __attribute__((convergent))
#endif
EOF

echo "✅ Created /usr/include/clc/clcfunc.h"

# Create clctypes.h with FULL vector type definitions
sudo tee /usr/include/clc/clctypes.h > /dev/null << 'EOF'
#ifndef __CLC_CLCTYPES_H__
#define __CLC_CLCTYPES_H__

// Vector types for Mesa libclc compatibility
typedef char __attribute__((ext_vector_type(2))) char2;
typedef char __attribute__((ext_vector_type(3))) char3;
typedef char __attribute__((ext_vector_type(4))) char4;
typedef char __attribute__((ext_vector_type(8))) char8;
typedef char __attribute__((ext_vector_type(16))) char16;

typedef unsigned char __attribute__((ext_vector_type(1))) uchar;
typedef unsigned char __attribute__((ext_vector_type(2))) uchar2;
typedef unsigned char __attribute__((ext_vector_type(3))) uchar3;
typedef unsigned char __attribute__((ext_vector_type(4))) uchar4;
typedef unsigned char __attribute__((ext_vector_type(8))) uchar8;
typedef unsigned char __attribute__((ext_vector_type(16))) uchar16;

typedef short __attribute__((ext_vector_type(2))) short2;
typedef short __attribute__((ext_vector_type(3))) short3;
typedef short __attribute__((ext_vector_type(4))) short4;
typedef short __attribute__((ext_vector_type(8))) short8;
typedef short __attribute__((ext_vector_type(16))) short16;

typedef unsigned short __attribute__((ext_vector_type(1))) ushort;
typedef unsigned short __attribute__((ext_vector_type(2))) ushort2;
typedef unsigned short __attribute__((ext_vector_type(3))) ushort3;
typedef unsigned short __attribute__((ext_vector_type(4))) ushort4;
typedef unsigned short __attribute__((ext_vector_type(8))) ushort8;
typedef unsigned short __attribute__((ext_vector_type(16))) ushort16;

typedef int __attribute__((ext_vector_type(2))) int2;
typedef int __attribute__((ext_vector_type(3))) int3;
typedef int __attribute__((ext_vector_type(4))) int4;
typedef int __attribute__((ext_vector_type(8))) int8;
typedef int __attribute__((ext_vector_type(16))) int16;

typedef unsigned int __attribute__((ext_vector_type(1))) uint;
typedef unsigned int __attribute__((ext_vector_type(2))) uint2;
typedef unsigned int __attribute__((ext_vector_type(3))) uint3;
typedef unsigned int __attribute__((ext_vector_type(4))) uint4;
typedef unsigned int __attribute__((ext_vector_type(8))) uint8;
typedef unsigned int __attribute__((ext_vector_type(16))) uint16;

typedef long __attribute__((ext_vector_type(2))) long2;
typedef long __attribute__((ext_vector_type(3))) long3;
typedef long __attribute__((ext_vector_type(4))) long4;
typedef long __attribute__((ext_vector_type(8))) long8;
typedef long __attribute__((ext_vector_type(16))) long16;

typedef unsigned long __attribute__((ext_vector_type(1))) ulong;
typedef unsigned long __attribute__((ext_vector_type(2))) ulong2;
typedef unsigned long __attribute__((ext_vector_type(3))) ulong3;
typedef unsigned long __attribute__((ext_vector_type(4))) ulong4;
typedef unsigned long __attribute__((ext_vector_type(8))) ulong8;
typedef unsigned long __attribute__((ext_vector_type(16))) ulong16;

typedef float __attribute__((ext_vector_type(2))) float2;
typedef float __attribute__((ext_vector_type(3))) float3;
typedef float __attribute__((ext_vector_type(4))) float4;
typedef float __attribute__((ext_vector_type(8))) float8;
typedef float __attribute__((ext_vector_type(16))) float16;

typedef double __attribute__((ext_vector_type(2))) double2;
typedef double __attribute__((ext_vector_type(3))) double3;
typedef double __attribute__((ext_vector_type(4))) double4;
typedef double __attribute__((ext_vector_type(8))) double8;
typedef double __attribute__((ext_vector_type(16))) double16;

// Half precision (fp16) types
typedef __fp16 half;
typedef __fp16 __attribute__((ext_vector_type(2))) half2;
typedef __fp16 __attribute__((ext_vector_type(3))) half3;
typedef __fp16 __attribute__((ext_vector_type(4))) half4;
typedef __fp16 __attribute__((ext_vector_type(8))) half8;
typedef __fp16 __attribute__((ext_vector_type(16))) half16;

#endif
EOF

echo "✅ Created /usr/include/clc/clctypes.h (with vector types)"

# Patch convert.h to include types FIRST
echo "Patching /usr/include/clc/convert.h to include clctypes.h..."
if ! grep -q "clctypes.h" /usr/include/clc/convert.h 2>/dev/null; then
    sudo sed -i '1i #include "clctypes.h"' /usr/include/clc/convert.h
    echo "✅ Patched convert.h"
else
    echo "✅ convert.h already patched"
fi

# Verify installation
if [ -f /usr/include/clc/clcfunc.h ] && [ -f /usr/include/clc/clctypes.h ]; then
    echo "✅ Stub headers installed successfully!"
    echo ""
    echo "Next steps:"
    echo "1. Test GPU mining: /home/bob/.btpc/bin/btpc_miner --gpu --address btpc1qtest"
    echo "2. Check for 'GPU mode' in mining thread output"
    echo "3. Verify no 'clc/clcfunc.h' error"
    echo ""
    echo "To revert (if needed):"
    echo "  sudo rm /usr/include/clc/clcfunc.h /usr/include/clc/clctypes.h"
else
    echo "❌ Installation failed - files not created"
    exit 1
fi