// NOTE: This shader requires being manually compiled to SPIR-V in order to
// avoid having downstream users require building shaderc and compiling the
// shader themselves. If you update this shader, be sure to also re-compile it.
//.You can do so using `glslangValidator` with the
// following command: `glslangValidator -V shader.vert -o shader.vert.spv`

#version 450

layout(location = 0) in vec2 position;
layout(location = 0) out vec4 ocolor;

layout(set = 0, binding = 0) uniform Data {
    vec4 dims;
} uniforms;

//here is the layout we need to match 
//when we decr
layout(location = 1) in vec4 point;
layout(location = 2) in vec4 data2;

void main() {
    ocolor = data2;
    vec2 center= point.xy;
    float size = point.z;
    vec2 p = position*vec2(uniforms.dims.y/uniforms.dims.x,1.0);
    gl_Position =  vec4(p*size+vec2(2.0*center.x/uniforms.dims.x, 2.0*center.y/uniforms.dims.y),0.0, 1.0);
}
