#[[
    GlslShaders.cmake
    ------------------
    Provides target_add_glsl_shader(), a helper function to compile
    GLSL shaders into SPIR-V (.spv) via Vulkan::glslc and properly
    wire the output into the build of the specified target.

    Requirements:
        The Vulkan SDK must be installed and expose the glslc component.
        The module resolves this dependency internally (via a guarded
        find_package(Vulkan COMPONENTS glslc) call), so no explicit
        find_package() call is required by the including project.

    Usage:
        target_add_glsl_shader(my_app shaders/triangle.vert)
        target_add_glsl_shader(my_app shaders/triangle.frag)

    Options:
        DEBUG_SHADERS  If set to ON, shaders are compiled with debug
                        information (-g).

    Notes:
        - The .spv file is generated in CMAKE_CURRENT_BINARY_DIR with
          the name "<source_filename>.spv".
        - Each shader creates a dedicated custom target (added as a
          dependency of the main target) to guarantee it is always
          rebuilt when needed, avoiding name collisions between
          shaders with the same filename across different targets.
]]

include_guard(GLOBAL)

function(target_add_glsl_shader target source)
    # Resolve the glslc dependency lazily and only once: if some other
    # part of the project already found it, this is a no-op.
    if (NOT TARGET Vulkan::glslc)
        find_package(Vulkan QUIET COMPONENTS glslc)
    endif ()

    if (NOT TARGET Vulkan::glslc)
        message(FATAL_ERROR
                "target_add_glsl_shader: Vulkan::glslc target not found. "
                "Ensure the Vulkan SDK is installed and provides the glslc component.")
    endif ()

    get_filename_component(source_abs ${source} ABSOLUTE)
    get_filename_component(filename ${source} NAME)
    set(spv_output "${CMAKE_CURRENT_BINARY_DIR}/${filename}.spv")

    set(shader_flags "")
    if (DEBUG_SHADERS)
        list(APPEND shader_flags "-g")
    endif ()

    add_custom_command(
            OUTPUT ${spv_output}
            COMMAND Vulkan::glslc ${shader_flags} ${source_abs} -o ${spv_output}
            MAIN_DEPENDENCY ${source_abs}
            DEPENDS Vulkan::glslc
            COMMENT "Compiling GLSL shader ${filename} -> ${filename}.spv"
            VERBATIM
    )

    # Add the source to the target for informational/IDE purposes only.
    target_sources(${target} PRIVATE ${source})

    # Dedicated custom target to force generation of the .spv file
    # as a real dependency of the target (unique name to avoid
    # collisions between same-named shaders across different targets).
    string(MAKE_C_IDENTIFIER "${target}_${filename}_spv" spv_target_name)
    add_custom_target(${spv_target_name} DEPENDS ${spv_output})
    add_dependencies(${target} ${spv_target_name})
endfunction()