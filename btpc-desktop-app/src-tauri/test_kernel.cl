__kernel void test_kernel(__global int *output) {
    int gid = get_global_id(0);
    output[gid] = gid * 2;
}
