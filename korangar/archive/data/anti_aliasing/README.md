# SMAA lookup textures

These two files are the unmodified lookup textures of the SMAA (Subpixel
Morphological Antialiasing) reference implementation by Jorge Jimenez, Jose I.
Echevarria, Belen Masia, Fernando Navarro and Diego Gutierrez:

* `smaa_area_tex.bin`: AreaTex, 160x560 pixels, R8G8_UNORM, extracted from
  `Textures/AreaTex.h` of <https://github.com/iryoku/smaa>.
* `smaa_search_tex.bin`: SearchTex, 64x16 pixels, R8_UNORM, extracted from
  `Textures/SearchTex.h` of <https://github.com/iryoku/smaa>.

The data is used by `korangar/src/graphics/smaa.rs` to build the lookup
textures for the SMAA 1x post-processing passes. The reference implementation
is licensed under an MIT style license:

```
Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software. As clarification, there
is no requirement that the copyright notice and permission be included in
binary distributions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
