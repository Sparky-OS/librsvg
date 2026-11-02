//! CSS color values.

use cssparser::{ParseErrorKind, Parser};
use cssparser_color as cssc;
use cssparser_color::{hsl_to_rgb, hwb_to_rgb};

use crate::css_color::{self, CssColor};
use crate::error::*;
use crate::parsers::Parse;
use crate::unit_interval::UnitInterval;
use crate::util;

/// Subset of <https://drafts.csswg.org/css-color-4/#color-type>
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    /// The 'currentcolor' keyword.
    CurrentColor,
    /// Specify sRGB colors directly by their red/green/blue/alpha chanels.
    Rgba(RGBA),
    /// Specifies a color in sRGB using hue, saturation and lightness components.
    Hsl(Hsl),
    /// Specifies a color in sRGB using hue, whiteness and blackness components.
    Hwb(Hwb),
    /// Any `<color>` of CSS Color 4, CSS Color 5 and CSS Color HDR, kept in its own colour space in double precision.
    Css(CssColor),
}

/// A color with red, green, blue, and alpha components, in a byte each.
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RGBA {
    /// The red component.
    pub red: u8,
    /// The green component.
    pub green: u8,
    /// The blue component.
    pub blue: u8,
    /// The alpha component.
    pub alpha: f32,
}

/// Color specified by hue, saturation and lightness components.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hsl {
    /// The hue component.
    pub hue: Option<f32>,
    /// The saturation component.
    pub saturation: Option<f32>,
    /// The lightness component.
    pub lightness: Option<f32>,
    /// The alpha component.
    pub alpha: Option<f32>,
}

/// Color specified by hue, whiteness and blackness components.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hwb {
    /// The hue component.
    pub hue: Option<f32>,
    /// The whiteness component.
    pub whiteness: Option<f32>,
    /// The blackness component.
    pub blackness: Option<f32>,
    /// The alpha component.
    pub alpha: Option<f32>,
}

const OPAQUE: f32 = 1.0;

impl RGBA {
    /// Constructs a new RGBA value from float components. It expects the red,
    /// green, blue and alpha channels in that order, and all values will be
    /// clamped to the 0.0 ... 1.0 range.
    #[inline]
    fn from_floats(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self::new(
            clamp_unit_f32(red),
            clamp_unit_f32(green),
            clamp_unit_f32(blue),
            alpha.clamp(0.0, OPAQUE),
        )
    }

