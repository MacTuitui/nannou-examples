use nannou::prelude::Frame;
use nannou::prelude::*;
use nannou::rand::*;

use std::cell::RefCell;

use nannou::wgpu::util::*;
use nannou::wgpu::*;
use rand_xorshift::XorShiftRng;

use std::path::Path;

const LENGTH_FRAME: u64 = 420;
const SEED: u64 = 42;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

//two triangles to cover the whole screen
const VERTICES: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
];

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Instance {
    data: [f32; 4],
    data2: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Uniforms {
    dims: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BlurUniforms {
    dims: [f32; 4],
    blur: f32,
}

//there's a bunch of stuff not explicitely used in this example
struct Graphics {
    circle_vertex_buffer: wgpu::Buffer,
    circle_index_buffer: wgpu::Buffer,
    rect_vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    blur_uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    pipeline_layout: wgpu::PipelineLayout,
    n_circle_indices: u32,
    render_pipeline_from_texture: wgpu::RenderPipeline,
    bind_group_texture: wgpu::BindGroup,
    render_pipeline_blur: wgpu::RenderPipeline,
    bind_group_texture_blur_a: wgpu::BindGroup,
    bind_group_texture_blur_draw_texture: wgpu::BindGroup,
    bind_group_texture_a: wgpu::BindGroup,
    texture_a: wgpu::Texture,
    texture_a_view: wgpu::TextureView,
    draw_texture_view: wgpu::TextureView,
}

struct Model {
    points: Vec<Particle>,
    //wgpu stuff
    graphics: RefCell<Graphics>,
    // The texture that we will draw to (for drawing to custom)
    draw_texture: wgpu::Texture,
    // Create a `Draw` instance for drawing to our texture.
    draw: nannou::Draw,
    // The type used to render the `Draw` vertices to our texture.
    draw_renderer: nannou::draw::Renderer,
    do_blur: bool,
}

//basic struct to keep track of some things
//we're going to render those as instances
//
//comment: well maybe this could directly be *an instance*
//but I actually have a monster particle struct to do far
//more than this
struct Particle {
    position: Vec2,
    size: f32,
    frac: f32,
    color: [f32; 4],
}
impl Particle {
    pub fn new(x: f32, y: f32, s: f32, f: f32, r: f32, g: f32, b: f32, a: f32) -> Self {
        Particle {
            position: vec2(x, y),
            size: s,
            frac: f,
            color: [r, g, b, a],
        }
    }
}

//the all mighty event processing
fn window_event(app: &App, model: &mut Model, event: WindowEvent) {
    match event {
        KeyPressed(_key) => {
            println!("{}", app.elapsed_frames());
        }
        KeyReleased(_key) => {}
        MouseMoved(_pos) => {}
        MousePressed(_button) => {}
        MouseReleased(_button) => {}
        MouseEntered => {}
        MouseExited => {}
        MouseWheel(_amount, _phase) => {}
        Moved(_pos) => {}
        Resized(_size) => {}
        Touch(_touch) => {}
        TouchPressure(_pressure) => {}
        HoveredFile(_path) => {}
        DroppedFile(_path) => {}
        HoveredFileCancelled => {}
        Focused => {}
        Unfocused => {}
        Closed => {}
        ReceivedCharacter(_c) => {}
    }
}
fn main() {
    nannou::app(model).update(update).run();
}

pub fn shader_from_spirv(device: &wgpu::Device, path: &Path) -> wgpu::ShaderModule {
    //load them so that we are not depending on shaderc
    //I use this to actually do separate the shader spirv files from this source
    //no need to recompile this whole project when a shader is updated
    let data = std::fs::read(path).unwrap();
    shader_from_spirv_bytes(&device, &data)
}
fn model(app: &App) -> Model {
    let window_id = app
        .new_window()
        //.fullscreen()
        .size(1280, 720)
        //I'll probably figure out msaa eventually, but not today,
        //not today...
        .msaa_samples(1)
        .view(view)
        .event(window_event)
        .build()
        .unwrap();

    let win = app.window_rect();

    let r = win.h() * 0.60;

    let win_rect = app.main_window().rect();
    //let step_length = 5.0;

    let dims = app.window_rect();
    let w = dims.w();
    let h = dims.h();

    //WGPU
    let window = app.window(window_id).unwrap();
    let device = window.device();
    let format = Frame::TEXTURE_FORMAT;
    let sample_count = window.msaa_samples();

    //create the basic circle
    let mut circle_vertices = Vec::new();
    let center = Vertex {
        position: [0.0, 0.0],
    };
    circle_vertices.push(center);
    let mut circle_indices = Vec::new();
    let ni = 120;
    for i in 0..ni {
        let fi = i as f32 / ni as f32;
        let angle = fi * TAU + TAU * 0.125;
        let v = Vertex {
            position: [angle.cos(), angle.sin()],
        };
        circle_vertices.push(v);
        circle_indices.push(0);
        circle_indices.push(i + 1);
        if i != ni - 1 {
            circle_indices.push(i + 2);
        } else {
            circle_indices.push(1);
        }
    }
    let n_circle_indices = (ni * 3) as u32; //180;

    let circle_vertices_bytes = vertices_as_bytes(&circle_vertices);
    let circle_indices_bytes = indices_as_bytes(&circle_indices);
    let vertex_usage = wgpu::BufferUsages::VERTEX;
    let index_usage = wgpu::BufferUsages::INDEX;
    let circle_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: circle_vertices_bytes,
        usage: vertex_usage,
    });
    let circle_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: circle_indices_bytes,
        usage: index_usage,
    });

    // Create the vertex buffer.
    let vertices_bytes = vertices_as_bytes(&VERTICES[..]);
    let usage = wgpu::BufferUsages::VERTEX;
    let rect_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: vertices_bytes,
        usage,
    });

    // Create the uniform buffer.
    let uniforms = Uniforms {
        dims: [w, h, 0.0, 0.0],
    };
    let uniforms_bytes = uniforms_as_bytes(&uniforms);
    let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;

    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: uniforms_bytes,
        usage,
    });

    //we want a layout where we have
    //a uniform buffer shown in the vertex shader
    let bind_group_layout = wgpu::BindGroupLayoutBuilder::new()
        .uniform_buffer(wgpu::ShaderStages::VERTEX, false)
        .build(device);
    //we create a bind group that specifies that specific buffer
    //(we will then have to copy to that buffer for things to work)
    let bind_group = create_bind_group(device, &bind_group_layout, &uniform_buffer);
    //we create a pipeline layout that works with this bind group layout
    let pipeline_layout = create_pipeline_layout(device, &bind_group_layout);
    //we are using instance rendering here!
    let vs_mod_instance =
        shader_from_spirv(&device, Path::new("src/shaders/basic_instance.vert.spv"));
    let fs_mod_instance =
        shader_from_spirv(&device, Path::new("src/shaders/basic_instance.frag.spv"));
    //we make the instance pipeline here
    let render_pipeline = create_render_pipeline_instance(
        device,
        &pipeline_layout,
        &vs_mod_instance,
        &fs_mod_instance,
        format,
        sample_count,
    );

    //here we'll make a bunch of rendering targets
    let texture_size = [w as u32, h as u32];

    let msaa_samples = window.msaa_samples();
    //for basic draw: we'll store our result here
    let draw_texture = create_render_texture(
        device,
        texture_size,
        wgpu::TextureFormat::Rgba16Float,
        msaa_samples,
    );
    let draw_texture_view = draw_texture.view().build();
    //for blur, let's create a backing texture
    let texture_a = create_render_texture(
        device,
        texture_size,
        wgpu::TextureFormat::Rgba16Float,
        msaa_samples,
    );
    let texture_a_view = texture_a.view().build();

    let texture_sample_type = draw_texture.sample_type();

    // Create our `Draw` instance and a renderer for it.
    let draw = nannou::Draw::new();
    let descriptor = draw_texture.descriptor();
    let draw_renderer =
        nannou::draw::RendererBuilder::new().build_from_texture_descriptor(device, descriptor);

    //how to render that texture

    //the shaders are very simple here
    let vs_mod_texture = shader_from_spirv(&device, Path::new("src/shaders/tex-simple.vert.spv"));
    let fs_mod_texture = shader_from_spirv(&device, Path::new("src/shaders/tex-simple.frag.spv"));

    let sampler_desc = wgpu::SamplerBuilder::new().into_descriptor();
    let sampler_filtering = wgpu::sampler_filtering(&sampler_desc);
    let sampler = device.create_sampler(&sampler_desc);
    //how we what to get the texture: a 2D texture NOT multi-sampled
    //seen in the frag shader
    let bind_group_layout_texture = wgpu::BindGroupLayoutBuilder::new()
        .texture(
            wgpu::ShaderStages::FRAGMENT,
            false, //multi-sample
            wgpu::TextureViewDimension::D2,
            texture_sample_type,
        )
        .sampler(wgpu::ShaderStages::FRAGMENT, sampler_filtering)
        .build(device);

    //for blur, we want a similar layout, but we add an uniform buffer - the bluruniform
    let bind_group_layout_texture_blur = wgpu::BindGroupLayoutBuilder::new()
        .texture(
            wgpu::ShaderStages::FRAGMENT,
            false, //multi-sample
            wgpu::TextureViewDimension::D2,
            texture_sample_type,
        )
        .sampler(wgpu::ShaderStages::FRAGMENT, sampler_filtering)
        .uniform_buffer(wgpu::ShaderStages::FRAGMENT, false)
        .build(device);

    let desc_texture = wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline for rendering a texture"),
        bind_group_layouts: &[&bind_group_layout_texture],
        push_constant_ranges: &[],
    };

    let desc_texture_blur = wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline for blurring a texture"),
        bind_group_layouts: &[&bind_group_layout_texture_blur],
        push_constant_ranges: &[],
    };
    let blur_uniforms = BlurUniforms {
        dims: [1.0 / 884.0, 0.0, 0.0, 0.0],
        blur: 1.0,
    };
    let blur_uniforms_bytes = blur_uniforms_as_bytes(&blur_uniforms);
    let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
    let blur_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: blur_uniforms_bytes,
        usage,
    });

    let bind_group_texture = wgpu::BindGroupBuilder::new()
        .texture_view(&draw_texture_view)
        .sampler(&sampler)
        .build(device, &bind_group_layout_texture);

    let bind_group_texture_a = wgpu::BindGroupBuilder::new()
        .texture_view(&texture_a_view)
        .sampler(&sampler)
        .build(device, &bind_group_layout_texture);

    let bind_group_texture_blur_a = wgpu::BindGroupBuilder::new()
        .texture_view(&texture_a_view)
        .sampler(&sampler)
        .buffer::<BlurUniforms>(&blur_uniform_buffer, 0..1)
        .build(device, &bind_group_layout_texture_blur);

    let bind_group_texture_blur_draw_texture = wgpu::BindGroupBuilder::new()
        .texture_view(&draw_texture_view)
        .sampler(&sampler)
        .buffer::<BlurUniforms>(&blur_uniform_buffer, 0..1)
        .build(device, &bind_group_layout_texture_blur);

    let pipeline_layout_texture = device.create_pipeline_layout(&desc_texture);
    //how we want to render that
    let render_pipeline_from_texture =
        wgpu::RenderPipelineBuilder::from_layout(&pipeline_layout_texture, &vs_mod_texture)
            .fragment_shader(&fs_mod_texture)
            .color_format(format)
            .color_blend(wgpu::BlendComponent::REPLACE)
            .alpha_blend(wgpu::BlendComponent::REPLACE)
            .add_vertex_buffer::<Vertex>(&wgpu::vertex_attr_array![0 => Float32x2])
            .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
            //.depth_format(depth_format)
            .sample_count(msaa_samples)
            .build(device);

    //the blur vertex is the same
    let vs_mod_texture_blur =
        shader_from_spirv(&device, Path::new("src/shaders/tex-simple.vert.spv"));
    let fs_mod_texture_blur =
        shader_from_spirv(&device, Path::new("src/shaders/tex-blur.frag.spv"));
    //the blur render pipeline is very similar, the only change is the addition
    //of a uniform buffer in the layout
    let pipeline_layout_blur = device.create_pipeline_layout(&desc_texture_blur);
    let render_pipeline_blur =
        wgpu::RenderPipelineBuilder::from_layout(&pipeline_layout_blur, &vs_mod_texture_blur)
            .fragment_shader(&fs_mod_texture_blur)
            .color_format(format)
            .color_blend(wgpu::BlendComponent::REPLACE)
            .alpha_blend(wgpu::BlendComponent::REPLACE)
            .add_vertex_buffer::<Vertex>(&wgpu::vertex_attr_array![0 => Float32x2])
            .primitive_topology(wgpu::PrimitiveTopology::TriangleStrip)
            //.depth_format(depth_format)
            .sample_count(msaa_samples)
            .build(device);

    //make some stuff to draw as instances
    let mut points: Vec<Particle> = Vec::new();
    let mut rng = XorShiftRng::seed_from_u64(SEED);
    let ni = 10000;
    for _i in 0..ni {
        let r = (rng.gen::<f32>() * 0.5 + 0.475) * w * 0.5;
        let angle = rng.gen::<f32>() * TAU;

        let x = angle.cos() * r;
        let y = angle.sin() * r;
        let s = (2.0 + rng.gen::<f32>() * 19.0) * w / 1024.0;

        let frac = rng.gen::<f32>();
        let red = rng.gen::<f32>();
        let green = rng.gen::<f32>();
        let blue = rng.gen::<f32>();
        points.push(Particle::new(x, y, s, frac, red, green, blue, 0.2));
    }

    //store all our stuff
    let graphics = RefCell::new(Graphics {
        circle_vertex_buffer,
        circle_index_buffer,
        rect_vertex_buffer,
        uniform_buffer,
        blur_uniform_buffer,
        bind_group,
        render_pipeline,
        pipeline_layout,
        n_circle_indices,
        render_pipeline_from_texture,
        bind_group_texture,
        render_pipeline_blur,
        bind_group_texture_blur_a,
        bind_group_texture_blur_draw_texture,
        bind_group_texture_a,
        texture_a,
        texture_a_view,
        draw_texture_view,
    });

    Model {
        points,
        graphics,
        draw_texture,
        draw,
        draw_renderer,
        do_blur: true,
    }
}

