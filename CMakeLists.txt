cmake_minimum_required(VERSION 3.8)
project(VulkanComputeExercsises)

set(CMAKE_CXX_STANDARD 17)

find_package(Vulkan REQUIRED)

include(FetchContent)
FetchContent_Declare(
	glm
	GIT_REPOSITORY https://github.com/g-truc/glm.git
	GIT_TAG        0af55ccecd98d4e5a8d1fad7de25ba429d60e863 #1.0.1
	GIT_SHALLOW    1
	EXCLUDE_FROM_ALL
)
FetchContent_MakeAvailable(glm)

OPTION(DEBUG_SHADERS "Build debug info for shader files" OFF)

IF (DEBUG_SHADERS)
	add_compile_definitions(DEBUG_SHADERS)
ENDIF()

function(addGLSL target source)
	target_sources(${target} PRIVATE ${source})
	get_filename_component(filename ${source} NAME)
	IF(DEBUG_SHADERS)
		SET(SHADER_FLAGS "-g")
	ENDIF()
	add_custom_command(
	POST_BUILD 
	MAIN_DEPENDENCY ${source}
	COMMAND Vulkan::glslc ${SHADER_FLAGS} ${CMAKE_CURRENT_SOURCE_DIR}/${source} -o ${CMAKE_CURRENT_BINARY_DIR}/${filename}.spv
	OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${filename}.spv
	)
endfunction()

add_subdirectory(framework)
add_subdirectory(00_MyGPU)
add_subdirectory(01_HelloGPU)
add_subdirectory(02_UniformBuffer)
add_subdirectory(03_StorageBuffer)
add_subdirectory(04_Copying)
add_subdirectory(05_EdgeDetector)
add_subdirectory(06_Atomics)
add_subdirectory(07_PointCloudRender)
add_subdirectory(08_SharedMemory)
add_subdirectory(09_MatrixMultiplication)
add_subdirectory(10_Reduction)
add_subdirectory(11_StagingBuffer)