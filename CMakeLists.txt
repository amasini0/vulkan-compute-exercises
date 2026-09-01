cmake_minimum_required(VERSION 3.30)
project(VulkanComputeExercises LANGUAGES CXX)

# Add custom modules to module path
list(APPEND CMAKE_MODULE_PATH ${CMAKE_SOURCE_DIR}/cmake)

# Find project dependencies
find_package(Vulkan REQUIRED MODULE)

# Project options
option(DEBUG_SHADERS "Build debug info for shader files" OFF)
if (DEBUG_SHADERS)
	add_compile_definitions(DEBUG_SHADERS)
endif ()

# Define project build settings
add_library(shared_configs INTERFACE EXCLUDE_FROM_ALL)
target_compile_options(shared_configs INTERFACE -Wall -Wextra -Wpedantic)
target_compile_features(shared_configs INTERFACE cxx_std_20)
add_library(project::shared_configs ALIAS shared_configs)

# Add framework library
# add_subdirectory(framework)

# Add applications
add_subdirectory(00_MyGPU)
#add_subdirectory(01_HelloGPU)
#add_subdirectory(02_UniformBuffer)
#add_subdirectory(03_StorageBuffer)
#add_subdirectory(04_Copying)
#add_subdirectory(05_EdgeDetector)
#add_subdirectory(06_Atomics)
#add_subdirectory(07_PointCloudRender)
#add_subdirectory(08_SharedMemory)
#add_subdirectory(09_MatrixMultiplication)
#add_subdirectory(10_Reduction)
#add_subdirectory(11_StagingBuffer)