#language slang 2025

static const float phi = 0.5 * (1.0 + sqrt(5));
static const float psi = 0.5 * (1.0 - sqrt(5));

static const uint group_size = 32;
static const uint num_prints = 20;

// Declare a shared array large enough to hold 32 integers
groupshared int buffer[group_size];

[shader("compute")]
[numthreads(group_size, 1, 1)]
void main(uint3 thread_id: SV_DispatchThreadID)
{
    // Let each thread n compute the n-th Fibonacci number. Then let each thread
    // store his computed number in the shared array at position n.
    const uint n = thread_id.x;
    const float phi_n = pow(phi, n);
    const float psi_n = ((n % 2 == 1) ? -1 : 1) * pow(-psi, n); // WTF
    buffer[n] = int((phi_n - psi_n) / sqrt(5.0));

    // Add a barrier. Before you can use the data, you must ensure that the
    // consuming threads wait until the writers have finished execution and the
    // memory is visible. Within a block, both of these things are realized by a
    // barrier(), which makes all previous accesses visible to everyone else in
    // the group (and only in the group).
    GroupMemoryBarrier();

    // To produce a nicely organized output, thread 0 should print all computed
    // Fibonacci numbers, in order, by reading them from shared memory. Note how
    // this thread is now using data that other threads have computed! This is a
    // very powerful concept for GPU programming.
    if (thread_id.x == 0)
    {
        for (int i = 1; i <= num_prints && i < group_size; ++i)
        {
            printf("%d ", buffer[i]);
        }
        printf("\n");
    }
}