    /// Same thing, but with `u8` values instead of floats in the 0 to 1 range.
    #[inline]
    pub const fn new(red: u8, green: u8, blue: u8, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<cssc::RgbaLegacy> for RGBA {
    fn from(c: cssc::RgbaLegacy) -> RGBA {
        RGBA {
            red: c.red,
            green: c.green,
            blue: c.blue,
            alpha: c.alpha,
        }
    }
}

impl From<cssc::Hsl> for Hsl {
    fn from(c: cssc::Hsl) -> Hsl {
        Hsl {
            hue: c.hue,
            saturation: c.saturation,
            lightness: c.lightness,
            alpha: c.alpha,
        }
    }
}

impl From<cssc::Hwb> for Hwb {
    fn from(c: cssc::Hwb) -> Hwb {
        Hwb {
            hue: c.hue,
            whiteness: c.whiteness,
            blackness: c.blackness,
            alpha: c.alpha,
        }
    }
}

fn clamp_unit_f32(val: f32) -> u8 {
    // Whilst scaling by 256 and flooring would provide
    // an equal distribution of integers to percentage inputs,
    // this is not what Gecko does so we instead multiply by 255
    // and round (adding 0.5 and flooring is equivalent to rounding)
    //
    // Chrome does something similar for the alpha value, but not
    // the rgb values.
    //
    // See <https://bugzilla.mozilla.org/show_bug.cgi?id=1340484>
    //
    // Clamping to 256 and rounding after would let 1.0 map to 256, and
    // `256.0_f32 as u8` is undefined behavior:
    //
    // <https://github.com/rust-lang/rust/issues/10184>
    clamp_floor_256_f32(val * 255.)
}

fn clamp_floor_256_f32(val: f32) -> u8 {
    val.round().clamp(0., 255.) as u8
}

fn parse_plain_color<'i>(parser: &mut Parser<'i, '_>) -> Result<Color, ParseError<'i>> {
    let loc = parser.current_source_location();
    let start = parser.position();

    // One component value: a function with its arguments, an identifier or a hash.  Its source text goes to the
    // CSS colour parser, which knows the whole <color> grammar.
    let is_function = matches!(parser.next()?, cssparser::Token::Function(_));
    if is_function {
        parser.parse_nested_block(|p| {
            while p.next_including_whitespace_and_comments().is_ok() {}
            Ok::<(), ParseError<'i>>(())
        })?;
    }
    let text = parser.slice_from(start).trim();

    if text.eq_ignore_ascii_case("currentcolor") {
        return Ok(Color::CurrentColor);
    }

    css_color::parse_css_color(text, &css_color::Context::default())
        .map(|css| {
            // an sRGB colour that 8 bits hold exactly (hex, names, rgb() with integers) stays an RGBA
            let byte = |v: f64| {
                let b = v * 255.0;
                if (b - b.round()).abs() < 1e-9 { Some(b.round() as u8) } else { None }
            };
            if css.space == css_color::Space::Srgb && css.missing == 0 {
                if let (Some(red), Some(green), Some(blue)) = (byte(css.c[0]), byte(css.c[1]), byte(css.c[2])) {
                    return Color::Rgba(RGBA::new(red, green, blue, css.alpha as f32));
                }
            }
            Color::Css(css)
        })
        .map_err(|e| ParseError {
            kind: ParseErrorKind::Custom(ValueErrorKind::parse_error(&e)),
            location: loc,
        })
}

/// Parse a custom property name.
///
/// <https://drafts.csswg.org/css-variables/#typedef-custom-property-name>
fn parse_name(s: &str) -> Result<&str, ()> {
    if s.starts_with("--") && s.len() > 2 {
        Ok(&s[2..])
    } else {
        Err(())
    }
}

fn parse_var_with_fallback<'i>(parser: &mut Parser<'i, '_>) -> Result<Color, ParseError<'i>> {
    let name = parser.expect_ident_cloned()?;

    // ignore the name for now; we'll use it later when we actually
    // process the names of custom variables
    let _name = parse_name(&name).map_err(|()| {
        parser.new_custom_error(ValueErrorKind::parse_error(&format!(
            "unexpected identifier {}",
            name
        )))
    })?;

    parser.expect_comma()?;

    // FIXME: when fixing #459 (full support for var()), note that
    // https://drafts.csswg.org/css-variables/#using-variables indicates that var(--a,) is
    // a valid function, which means that the fallback value is an empty set of tokens.
    //
    // Also, see Servo's extra code to handle semicolons and stuff in toplevel rules.
    //
    // Also, tweak the tests tagged with "FIXME: var()" below.

    parse_plain_color(parser)
}

impl Parse for Color {
    fn parse<'i>(parser: &mut Parser<'i, '_>) -> Result<Color, ParseError<'i>> {
        if let Ok(c) = parser.try_parse(|p| {
            p.expect_function_matching("var")?;
            p.parse_nested_block(parse_var_with_fallback)
        }) {
            Ok(c)
        } else {
            parse_plain_color(parser)
        }
    }
}

/// Normalizes `h` (a hue value in degrees) to be in the interval `[0.0, 1.0]`.
///
/// Rust-cssparser (the cssparser-color crate) provides
/// [`hsl_to_rgb()`], but it assumes that the hue is between 0 and 1.
/// `normalize_hue()` takes a value with respect to a scale of 0 to
/// 360 degrees and converts it to that different scale.
fn normalize_hue(h: f32) -> f32 {
    h.rem_euclid(360.0) / 360.0
}

pub fn color_to_rgba(color: &Color) -> RGBA {
    match color {
        Color::Rgba(rgba) => *rgba,

        Color::Hsl(hsl) => {
            let hue = normalize_hue(hsl.hue.unwrap_or(0.0));
            let (red, green, blue) = hsl_to_rgb(
                hue,
                hsl.saturation.unwrap_or(0.0),
                hsl.lightness.unwrap_or(0.0),
            );

            RGBA::from_floats(red, green, blue, hsl.alpha.unwrap_or(OPAQUE))
        }

        Color::Hwb(hwb) => {
            let hue = normalize_hue(hwb.hue.unwrap_or(0.0));
            let (red, green, blue) = hwb_to_rgb(
                hue,
                hwb.whiteness.unwrap_or(0.0),
                hwb.blackness.unwrap_or(0.0),
            );

            RGBA::from_floats(red, green, blue, hwb.alpha.unwrap_or(OPAQUE))
        }

        Color::Css(css) => {
            let ([red, green, blue, alpha], _) = color_to_rgba_f64(color);
            let _ = css;
            RGBA::from_floats(red as f32, green as f32, blue as f32, alpha as f32)
        }

        _ => unimplemented!(),
    }
}

/// The colour for an sRGB cairo target in double precision, gamut mapped into [0, 1] as CSS Color 4 section
/// 14.2 says when it is outside sRGB; the second value says whether it was.
pub fn color_to_rgba_f64(color: &Color) -> ([f64; 4], bool) {
    match color {
        Color::Css(css) => {
            let (rgb, mapped) = css_color::gamut_map(css, css_color::Space::Srgb);
            let alpha = if css.alpha_missing() { 0.0 } else { css.alpha };
            ([rgb[0], rgb[1], rgb[2], alpha.clamp(0.0, 1.0)], mapped)
        }
        Color::Hsl(_) | Color::Hwb(_) | Color::Rgba(_) => {
            let rgba = color_to_rgba(color);
            (
                [
                    f64::from(rgba.red) / 255.0,
                    f64::from(rgba.green) / 255.0,
                    f64::from(rgba.blue) / 255.0,
                    f64::from(rgba.alpha),
                ],
                false,
            )
        }
        Color::CurrentColor => unimplemented!(),
    }
}

/// Takes the `opacity` property and an alpha value from a CSS `<color>` and returns a resulting
/// alpha for a computed value.
///
/// `alpha` is `Option<f32>` because that is what cssparser uses everywhere.
fn resolve_alpha(opacity: UnitInterval, alpha: Option<f32>) -> f32 {
    let UnitInterval(o) = opacity;

    let alpha = f64::from(alpha.unwrap_or(0.0)) * o;
    let alpha = util::clamp(alpha, 0.0, 1.0);
    cast::f32(alpha).unwrap()
}

fn black() -> Color {
    Color::Rgba(RGBA::new(0, 0, 0, 1.0))
}

/// Resolves a CSS color from itself, an `opacity` property, and a `color` property (to resolve `currentColor`).
///
/// A CSS color can be `currentColor`, in which case the computed value comes from
/// the `color` property.  You should pass the `color` property's value for `current_color`.
///
/// Note that `currrent_color` can itself have a value of `currentColor`.  In that case, we
/// consider it to be opaque black.
pub fn resolve_color(color: &Color, opacity: UnitInterval, current_color: &Color) -> Color {
    let without_opacity_applied = match color {
        Color::CurrentColor => {
            if let Color::CurrentColor = current_color {
                black()
            } else {
                *current_color
            }
        }

        _ => *color,
    };

    match without_opacity_applied {
        Color::CurrentColor => unreachable!(),

        Color::Rgba(rgba) => Color::Rgba(RGBA {
            alpha: resolve_alpha(opacity, Some(rgba.alpha)),
            ..rgba
        }),

        Color::Hsl(hsl) => Color::Hsl(Hsl {
            alpha: Some(resolve_alpha(opacity, hsl.alpha)),
            ..hsl
        }),

        Color::Css(css) => {
            let mut out = css;
            let UnitInterval(o) = opacity;
            let alpha = if css.alpha_missing() { 0.0 } else { css.alpha };
            out.alpha = util::clamp(alpha * o, 0.0, 1.0);
            out.missing &= !8;
            Color::Css(out)
        }

        Color::Hwb(hwb) => Color::Hwb(Hwb {
            alpha: Some(resolve_alpha(opacity, hwb.alpha)),
            ..hwb
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_color() {
        assert_eq!(
            Color::parse_str("#112233").unwrap(),
            Color::Rgba(RGBA::new(0x11, 0x22, 0x33, 1.0))
        );
    }

    #[test]
    fn var_with_fallback_parses_as_color() {
        assert_eq!(
            Color::parse_str("var(--foo, #112233)").unwrap(),
            Color::Rgba(RGBA::new(0x11, 0x22, 0x33, 1.0))
        );

        // CSS Color 4 section 5.1: the precision as authored, so 50% is 0.5 rather than 128 / 255
        assert_eq!(
            Color::parse_str("var(--foo, rgb(100% 50% 25%)").unwrap(),
            Color::Css(CssColor::new(css_color::Space::Srgb, 1.0, 0.5, 0.25, 1.0))
        );
    }

    #[test]
    fn parses_css_color_4_5_and_hdr() {
        for text in [
            "color(display-p3 0.423457 0.554321 0.676543)",
            "lab(52.2345% 40.1645 59.9971)",
            "oklch(0.6 0.2 30)",
            "color(rec2100-pq 0.5 0.45 0.4)",
            "color-mix(in oklab, #336699 30%, color(display-p3 0.9 0.6 0.1))",
            "oklch(from #336699 calc(l + 0.1) calc(c * 1.5) calc(h + 30))",
            "jzczhz(0.12 0.04 300)",
        ] {
            assert!(matches!(Color::parse_str(text).unwrap(), Color::Css(_)), "{}", text);
        }
    }

    // a colour keeps more than 8 bits: color(srgb 0.123457 ...) reaches cairo as that value
    #[test]
    fn keeps_precision() {
        let c = Color::parse_str("color(srgb 0.123457 0.654321 0.876543)").unwrap();
        let ([r, g, b, a], mapped) = color_to_rgba_f64(&c);
        assert!(!mapped);
        assert!((r - 0.123457).abs() < 1e-12 && (g - 0.654321).abs() < 1e-12 && (b - 0.876543).abs() < 1e-12);
        assert_eq!(a, 1.0);
    }

    // CSS Color 4 section 14.2: outside sRGB, the colour is gamut mapped and that is reported
    #[test]
    fn gamut_maps_outside_srgb() {
        let c = Color::parse_str("color(display-p3 1 1 0)").unwrap();
        let ([r, g, b, _], mapped) = color_to_rgba_f64(&c);
        assert!(mapped);
        assert!((r - 0.99116).abs() < 0.01 && (g - 0.99733).abs() < 0.01 && b.abs() < 0.01);
    }

    #[test]
    fn invalid_colour_is_an_error_that_says_why() {
        assert!(Color::parse_str("color(profoto-rgb 0.4835 0.9167 0.2188)").is_err());
        assert!(Color::parse_str("lab(50% 40 abc)").is_err());
    }

    // FIXME: var() - when fixing #459, see the note in the code above.  All the syntaxes
    // in this test function will become valid once we have full support for var().
    #[test]
    fn var_without_fallback_yields_error() {
        assert!(Color::parse_str("var(--foo)").is_err());
        assert!(Color::parse_str("var(--foo,)").is_err());
        assert!(Color::parse_str("var(--foo, )").is_err());
        assert!(Color::parse_str("var(--foo, this is not a color)").is_err());
        assert!(Color::parse_str("var(--foo, #112233, blah)").is_err());
    }

    #[test]
    fn normalizes_hue() {
        assert_eq!(normalize_hue(0.0), 0.0);
        assert_eq!(normalize_hue(360.0), 0.0);
        assert_eq!(normalize_hue(90.0), 0.25);
        assert_eq!(normalize_hue(-90.0), 0.75);
        assert_eq!(normalize_hue(450.0), 0.25); // 360 + 90 degrees
        assert_eq!(normalize_hue(-450.0), 0.75);
    }

    // Bug #1117
    #[test]
    fn large_hue_value() {
        let _ = color_to_rgba(&Color::parse_str("hsla(70000000000000,4%,10%,.2)").unwrap());
    }
}
