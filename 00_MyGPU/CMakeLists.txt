# 00_MyGPU application target
add_executable(00_MyGPU main.cpp)
target_link_libraries(00_MyGPU PRIVATE Vulkan::Vulkan)