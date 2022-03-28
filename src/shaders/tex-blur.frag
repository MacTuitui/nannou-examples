#version 450
//blur taken from
// https://github.com/mattdesl/lwjgl-basics/wiki/ShaderLesson5

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D tex;
layout(set = 0, binding = 1) uniform sampler tex_sampler;

//matching BlurUniform
layout(set = 0, binding = 2) uniform Data {
    vec4 dir;
    float blur;
} uniforms;

void main() {
    float hstep = uniforms.dir.x;
    float vstep = uniforms.dir.y;
    vec4 sum = vec4(0.0);
    vec2 tc = tex_coords;
    float blur = uniforms.blur;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x - 4.0*blur*hstep, tc.y - 4.0*blur*vstep)) * 0.0162162162;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x - 3.0*blur*hstep, tc.y - 3.0*blur*vstep)) * 0.0540540541;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x - 2.0*blur*hstep, tc.y - 2.0*blur*vstep)) * 0.1216216216;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x - 1.0*blur*hstep, tc.y - 1.0*blur*vstep)) * 0.1945945946;

    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x, tc.y)) * 0.2270270270;

    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x + 1.0*blur*hstep, tc.y + 1.0*blur*vstep)) * 0.1945945946;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x + 2.0*blur*hstep, tc.y + 2.0*blur*vstep)) * 0.1216216216;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x + 3.0*blur*hstep, tc.y + 3.0*blur*vstep)) * 0.0540540541;
    sum += texture(sampler2D(tex, tex_sampler), vec2(tc.x + 4.0*blur*hstep, tc.y + 4.0*blur*vstep)) * 0.0162162162;
    f_color = 1.0*sum;
}


