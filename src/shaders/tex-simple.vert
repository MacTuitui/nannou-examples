#version 450

layout(location = 0) in vec2 position;
layout(location = 0) out vec2 tex_coords;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    tex_coords = vec2(position.x * 0.5 + 0.5, 1.0 - (position.y * 0.5 + 0.5));
}
