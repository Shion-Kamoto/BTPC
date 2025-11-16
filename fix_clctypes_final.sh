#!/bin/bash
# Final fix: Remove half typedef, add OpenCL built-in function declarations

sudo tee /usr/include/clc/clctypes.h > /dev/null << 'EOF'
#ifndef __CLC_CLCTYPES_H__
#define __CLC_CLCTYPES_H__

// Scalar unsigned types (NOT vectors)
typedef unsigned char uchar;
typedef unsigned short ushort;
typedef unsigned int uint;
typedef unsigned long ulong;

// Vector types (2, 3, 4, 8, 16 element vectors)
typedef char __attribute__((ext_vector_type(2))) char2;
typedef char __attribute__((ext_vector_type(3))) char3;
typedef char __attribute__((ext_vector_type(4))) char4;
typedef char __attribute__((ext_vector_type(8))) char8;
typedef char __attribute__((ext_vector_type(16))) char16;

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

// DO NOT define half - let OpenCL compiler provide it

// OpenCL built-in function declarations
typedef unsigned long size_t;
size_t get_global_id(uint dimindx);
size_t get_local_id(uint dimindx);
size_t get_global_size(uint dimindx);
size_t get_local_size(uint dimindx);
size_t get_num_groups(uint dimindx);
size_t get_group_id(uint dimindx);
size_t get_global_offset(uint dimindx);

// Atomic functions
uint atomic_cmpxchg(__global uint *p, uint cmp, uint val);
uint atomic_xchg(__global uint *p, uint val);
uint atomic_inc(__global uint *p);
uint atomic_dec(__global uint *p);
uint atomic_add(__global uint *p, uint val);
uint atomic_sub(__global uint *p, uint val);

#endif
EOF

echo "✅ Final clctypes.h (removed half, added built-ins)"
echo "Test: cargo run"