fn update(app: &App, model: &mut Model, _update: Update) {
    let window = app.main_window();

    //using this as our 'internal clock' to make loops based on the frame rate
    //I'm not targetting real time capture
    let frac = ((app.elapsed_frames() % LENGTH_FRAME) as f32) / (LENGTH_FRAME as f32);

    //draw to our inner draw
    let draw = &model.draw;
    draw.reset();

    // Create a `Rect` for our texture to help with drawing.
    let [wu, hu] = model.draw_texture.size();
    let [w, h] = [wu as f32, hu as f32];

    // Draw like we normally would in the `view`.
    if app.elapsed_frames() <= 1 {
        //clear the texture for the beginning - otherwise you get whatever
        //default there is
        draw.background().color(BLACK);
    } else {
        // Important: if you use background, the contents of the texture will
        // be *REPLACED* by the background color
        //draw.background().color(srgba(0.0,0.0,0.0,0.1)); //NOT WORKING

        //if we want some fade out we need to draw an almost transparent rect
        let c: Srgba = srgba(0.0, 0.0, 0.0, 0.001);
        draw.rect().w_h(w, h).x_y(0.0, 0.0).color(c);
    }

    //draw some boring stuff
    let radius = w.min(h) * 0.2;
    let angle = frac * TAU;
    let c: Srgba = srgba(angle.cos() * 0.5 + 0.5, 0.0, 0.0, 1.0);
    draw.ellipse()
        .radius(radius * (angle.sin() * 0.2 + 1.0))
        .x_y(radius * angle.cos(), radius * angle.sin())
        .color(c);
    let c: Srgba = srgba(0.0, angle.sin() * 0.5 + 0.5, 0.0, 1.0);
    draw.ellipse()
        .radius(0.8 * radius * (angle.sin() * 0.2 + 1.0))
        .x_y(radius * angle.cos(), radius * angle.sin())
        .color(c);

    // to "perform" the draw we need to manually ask for rendering to happen
    // we do this by submitting the job to the device
    let window = app.main_window();
    let device = window.device();
    let ce_desc = wgpu::CommandEncoderDescriptor {
        label: Some("texture renderer"),
    };

    // the draw_renderer is doing all the heavy work of rasterizing and all
    let mut encoder = device.create_command_encoder(&ce_desc);
    model
        .draw_renderer
        .render_to_texture(device, &mut encoder, &draw, &model.draw_texture);
    window.queue().submit(Some(encoder.finish()));
    // at this point, the draw_texture contains the result of the draw
    // we're done here, see you in view for the next steps
}

