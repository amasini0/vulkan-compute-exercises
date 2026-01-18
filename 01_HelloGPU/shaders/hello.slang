[shader("compute")]
[numthreads(8,1,1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    if (thread_id.x == 0 || thread_id.x == 7) {
        printf("Hello world from thread %u\n", thread_id.x);
    }
}