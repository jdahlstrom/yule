#![no_std]

extern crate alloc;
extern crate core;

mod utils;

use wasm_bindgen::prelude::*;

use alloc::vec::Vec;
use core::iter::zip;
use core::ops::ControlFlow::Continue;

use re::prelude::*;

use re::math::rand::Distrib;
use re::prelude::tex::{Atlas, Layout, SamplerClamp};
use re::render::render;
use re::util::pnm::parse_pnm;
use re_front::dims::QVGA_1280_960;
use re_front::wasm::Window;

#[wasm_bindgen(start)]
fn main() {
    let res = (1280, 720);
    let mut win = Window::new(res).unwrap();

    let tex = *include_bytes!("../../assets/flakes.pgm");

    let tex = parse_pnm(tex).unwrap();

    let font = *include_bytes!("../../assets/font_16x24.pbm");
    let font = parse_pnm(font).unwrap();
    let (cw, ch) = (font.width() / 16, font.height() / 16);
    let font = Atlas::new(Layout::Grid(cw, ch), font);

    let text = b"\x0F Iloista joulua! \x0F";

    let mut buf = Buf2::new((cw * text.len() as u32, ch));
    for (&c, x) in zip(text, 0..) {
        buf.slice_mut(((x * cw)..(x + 1) * cw, 0..ch))
            .copy_from(*font.get(c as u32).data());
    }
    let text = buf.into();

    let tris = [Tri([0, 1, 2]), Tri([0, 2, 3])];
    let verts = [
        (pt3(0.0, 0.0, 0.0), uv(0.0, 0.0)),
        (pt3(1.0, 0.0, 0.0), uv(1.0, 0.0)),
        (pt3(1.0, 1.0, 0.0), uv(1.0, 1.0)),
        (pt3(0.0, 1.0, 0.0), uv(0.0, 1.0)),
    ]
    .map(|(pos, uv): (Point3<Model>, _)| vertex(pos - vec3(0.5, 0.5, 0.0), uv));

    let proj =
        perspective(1.0, win.dims.0 as f32 / win.dims.1 as f32, 0.1..1000.0);
    let to_screen = viewport(pt2(0, 0)..pt2(res.0, res.1));

    let mut rng = rand::DefaultRng::default();

    const N: usize = 300;

    let mut vecs = rand::Uniform(splat(-1.0)..splat(1.0));
    let ps: Vec<Vec3> = vecs.samples(&mut rng).take(N).collect();
    let vs: Vec<Vec3> = vecs.samples(&mut rng).take(N).collect();

    win.ctx.color_clear = Some(rgba(32, 8, 8, 255));

    win.run(move |frame| {
        frame.ctx.face_cull = None;

        let flakes = [
            // Row 0
            // Row 1
            (10..19, 8..18),
            (30..40, 8..18),
            (40..49, 8..18),
            (50..59, 8..18),
            // Row 2
            (0..12, 18..29),
            (12..24, 18..29),
            // Row 3
            (0..11, 30..44),
            (12..25, 30..43),
            (26..39, 30..43),
            (40..51, 30..43),
        ]
        .map(|c| Texture::from(tex.slice(c)));

        let t = frame.t.as_secs_f32();

        for i in 0..N {
            let rx = ps[i].x();
            let ry = ps[i].y();

            let rvx = vs[i].x() * 0.5;
            let rvy = vs[i].y() * 0.4 + 0.8;

            let y = (ry * 3.0 + rvy * t).rem_euclid(5.0) - 2.5;
            let x = (rx * 4.0 + rvx * t).rem_euclid(8.0) - 4.0;
            let a = rads((t + rvx).sin()) * 0.25;

            let flake_shader = Shader::new(
                |v: Vertex<Point3<_>, TexCoord>,
                 tf: Mat4x4<mat::RealToProj<_>>| {
                    vertex(tf.apply(&v.pos), v.attrib)
                },
                |f: Frag<TexCoord>| {
                    let c =
                        SamplerClamp.sample(&flakes[i % flakes.len()], f.var);
                    (c.r() > 0).then(|| c.to_rgba())
                },
            );

            let tf = scale(splat(0.08))
                .then(&rotate_z(a + turns(rvx)))
                .then(&rotate_x(a))
                .then(&translate(vec3(x, y, 2.0 + ps[i].z() * 1.0)))
                .to()
                .then(&proj);

            render(
                tris,
                verts,
                &flake_shader,
                tf,
                to_screen,
                &mut frame.buf,
                frame.ctx,
            );
        }

        let text_shader = Shader::new(
            |v: Vertex<_, _>, mvp: &Mat4x4<ModelToProj>| {
                vertex(mvp.apply(&v.pos), v.attrib)
            },
            |frag: Frag<TexCoord>| {
                Some(SamplerClamp.sample(&text, frag.var).to_rgba())
                    .filter(|c| c.r() > 0)
                    .map(|_| rgba(0x33, 0x44, 0xCC, 0xFF))
            },
        );
        let secs = frame.t.as_secs_f32();
        let mvp = scale(vec3(1.0, 1.0 / 10.0, 1.0))
            .then(&translate(vec3(0.0, 0.0, 0.8 * secs.sin())))
            .then(&rotate_y(rads((secs * 0.59).sin())))
            .then(&rotate_z(rads((secs * 1.13).sin())))
            .then(&translate(vec3(0.0, 0.0, 1.2)))
            .to()
            .then(&proj);

        render(
            tris,
            verts,
            &text_shader,
            &mvp,
            to_screen,
            &mut frame.buf,
            &frame.ctx,
        );

        Continue(())
    });
}
