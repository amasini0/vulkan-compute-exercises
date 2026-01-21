// Uniform buffer bound to set 0, index 0
ConstantBuffer<int3> numbers;

[shader("compute")]
[numthreads(1,1,1)]
void main(uint3 thread_id: SV_DispatchThreadID) {
    int sum = 0;
    sum += numbers[0];
    sum += numbers[1];
    sum += numbers[2];

    if (thread_id.x == 0) {
	    printf("Good job, now you deserve a %x!\n", sum);
	}
}