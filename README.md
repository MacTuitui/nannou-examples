# nannou examples

A collection of examples that might use some non-beginner things.

## Advance wgpu

This example combines nannou's standard `draw` API on top
of instance rendering and a two-pass full screen shader pass (for effects, here
blur).

I tried my best to comment the why and how this work, but this can get 
pretty confusing if you don't really follow how WebGPU works 
(and nannou does a great job at keeping it hidden yet available!).

Run with
```
cargo run --release --example advance-wgpu
```

## FFT

Your old spectrum visualizer. The only trick is to use a channel to get information 
from the audio thread.

Run with
```
cargo run --release --example fft
```
