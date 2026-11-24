# Phase 2 gradient probe

**Recommendation, 2026-11-24.** Proceed with direct interpolation and
per-pixel placement for the first renderer slice. Use row reuse on the
CPU for axis-aligned linear gradients. Keep the existing solid pipelines.
The measurements support this choice for title surfaces and meter strips.
For hard conic stops, carry a prepared ray and test its side. Extending that
classification to arbitrary stop lists and Repeat/Reflect boundaries must
pass parity tests before the complete paint API ships.

The [specification](../gradient-spec.md) describes the proposed library API.
This experiment is a standalone crate outside the workspace. It draws to an
sRGB attachment with hardware blending on wgpu/Vulkan and reads actual
RGBA8 pixels back. The scalar Rust implementation composites over the same
background and encodes to bytes. Neither renderer is installed in the facade.

## The 40-pixel title transition

A 320-logical-pixel title was drawn with the gradient anchored from 0 to 40.
We checked all channels against the CPU reference at display scales 1, 1.25
and 2. The transition occupied 40, 50 and 80 physical pixels respectively.
Every pixel beyond it held the end color. Scissoring the overlay left the
background visible there. Doubling the surface width preserved the first
row's colors across the original width. Shrinking the title below 40
preserved the prefix of the original ramp. These are offscreen paints with
the title's geometry; the library's floating-window component has yet to
adopt them.

## Fidelity and numerical limits

- The regular corpus covers three geometries and interpolation spaces,
  repeated negative phases, reflected phases, concentric inner radius,
  partial sweep, alpha-only and near-transparent stops, a solid border
  composited over a gradient fill, clipped rectangles, fractional position
  and zoom. A physical-frame-relative mapping after f64 subtraction at a
  logical origin of 2³⁵ reproduced the capture at the origin. Worst
  CPU f32 versus f64 and CPU versus GPU differences were one byte/channel.
  The f64 oracle starts with the same prepared f32 stops, so this measures
  evaluation and compositing precision after stop preparation.
  [Pixel results, including worst locations](results/validation.log)
- A direct GPU projection originally put one exact linear Repeat wrap on
  the wrong side, with a 145-byte green-channel difference. Computing the
  dot product relative to the nearest period origin removed it in the
  regression case. For full-turn sweeps with a jump halfway around, ordinary
  atan2 flipped 281 samples, including 257 ties. Testing the signed side of
  a prepared ray scaled by its dominant component settled every tie and
  left one opposite-side sample, 0.000004595 physical pixels from the
  jump. The sample was one of the two permitted side colors. Merely
  normalizing the ray to unit length had still flipped exact diagonal ties.
- For this last corpus the largest signed-ray error corresponds to
  0.0000000012 turns. This is an observation on radii up to roughly
  47,000 frame pixels and selected angles and phases. For the next slice
  prepare coefficients relative to the clipped span in f64, check finite
  narrowing and representability, then classify ambiguous samples within
  a conservative half-physical-pixel maximum. Reject mappings whose
  uncertainty overlaps multiple jumps. The sampled maximum cannot set a
  universal t epsilon across tiny radii, extreme zoom and arbitrary repeat
  counts. The signed-ray experiment isolates the binary decision; the
  general shader still uses atan2. The full implementation must connect
  the classifier to each relevant discontinuity.
- With a two-stop Oklab black-to-white ramp, 24 adaptive linear segments
  brought the sampled pre-dither error to 0.490 encoded byte. With 16
  alternating black/white stops the 64-segment cap still gave **15.38**
  bytes. The comparison sampled 65,537 positions and compared premultiplied
  results over white and black and the alpha channel. Even passing that
  grid is insufficient to guarantee an approximation bound. Direct
  interpolation admitted both inputs and was faster on this GPU on the
  two-stop workload.

## Dither and opacity

On the 32→49 encoded-gray ramp the GPU's 8×8 tile-mean RMS error fell from
0.26067 to 0.05440 encoded bytes, or 21% of the undithered error. The
proposed gate was 50%. Repeated captures matched byte for byte. At 1:1
the dark bands were less distinct; the ordered pattern introduces a fine
regular texture. The top strip is undithered, the bottom uses Bayer.