fn view(app: &App, model: &Model, frame: Frame) {
    //we don't even use draw here!
    //let draw = app.draw();
    let device = frame.device_queue_pair().device();

    let dims = app.window_rect();
    let w = dims.w();
    let h = dims.h();

    //here we go
    let g = model.graphics.borrow_mut();
    //what to do when we render:
    //load takes the existing color
    let mut what = LoadOp::Load;
    //a clear will set the color to that color
    if app.elapsed_frames() == 1 {
        what = LoadOp::Clear(Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        });
    }

    //let's blur the contents of our draw_texture
    //you could do that at a different point in time
    if model.do_blur {
        let blur_strength = 1.0;
        //we do a two-pass blur
        {
            //first blur is horizontal
            //so we want to blur along this direction
            //we do this by setting the uniforms
            let blur_uniforms = BlurUniforms {
                dims: [blur_strength * 1.0 / w, 0.0, 0.0, 0.0],
                blur: 1.0,
            };

            let uniforms_size = std::mem::size_of::<BlurUniforms>() as wgpu::BufferAddress;
            let blur_uniforms_bytes = blur_uniforms_as_bytes(&blur_uniforms);
            let usage = wgpu::BufferUsages::COPY_SRC;
            let new_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: blur_uniforms_bytes,
                usage,
            });

            //then we push that buffer to the correct spot on the gpu
            let mut encoder = frame.command_encoder();
            encoder.copy_buffer_to_buffer(
                &new_uniform_buffer,
                0,
                &g.blur_uniform_buffer,
                0,
                uniforms_size,
            );

            //we put the result of the blur pass into target_a
            let mut render_pass = wgpu::RenderPassBuilder::new()
                .color_attachment(&g.texture_a_view, |color| color.load_op(what))
                .begin(&mut encoder);
            //we take the draw_texture as the texture to be sampled (blurred)
            render_pass.set_bind_group(0, &g.bind_group_texture_blur_draw_texture, &[]);
            render_pass.set_pipeline(&g.render_pipeline_blur);
            //and we render a simple quad to work on the whole surface
            render_pass.set_vertex_buffer(0, g.rect_vertex_buffer.slice(..));
            let vertex_range = 0..VERTICES.len() as u32;
            let instance_range = 0..1;
            render_pass.draw(vertex_range, instance_range);
        }

        {
            //second blur is the same, but on the vertical axis
            let blur_uniforms = BlurUniforms {
                dims: [0.0, blur_strength * 1.0 / h, 0.0, 0.0],
                blur: 1.0,
            };
            let uniforms_size = std::mem::size_of::<BlurUniforms>() as wgpu::BufferAddress;
            let blur_uniforms_bytes = blur_uniforms_as_bytes(&blur_uniforms);
            let usage = wgpu::BufferUsages::COPY_SRC;
            let new_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: blur_uniforms_bytes,
                usage,
            });

            let mut encoder = frame.command_encoder();
            encoder.copy_buffer_to_buffer(
                &new_uniform_buffer,
                0,
                &g.blur_uniform_buffer,
                0,
                uniforms_size,
            );

            //we put the result into our draw_texture this time though!
            let mut render_pass = wgpu::RenderPassBuilder::new()
                .color_attachment(&g.draw_texture_view, |color| color.load_op(what))
                .begin(&mut encoder);
            // and we blur from texture_a - what we just used to store the
            // result of the first blur
            render_pass.set_bind_group(0, &g.bind_group_texture_blur_a, &[]);
            render_pass.set_pipeline(&g.render_pipeline_blur);
            render_pass.set_vertex_buffer(0, g.rect_vertex_buffer.slice(..));
            let vertex_range = 0..VERTICES.len() as u32;
            let instance_range = 0..1;
            render_pass.draw(vertex_range, instance_range);
        }
    }
    //at this point our draw_texture contains the blurred version of draw
    //(and the previous frame)

    // INSTANCE RENDERING
    {
        let mut encoder = frame.command_encoder();
        let what = LoadOp::Load;

        //make a uniform buffer with our current frame size
        let uniforms = Uniforms {
            dims: [w, h, 0.0, 0.0],
        };

        let uniforms_bytes = uniforms_as_bytes(&uniforms);
        let uniforms_size = std::mem::size_of::<Uniforms>() as wgpu::BufferAddress;
        let usage = wgpu::BufferUsages::COPY_SRC;

        let new_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: uniforms_bytes,
            usage,
        });

        //copy that buffer to the right spot
        encoder.copy_buffer_to_buffer(&new_uniform_buffer, 0, &g.uniform_buffer, 0, uniforms_size);

        //the instance rendering
        //first, make a Vec of all the data
        let mut instances: Vec<Instance> = Vec::new();
        for point in model.points.iter() {
            instances.push(Instance {
                data: [
                    point.position.x,
                    point.position.y,
                    2.0 * point.size / h,
                    point.frac,
                ],
                data2: point.color,
            });
        }

        //then we make a buffer out of that
        let instances_bytes = instances_as_bytes(&instances);
        let usage = wgpu::BufferUsages::VERTEX;
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: instances_bytes,
            usage,
        });
        //we are not copying that buffer - we don't need to have that buffer
        //in a bind_group so we can keep it here

        //start our render pass
        //targeting our render texture

        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(&g.draw_texture_view, |color| color.load_op(what))
            .begin(&mut encoder);

        //render the instances
        render_pass.set_bind_group(0, &g.bind_group, &[]);
        render_pass.set_pipeline(&g.render_pipeline);
        //we want to render circles
        render_pass.set_vertex_buffer(0, g.circle_vertex_buffer.slice(..));
        render_pass.set_index_buffer(g.circle_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        let index_range = 0..g.n_circle_indices as u32;
        let start_vertex = 0;
        //with these instance data
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        let instance_range = 0..instances.len() as u32;
        render_pass.draw_indexed(index_range, start_vertex, instance_range);
    }

    //THROUGH PASS TO NANNOU'S INTERNAL TEXTURE
    {
        let mut encoder = frame.command_encoder();
        let what = LoadOp::Load;
        //this time we render nannou's internal target
        let mut render_pass = wgpu::RenderPassBuilder::new()
            .color_attachment(frame.texture_view(), |color| color.load_op(what))
            .begin(&mut encoder);

        //draw our texture updated as a rect
        render_pass.set_bind_group(0, &g.bind_group_texture, &[]);
        render_pass.set_pipeline(&g.render_pipeline_from_texture);
        render_pass.set_vertex_buffer(0, g.rect_vertex_buffer.slice(..));
        let vertex_range = 0..VERTICES.len() as u32;
        let instance_range = 0..1;
        render_pass.draw(vertex_range, instance_range);
    }
}

