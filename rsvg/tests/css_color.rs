//! Rendering tests for the colors of CSS Color 4, CSS Color 5 and CSS Color HDR.
//!
//! Each color below is #336699 written in another space or function (values from the specifications' sample
//! code); each renders as the plain hex color does.

use rsvg::test_compare_render_output;

macro_rules! same_as_hex {
    ($name:ident, $color:expr) => {
        test_compare_render_output!(
            $name,
            20,
            20,
            concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill=""##, $color, r##""/></svg>"##).as_bytes(),
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#336699"/></svg>"##,
        );
    };
}

same_as_hex!(color_srgb, "color(srgb 0.2 0.4 0.6)");
same_as_hex!(color_srgb_linear, "color(srgb-linear 0.03310476657088507 0.13286832155381798 0.31854677812509186)");
same_as_hex!(color_display_p3, "color(display-p3 0.24985133313829078 0.3952400722189161 0.5840337707927922)");
same_as_hex!(color_rec2020, "color(rec2020 0.34602728016089684 0.4247406336038397 0.6034416215886044)");
same_as_hex!(color_xyz, "color(xyz 0.11865530579242775 0.12505925609252708 0.31926610717393133)");
same_as_hex!(lab, "lab(41.52082392740827 -4.573089932636054 -33.494194592438944)");
same_as_hex!(oklch, "oklch(0.49931445584520834 0.09866437712418329 250.4330574201754)");
same_as_hex!(color_mix, "color-mix(in oklab, #336699 30%, #336699)");
same_as_hex!(relative_color, "oklch(from #336699 l c h)");

// CSS Color 4 section 4.5: an invalid color drops the declaration, so the fill keeps its initial value, black
test_compare_render_output!(
    invalid_color_space_is_dropped,
    20,
    20,
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="color(profoto-rgb 0.4835 0.9167 0.2188)"/></svg>"##,
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="black"/></svg>"##,
);

// gradient stops take the same colors
test_compare_render_output!(
    gradient_stop_in_display_p3,
    20,
    20,
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><linearGradient id="g"><stop offset="0" stop-color="color(display-p3 0.24985133313829078 0.3952400722189161 0.5840337707927922)"/><stop offset="1" stop-color="lab(41.52082392740827 -4.573089932636054 -33.494194592438944)"/></linearGradient><rect width="20" height="20" fill="url(#g)"/></svg>"##,
    br##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#336699"/></svg>"##,
);

// light-dark() takes its first color in a light element color scheme and its second in a dark one (CSS Color 5
// section 7); the element color scheme comes from color-scheme (CSS Color Adjustment 1 sections 2.2 to 2.3).
// librsvg has no color scheme preference: without color-scheme it is light, otherwise the first supported one.
macro_rules! light_dark {
    ($name:ident, $svg_attrs:expr, $body:expr, $expected:expr) => {
        test_compare_render_output!(
            $name,
            20,
            20,
            concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20""##, $svg_attrs, ">", $body, "</svg>").as_bytes(),
            concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill=""##, $expected, r##""/></svg>"##).as_bytes(),
        );
    };
}


light_dark!(light_dark_without_color_scheme, "", r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#336699");
light_dark!(light_dark_in_dark, r##" style="color-scheme: dark""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#996633");
light_dark!(light_dark_in_light_dark, r##" style="color-scheme: light dark""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#336699");
light_dark!(light_dark_in_dark_light, r##" style="color-scheme: dark light""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#996633");
light_dark!(light_dark_in_only_light, r##" style="color-scheme: only light""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#336699");
light_dark!(light_dark_custom_ident_only, r##" style="color-scheme: foo""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#336699");
// SVG 2 section 6.6: color-scheme has no presentation attribute
light_dark!(color_scheme_attribute_is_not_read, r##" color-scheme="dark""##, r##"<rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#336699");
light_dark!(light_dark_inherited, r##" style="color-scheme: dark""##, r##"<g><rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/></g>"##, "#996633");
light_dark!(light_dark_normal_under_dark, "", r##"<g style="color-scheme: dark"><rect width="20" height="20" style="color-scheme: normal; fill: light-dark(#336699, #996633)"/></g>"##, "#336699");
light_dark!(light_dark_from_style_sheet, "", r##"<style>rect { color-scheme: dark }</style><rect width="20" height="20" style="fill: light-dark(#336699, #996633)"/>"##, "#996633");
light_dark!(light_dark_nested, r##" style="color-scheme: dark""##, r##"<rect width="20" height="20" style="fill: color-mix(in srgb, light-dark(#336699, #996633), #996633)"/>"##, "#996633");
light_dark!(light_dark_current_color, r##" style="color-scheme: dark""##, r##"<rect width="20" height="20" style="color: light-dark(#336699, #996633); fill: currentcolor"/>"##, "#996633");
light_dark!(light_dark_stop_color, r##" style="color-scheme: dark""##, r##"<linearGradient id="g"><stop offset="0" style="stop-color: light-dark(#336699, #996633)"/><stop offset="1" style="stop-color: light-dark(#336699, #996633)"/></linearGradient><rect width="20" height="20" fill="url(#g)"/>"##, "#996633");

// contrast-color() (CSS Color 5 section 8): white where WCAG 2.1's larger ratio alone would give black, still
// above 3:1; and black where white would be below 3:1
macro_rules! contrast {
    ($name:ident, $background:expr, $expected:expr) => {
        test_compare_render_output!(
            $name,
            20,
            20,
            concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="contrast-color("##, $background, r##")"/></svg>"##).as_bytes(),
            concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill=""##, $expected, r##""/></svg>"##).as_bytes(),
        );
    };
}

contrast!(contrast_color_red, "#ff0000", "white");
contrast!(contrast_color_e74c3c, "#e74c3c", "white");
contrast!(contrast_color_3498db, "#3498db", "white");
contrast!(contrast_color_808080, "#808080", "white");
contrast!(contrast_color_ff6600, "#ff6600", "black");
contrast!(contrast_color_light, "oklch(0.75 0.1 100)", "black");
