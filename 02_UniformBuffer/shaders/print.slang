struct UniformBuffer {
    int a;
    int b;
    int c;
}

ConstantBuffer<UniformBuffer> ubo;

[shader("compute")]
[numthreads(1,1,1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    int sum = 0;

    sum += ubo.a;
    sum += ubo.b;
    sum += ubo.c;

    if (thread_id.x == 0) {
	    printf("Good job, now you deserve a %x!\n", sum);
	}
}