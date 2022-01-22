// Copyright 2018 The Synthesizer IO Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A testbed for experimenting with scope display.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use png::HasParameters;

use scope::Scope;

fn main() {
    let path = Path::new("foo.png");
    let f = File::create(path).unwrap();
    let w = BufWriter::new(f);
    let mut encoder = png::Encoder::new(w, 640, 480);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    /*
    let z = 2.0 / ::std::f32::consts::PI.sqrt();
    let img = mk_uvmap_img(|u, v|
        gauss_approx(v * 5.0) * 0.5 * (erf_approx(u * 5.0 * z) - erf_approx((u - 0.5) * 5.0 * z))
    );
    */
    let mut scope = Scope::new(640, 480);
    let r = 1.0;
    let start = ::std::time::Instant::now();
    let mut xylast = None;
    // sinewave!
    for i in 0..1001 {
        let h = (i as f32) * 0.001;
        let x = 640.0 * h;
        let y = 240.0 + 200.0 * (h * 50.0).sin();
        if let Some((xlast, ylast)) = xylast {
            scope.add_line(xlast, ylast, x, y, r, 2.0);
        }
        xylast = Some((x, y));
    }
    println!("elapsed: {:?}", start.elapsed());
    let img = scope.as_rgba();
    println!("elapsed after rgba: {:?}", start.elapsed());
    writer.write_image_data(&img).unwrap();
}
