#language slang 2025

// Shader parameters
RWStructuredBuffer<Atomic<uint>> counter;
RWStructuredBuffer<uint> output_buffer;

// Primality test
bool is_prime(uint n)
{
    var limit = sqrt(n);
    for (uint k = 2; k <= limit; ++k)
    {
        if (n % k == 0)
            return false;
    }
    return true;
}

[shader("compute")]
[numthreads(128, 1, 1)]
void main(uint3 thread_id: SV_DispatchThreadID)
{
    var is_prime_id = is_prime(thread_id.x);
    if (is_prime_id)
    {
        var output_idx = counter[0].increment();
        output_buffer[output_idx] = thread_id.x;
    }
}
