use nannou::prelude::Frame;
use nannou::prelude::*;

use nannou_audio as audio;
use nannou_audio::Buffer;
use ringbuf::{Consumer, Producer, RingBuffer};
use rustfft::{num_complex::Complex, Fft, FftPlanner};

use std::sync::mpsc;
use std::sync::Arc;

struct AudioData {
    f: [f32; 512],
}

struct Audio {
    fft: Arc<dyn Fft<f32>>,
    prod: Producer<f32>,
    cons: Consumer<f32>,
    window: [f32; 1024],
    tx: mpsc::Sender<AudioData>,
}

struct Model {
    _stream: audio::Stream<Audio>,
    receiver_audio_data: Option<mpsc::Receiver<AudioData>>,
    spectrum: [f32; 512],
}

fn main() {
    nannou::app(model).update(update).run();
}
fn model(app: &App) -> Model {
    let _window_id = app
        .new_window()
        .size(1280, 720)
        .view(view)
        .build()
        .unwrap();

    //audio init
    // Initialise the audio host so we can spawn an audio stream.
    let audio_host = audio::Host::new();

    // Initialise the fft
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(1024);

    // Create a ring buffer and split it into producer and consumer
    let ring_buffer_samples = 1024;
    let ring_buffer = RingBuffer::<f32>::new(ring_buffer_samples * 2); // Add some latency
    let (mut prod, cons) = ring_buffer.split();
    for _ in 0..ring_buffer_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        prod.push(0.0).unwrap();
    }

    let mut hann_window = [0.0; 1024];
    for k in 0..512 {
        let v = 0.5 * (1.0 - (k as f32 / 1024.0 * TAU).cos());
        hann_window[k] = v;
        hann_window[1023 - k] = v;
    }

    //create a channel to send info back to the main thread
    let (sender_audio_data, rad) = mpsc::channel::<AudioData>();

    let audio_model = Audio {
        fft,
        prod,
        cons,
        window: hann_window,
        tx: sender_audio_data,
    };

    //start the thing
    let stream = audio_host
        .new_input_stream(audio_model)
        .capture(capture_audio)
        .build()
        .unwrap();

    stream.play().unwrap();

    Model {
        _stream: stream,
        receiver_audio_data: Some(rad),
        spectrum: [0.0; 512],
    }
}

fn capture_audio(audio: &mut Audio, buffer: &Buffer) {
    //push the contents to a ring buffer
    //when we have enough samples, make fft

    for frame in buffer.frames() {
        //we only take the first channel
        //by taking the first sample in the frame
        audio.prod.push(frame[0]).unwrap();
    }

    //can we do the fft now?
    if audio.cons.len() > audio.fft.len() {
        let mut fft_buffer = Vec::new();
        for i in 0..audio.fft.len() {
            let f = audio.cons.pop().unwrap();
            fft_buffer.push(Complex {
                re: f * audio.window[i],
                im: 0.0,
            });
        }
        //put the whole buffer
        audio.fft.process(&mut fft_buffer);
        let mut f = [0.0; 512];
        for i in 0..512 {
            f[i] = fft_buffer[i].norm().sqrt();
        }

        audio.tx.send(AudioData { f }).unwrap();
    }
}
fn update(_app: &App, model: &mut Model, _update: Update) {
    //receive data from the audio stream
    if let Some(rx) = &model.receiver_audio_data {
        while let Ok(msg) = rx.try_recv() {
            for k in 0..msg.f.len() {
                //add a bit of damping
                model.spectrum[k] = msg.f[k].max(model.spectrum[k] * 0.8 + msg.f[k] * 0.2);
            }
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    let rect = app.window_rect();
    let w = rect.w();
    let h = rect.h();

    draw.background().color(BLACK);
    let nk = model.spectrum.len();
    let w_rect = w / nk as f32;
    for k in 0..nk {
        let x = ((k as f32 + 0.5) / nk as f32 - 0.5) * w;
        draw.rect()
            .w_h(w_rect, h * model.spectrum[k])
            .x_y(x, 0.0)
            .color(WHITE);
    }
    draw.to_frame(app, &frame).unwrap();
}
