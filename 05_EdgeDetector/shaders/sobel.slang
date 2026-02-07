struct ImageParams
{
    uint width;
    uint height;
}

// Binding 0, image params
ConstantBuffer<ImageParams> params;

// Binding 1, source image
RWStructuredBuffer<uint> incoming;

// Binding 2, result image
RWStructuredBuffer<uint> outgoing;

// Decompress 256-color RGBA into a float4 with colors in range [0,1]
float4 decompress(uint rgba)
{
    float4 result;
    result.r = (rgba & 0xff) / 255.0f;
    result.g = ((rgba >> 8) & 0xff) / 255.0f;
    result.b = ((rgba >> 16) & 0xff) / 255.0f;
    result.a = ((rgba >> 24) & 0xff) / 255.0f;
    return result;
}

// Compress a float4 with values in [0, 1] to 256-color RGBA
uint compress(float4 rgba)
{
    uint result = 0;
    rgba = clamp(rgba, 0, 1);
    result = uint(rgba.r * 255.0f);
    result |= uint(rgba.g * 255.0f) << 8;
    result |= uint(rgba.b * 255.0f) << 16;
    result |= uint(rgba.a * 255.0f) << 24;
    return result;
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 thread_id: SV_DispatchThreadID)
{
    const int x = int(thread_id.x);
    const int y = int(thread_id.y);

    // Stop threads that would read/write outside the image
    if (x >= params.width || y >= params.height)
        return;

    // Sobel filter in x and y directions
    // https://en.wikipedia.org/wiki/Sobel_operator
    float sobel[3][3] = {
        { 1, 0, -1 },
        { 2, 0, -2 },
        { 1, 0, -1 },
    };

    // Summed RGBA filter results
    var Gx = float4(0, 0, 0, 1);
    var Gy = float4(0, 0, 0, 1);

    // Compute the linear index of this thread's pixel location in the image.
    // Iterate over a 3x3 2D window around the thread's pixel location.
    // For this, make sure you don't read outside the image: clamp the reading
    // coordinates to 0 from below, and to width - 1 from above for X,
    // and to height - 1 from above for Y.
    // Read the compressed integer value at the corresponding location from the
    // input image, and use decompress to turn it into a float4 with RGBA in
    // range [0, 1].
    // Multiply it with the appropriate weight from the given Sobel filter to do
    // detection of horizontal lines and accumulate the result in Gx.
    // Do the same for detection of vertical lines and store the result in Gy.
    //
    // When you processed the whole window, take the absolute values of Gx and
    // Gy and add them together. Compress the result and write it to this
    // thread's linear pixel index in the output image.

    for (int i = -1; i <= 1; ++i)
    {
        for (int j = -1; j <= 1; ++j)
        {
            var pixel_idx = clamp(y + i, 0, params.height - 1) * params.width +
                            clamp(x + j, 0, params.width - 1);
            var pixel_val = decompress(incoming[pixel_idx]);
            Gx += pixel_val * sobel[i + 1][j + 1];
            Gy += pixel_val * sobel[j + 1][i + 1];
        }
    }

    var pixel_idx = y * params.width + x;
    var pixel_val = compress(sqrt(Gx * Gx + Gy * Gy));
    outgoing[pixel_idx] = pixel_val;
}
