// Written to match the behavior of:
//   GDAL v3.13.3:
//     doc/source/programs/gdal_viewshed.rst
// No code is translated from these sources.

//! `terrio viewshed` (also exposed automatically as an MCP tool).
use super::rasterout::{OutputArgs, open};
use terrio::ops::viewshed::{
    DEFAULT_BAND, DEFAULT_MAX_DISTANCE, DEFAULT_OBSERVER_HEIGHT, DEFAULT_OBSERVER_SPACING, DEFAULT_OUT_OF_RANGE, DEFAULT_TARGET_HEIGHT,
    ViewshedMode, ViewshedOptions, viewshed,
};

#[derive(clap::Args)]
pub struct Args {
    /// Projected DEM, then output path.
    input: String,
    output: String,
    /// Observer easting in the DEM CRS.
    #[arg(long, allow_negative_numbers = true, required_if_eq_any([("om", "NORMAL"), ("om", "DEM"), ("om", "GROUND")]))]
    ox: Option<f64>,
    /// Observer northing in the DEM CRS.
    #[arg(long, allow_negative_numbers = true, required_if_eq_any([("om", "NORMAL"), ("om", "DEM"), ("om", "GROUND")]))]
    oy: Option<f64>,
    /// Observer height above the DEM, in elevation units.
    #[arg(long, default_value_t = DEFAULT_OBSERVER_HEIGHT, allow_negative_numbers = true)]
    oz: f64,
    /// Target height above the DEM (NORMAL only).
    #[arg(long, default_value_t = DEFAULT_TARGET_HEIGHT, allow_negative_numbers = true)]
    tz: f64,
    /// Maximum distance in projected CRS units; zero is unlimited. Also crops the output.
    #[arg(long, default_value_t = DEFAULT_MAX_DISTANCE)]
    md: f64,
    /// Curvature coefficient (1 minus refraction). 0 disables curvature, 1 disables refraction.
    #[arg(long, allow_negative_numbers = true)]
    cc: Option<f64>,
    /// NORMAL visibility, DEM minimum visible elevation, GROUND minimum height above DEM, or ACCUM cumulative visibility.
    #[arg(long, default_value = "NORMAL")]
    om: String,
    /// Spacing between ACCUM observers, in pixels.
    #[arg(long, default_value_t = DEFAULT_OBSERVER_SPACING)]
    os: usize,
    /// Output value outside the maximum distance.
    #[arg(long, default_value_t = DEFAULT_OUT_OF_RANGE)]
    ov: u8,
    /// Input band, from 1. Numeric nodata is treated as elevation, as in GDAL.
    #[arg(short = 'b', long, default_value_t = DEFAULT_BAND)]
    band: usize,
    #[command(flatten)]
    out: OutputArgs,
}

pub fn run(a: Args) -> terrio::Result<()> {
    let mode = ViewshedMode::parse(&a.om)?;
    let o = ViewshedOptions {
        band: a.band,
        observer_height: a.oz,
        target_height: a.tz,
        max_distance: a.md,
        curvature_coefficient: a.cc,
        mode,
        out_of_range: a.ov,
        observer_spacing: a.os,
        ..Default::default()
    }
    .checked(a.ox.zip(a.oy))?;
    let ds = open(&a.input)?;
    a.out.write(&viewshed(&ds, &o)?, &a.output)
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    #[test]
    fn parses_negative_curvature() {
        // Full-parser tests need the same stack as the CLI worker on Windows.
        std::thread::Builder::new()
            .stack_size(crate::STACK_SIZE)
            .spawn(|| {
                let cli =
                    crate::Cli::try_parse_from(["terrio", "viewshed", "dem.tif", "out.tif", "--ox", "5", "--oy", "5", "--cc", "-0.5"])
                        .unwrap();
                let crate::Command::Viewshed(args) = cli.command else { panic!("wrong command") };
                assert_eq!(args.cc, Some(-0.5));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn cli_viewshed_defaults_match_core() {
        use terrio::ops::viewshed::{
            DEFAULT_BAND, DEFAULT_MAX_DISTANCE, DEFAULT_OBSERVER_HEIGHT, DEFAULT_OBSERVER_SPACING, DEFAULT_OUT_OF_RANGE,
            DEFAULT_TARGET_HEIGHT,
        };
        std::thread::Builder::new()
            .stack_size(crate::STACK_SIZE)
            .spawn(|| {
                let cli = crate::Cli::try_parse_from(["terrio", "viewshed", "dem.tif", "out.tif", "--ox", "5", "--oy", "5"]).unwrap();
                let crate::Command::Viewshed(args) = cli.command else { panic!("wrong command") };
                assert_eq!(args.oz, DEFAULT_OBSERVER_HEIGHT);
                assert_eq!(args.tz, DEFAULT_TARGET_HEIGHT);
                assert_eq!(args.md, DEFAULT_MAX_DISTANCE);
                assert_eq!(args.os, DEFAULT_OBSERVER_SPACING);
                assert_eq!(args.ov, DEFAULT_OUT_OF_RANGE);
                assert_eq!(args.band, DEFAULT_BAND);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