![Undithered and dithered GPU captures](results/banding.png)

The continuous sRGB functions and a shared 4,097-entry linearly interpolated
transfer table both met the byte tolerance in the tested cases. The table
cost more GPU time on the panel and bars. Dither perturbs source RGB only.
For a black scrim it changed some RGB roundings on dark and white
backgrounds; alpha precision remains eight bits. A flat alpha ramp has no
extra alpha levels. Keep the opaque banding gate distinct from opacity.

## Cost

Fedora 45, Core i9-12900, Intel UHD Graphics 770, Mesa 26.2.7, Vulkan,
release build, task pinned to CPU 2. Entries are milliseconds, the median
of three run medians. Each run measures eleven groups. GPU columns are
elapsed time per submitted offscreen pass in groups of eight, including
host encoding, queue submission and completion. Readback is outside this
timer. CPU columns time scalar evaluation and RGBA8 writes into a reused
buffer. The CPU image reference filters the 256-texel strip in linear
light. CPU row reuse was checked against the scalar output byte for byte.
The numbers characterize the experiment on this machine; other work was
running and the raw ranges show scheduling variability.

| Paint and workload | GPU pass | CPU scalar | CPU row reuse |
|---|---:|---:|---:|
| Dark panel, 1024×64 | 0.138 | 4.61 | 0.073 |
| Dark panel, Bayer | 0.190 | 7.55 | 0.952 |
| Alpha strip, 320×24 | 0.082 | 0.36 | 0.015 |
| Alpha strip, Bayer | 0.129 | 0.74 | 0.314 |
| Phase 0 bars, direct | 0.313 | 15.41 | 0.517 |
| Phase 0 bars, Bayer | 0.411 | 25.94 | 0.796 |
| Same bars, stretched image | 0.226 | 20.38 | — |

The bar draws reuse the phase 0 positions, levels and colors and scissor
the lit portion against its full-height ramp. These times isolate the lit
draws on a cleared frame. Retained draws upload zero new paint bytes.
Moving clips plus rewriting placement took 0.326 ms with a 64-byte
uniform write per frame. Rewriting placement and the three stops took
0.339 ms with 160 bytes per frame. These are application writes to the
queue, excluding encoded commands, startup and driver traffic. The
stretched-image path remains faster on this GPU. On the CPU the measured
benefit depends on exploiting the repeated samples; generic per-pixel
evaluation alone is too expensive to justify migrating large surfaces.

[Run 1](results/run-1.log) · [Run 2](results/run-2.log) · [Run 3](results/run-3.log)

## Reproduction

From this directory, with the repository toolchain and a hardware Vulkan
adapter, run the following. Set PROBE_CAPTURES to an existing directory
to write both banding captures as PPM files. The executable reports the
adapter, per-case errors and locations, costs and the final gate status.
The angular diagnostic reports the ordinary shader's failures and checks
the prepared-ray candidate against the stated margin.

```sh
cargo fmt --check
cargo clippy --release --locked -- -D warnings
cargo build --release --locked
taskset -c 2 target/release/gradient-probe
```

To run the production solid-scene control from the repository root, use
the existing phase 1 harness in an unlocked graphical session:

```sh
PROBE_ONLY=mixer-diagram RUNS=3 plans/rendering/phase1-probe/run.sh target/solid-control reference=HEAD
```

The experiment changes no production command, solid record stride, shader
or draw path. Running the production harness twice measures its repeatability.
The production renderer must be measured again after integration to assess
the proposed 5% frame-time and unchanged-upload gates.

In the first six completed runs (cores 3 and 5) the odd/even groups'
median CPU times were 1.094/1.177 ms for the mixer and 2.063/2.001 ms for
diagram pan. The mixer's 7.6% difference between runs of the *same binary*
exceeds the proposed regression gate. A second series on cores 8–11
also varied, and the mixer tick timed out on attempted runs under load.
The control recorded zero new mask KiB for these workloads; its counters
do not cover every GPU write. Solid-path timing and total upload regression
therefore remain integration checks on a quiet machine. [Completed control runs](results/solid-control.tsv)
