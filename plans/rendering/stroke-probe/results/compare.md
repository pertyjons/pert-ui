# Stroke prototype: new coverage against today's

Today's CPU renderer against the reference, white strokes, coverage recovered from the green channel (8-bit sRGB, so ±1 step is quantization). Cases at command scale 1 without a clip; tile size 16.

| Shape | Pixels | ≤1 | ≤2 | ≤4 | ≤16 | ≤64 | >64 | Largest |
|---|---|---|---|---|---|---|---|---|
| zigzag | 673843 | 52.86 % | 2.37 % | 4.47 % | 17.28 % | 22.30 % | 0.71 % | 146 |
| wave | 1093153 | 55.59 % | 2.15 % | 3.62 % | 14.84 % | 22.75 % | 1.04 % | 127 |
| sharp turns | 730749 | 53.49 % | 2.32 % | 4.64 % | 16.93 % | 22.03 % | 0.58 % | 132 |
| closed | 600351 | 54.56 % | 2.02 % | 4.18 % | 16.79 % | 21.78 % | 0.67 % | 140 |
| reversal | 131423 | 70.75 % | 0.49 % | 0.58 % | 4.33 % | 11.43 % | 12.42 % | 140 |
| straight continuation | 283458 | 50.50 % | 2.65 % | 5.88 % | 20.12 % | 20.41 % | 0.43 % | 140 |
| short segments | 273874 | 51.36 % | 2.80 % | 5.31 % | 20.33 % | 19.85 % | 0.34 % | 143 |
| points | 26778 | 71.41 % | 1.21 % | 2.04 % | 7.90 % | 13.76 % | 3.69 % | 140 |
| far geometry | 308133 | 52.74 % | 2.24 % | 4.68 % | 19.56 % | 20.46 % | 0.31 % | 98 |
