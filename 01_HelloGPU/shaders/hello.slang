[shader("compute")]
[numthreads(1,1,1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    if (thread_id.x == 0)
        printf("Hello world from thread %u", thread_id.x);
}