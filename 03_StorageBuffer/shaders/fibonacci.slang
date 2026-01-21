static const float phi = 0.5 * (1.0 + sqrt(5));
static const float psi = 0.5 * (1.0 - sqrt(5));

// Storage buffer bound to set 0, index 0
RWStructuredBuffer<int> buffer;

[shader("compute")]
[numthreads(32, 1, 1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    const uint n = thread_id.x;
    const float phi_n = pow(phi, n);
    const float psi_n = ((n % 2 == 1) ? -1 : 1) * pow(-psi, n); // WTF

    buffer[n] = int((phi_n - psi_n) / sqrt(5.0));
}
