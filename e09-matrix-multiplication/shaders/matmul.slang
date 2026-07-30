#language slang 2025

static const TILE_SIZE = 16;

// Bindings.
Buffer<float> matrix_a;
Buffer<float> matrix_b;
RWBuffer<float> matrix_c;

groupshared float tile_a[TILE_SIZE][TILE_SIZE];
groupshared float tile_b[TILE_SIZE][TILE_SIZE];

[shader("compute")]
[numthreads(TILE_SIZE, TILE_SIZE, 1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    let x = thread_id.x;
    let y = thread_id.y;

    // Load element from matrix a/b in tile a/b
    printf("x: %d, y: %d\n", x, y);
}
