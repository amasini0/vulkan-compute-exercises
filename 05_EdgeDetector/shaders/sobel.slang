// Bindings
ConstantBuffer<uint2> image_params;
RWStructuredBuffer<uint> source_image;
RWStructuredBuffer<uint> result_image;

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    uint width = image_params.x;
    uint height = image_params.y;

    if (thread_id.x == 0 && thread_id.y == 0) {
        printf("width: %u, height: %u\n", width, height);
        for (uint i = 50932; i < 50934; i++)
            printf("%u -> %u\n", i, source_image[i]);
    }

    if (thread_id.x < width && thread_id.y < height) {
        uint pixel_id = thread_id.y * width + thread_id.x;
        result_image[pixel_id] = source_image[pixel_id];
    }
}