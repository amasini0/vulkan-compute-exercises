#language slang 2025

struct Params
{
    uint resolution[2];
    uint num_points;
    float4x4 mvp;
}

struct Point
{
    float3 position;
    float3 color;
}

// Shader parameters
ConstantBuffer<Params> params;
RWStructuredBuffer<Point> point_cloud;
RWStructuredBuffer<Atomic<uint>> image;

// Compresses a vec3 with colors in range [0, 1] to 16-color RGB
uint compress16(float3 rgb)
{
    uint result = 0;
    rgb = clamp(rgb, 0, 1);
    result = uint(rgb.r * 15.0f);
    result |= uint(rgb.g * 15.0f) << 4;
    result |= uint(rgb.b * 15.0f) << 8;
    return result;
}

uint3 ndc2pixel(float4 point, uint width, uint height)
{
    let px = uint((point.x + 1.0) * 0.5 * (width - 1));
    let py = uint((point.y + 1.0) * 0.5 * (height - 1));
    let pz = uint(point.z * ((1 << 20) - 1));
    return uint3(px, py, pz);
}

[shader("compute")]
[numthreads(128, 1, 1)]
void main(uint3 thread_id: SV_DispatchThreadID)
{
    // Debug prints.
    // if (thread_id.x == 0)
    // {
    //     printf("GPU DATA\n=======================\n");
    //     printf(
    //         "width: %u, height: %u\n",
    //         params.resolution[0],
    //         params.resolution[1]
    //     );
    //     printf("num_points: %u\n", params.num_points);
    //     printf(
    //         "mvp { x_axis: [%f, %f, %f, %f], ... }\n",
    //         params.mvp._11,
    //         params.mvp._21,
    //         params.mvp._31,
    //         params.mvp._41
    //     );
    //     printf(
    //         "points[20] { position: [%v3f], color: [%v3f] }\n",
    //         point_cloud[20].position,
    //         point_cloud[20].color
    //     );
    //     printf("\n");
    // }

    // Implement a simple point cloud renderer with atomics, using one thread
    // per point. To do this, we have to make sure that only the closest points
    // remain visible in every pixel. We will use atomicMin for this.
    //
    // TODO: Find the id of the current thread. We have started some number of
    // threads. Maybe more than there are points? Redundant threads should not
    // try to load points because we don't have data for them. Best terminate
    // them here.
    if (thread_id.x >= params.num_points)
    {
        return;
    }
    let width = params.resolution[0];
    let height = params.resolution[1];

    // Fetch the .position of this thread's point from the point cloud buffer.
    // Turn it into a float4 (fourth coordinate should be 1) and project it
    // onto the screen by multiplying it (from the left) with the matrix .mvp
    // from the render params struct.
    let position = float4(point_cloud[thread_id.x].position, 1.0f);
    var projection = mul(params.mvp, position);

    // The result is a float4, divide it by its .w  coordinate. Now, its x and
    // y coordinates are in range (-1, 1). The z coordinate is in range (0, 1).
    // These are called "normalized device coordinates" (or NDC).
    projection /= projection.w;

    // Now we need to know which pixels to update. Convert the coordinates to
    // uints in the range [0, params.width) for x [0, params.height) for y and
    // [0, 2^20) for z (we will only use 20 bits for the depth).
    // The coordinate (-1, -1, 0) should become (0, 0, 0)
    // The coordinate (1, 1, 1) should become (width-1, height-1, 2^20 - 1).
    let pixel = ndc2pixel(projection, width, height);

    // Fetch the .color of this thread's point from the point cloud buffer and
    // compress it into a uint using the compress16 function.
    var color = compress16(point_cloud[thread_id.x].color);

    // Now, take the z coordinate (depth) you computed before, shift it by 12
    // bits (<<) and add it onto the color. Now you have a single 32bit value
    // where the highest 20 bits are depth and the lower 12 bits are RGB color:
    //      [<----depth(20)----><-RGB (12)->].
    // Note that if we sort multiple of these values, the lowest one is also
    // the one with the lowest depth.
    color |= (pixel.z << 12);

    // Traverse a 3x3 window around the current point's pixel. For each pixel,
    // perform an atomicMin with the 32bit value (combined depth and color) you
    // computed on the image data buffer.
    // The image has been initialized with all pixels set to UINT_MAX. If done
    // correctly, the color of the closest point should end up in each output
    // pixel when the shader is done.
    for (int y_offset = -1; y_offset <= 1; ++y_offset)
    {
        let y = pixel.y + y_offset;
        if (y < 0 || y >= height)
            continue;

        for (int x_offset = -1; x_offset <= 1; ++x_offset)
        {
            let x = pixel.x + x_offset;
            if (x < 0 || x >= width)
                continue;

            let pixel_idx = y * width + x;
            image[pixel_idx].min(color);
        }
    }
}