//HELPER FUNCTIONS
fn uniforms_as_bytes(uniforms: &Uniforms) -> &[u8] {
    unsafe { wgpu::bytes::from(uniforms) }
}
fn blur_uniforms_as_bytes(blur_uniforms: &BlurUniforms) -> &[u8] {
    unsafe { wgpu::bytes::from(blur_uniforms) }
}
fn vertices_as_bytes(data: &[Vertex]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}
fn indices_as_bytes(data: &[u16]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}
fn instances_as_bytes(data: &[Instance]) -> &[u8] {
    unsafe { wgpu::bytes::from_slice(data) }
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    wgpu::BindGroupLayoutBuilder::new()
        .uniform_buffer(wgpu::ShaderStages::VERTEX, false)
        .build(device)
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    wgpu::BindGroupBuilder::new()
        .buffer::<Uniforms>(uniform_buffer, 0..1)
        .build(device, layout)
}

fn create_pipeline_layout(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    let desc = wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    };
    device.create_pipeline_layout(&desc)
}
fn create_render_pipeline_instance(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vs_mod: &wgpu::ShaderModule,
    fs_mod: &wgpu::ShaderModule,
    dst_format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    /*
    let add_color_blend = wgpu::BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                };

    let add_alpha_blend = wgpu::BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                };
    */
    let default_color_blend = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };

    let default_alpha_blend = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };

    //our render pipeline takes 2D vertices
    wgpu::RenderPipelineBuilder::from_layout(layout, vs_mod)
        .fragment_shader(&fs_mod)
        .color_format(dst_format)
        //.color_blend(wgpu::BlendComponent::REPLACE)
        //.alpha_blend(wgpu::BlendComponent::REPLACE)
        .color_blend(default_color_blend)
        .alpha_blend(default_alpha_blend)
        //.color_blend(add_color_blend)
        //.alpha_blend(add_alpha_blend)
        // we work with our Vertex type that is 2d
        .add_vertex_buffer::<Vertex>(&wgpu::vertex_attr_array![0 => Float32x2])
        // TODO: this can use the macro again when https://github.com/gfx-rs/wgpu/issues/836 is fixed
        // this is how we describe what our shader expects
        // here we want two vec4 per instance
        .add_instance_buffer::<Instance>(&[
            wgpu::VertexAttribute {
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as u64 * 0,
            },
            wgpu::VertexAttribute {
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
                offset: std::mem::size_of::<[f32; 4]>() as u64 * 1,
            },
        ])
        .sample_count(sample_count)
        .build(device)
}

fn create_render_texture(
    device: &wgpu::Device,
    size: [u32; 2],
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> wgpu::Texture {
    wgpu::TextureBuilder::new()
        .size(size)
        .format(format)
        // we want the texture to be able to be rendered to
        // and used in a sampler (that's the texture binding one)
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
        .sample_count(sample_count)
        .build(device)
}
