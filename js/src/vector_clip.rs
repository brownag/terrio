//! Vector clipping through the shared core.
use crate::ops::{file_bytes, input_layers, invalid, selected_input_layers};
use crate::{Dataset, Inner, js_err, parse_options};
use serde::Deserialize;
use terrio::crs::SpatialRef;
use terrio::formats::{Kind, VectorDriver};
use terrio::ops::vector_clip::{ClipSpec, ClipTemplateOptions, TemplateSource, validate_region_args};
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
struct Args {
    bbox: Option<[f64; 4]>,
    geometry: Option<serde_json::Value>,
    like: Option<serde::de::IgnoredAny>,
    crs: Option<String>,
    layers: Option<Vec<String>>,
    like_layer: Option<String>,
    like_where: Option<String>,
    like_sql: Option<String>,
}

/// Clips vector features. Supply exactly one of `bbox: [xmin,ymin,xmax,ymax]`,
/// `geometry` (polygon WKT or GeoJSON text/object), or `like` (vector/raster Dataset,
/// uploaded bytes, or GeoJSON). `crs` references bbox/geometry; GeoJSON defaults to
/// WGS84. Optional `layers` selects input layers. `likeLayer`, `likeWhere` and
/// `likeSql` select/filter a polygon template. Raster template bounds use metadata;
/// lazy raster pixels are not loaded. Returns a Dataset retaining attributes and
/// input CRS, with sequential feature IDs. Line cuts interpolate Z/M; constructed
/// polygon topology is XY.
#[wasm_bindgen(js_name=clipVector)]
pub fn clip_vector(src: JsValue, options: JsValue) -> Result<Dataset, JsError> {
    let args: Args = parse_options(&options)?;
    validate_region_args(
        args.bbox.is_some(),
        args.geometry.is_some(),
        args.like.is_some(),
        args.crs.is_some(),
        args.like_layer.is_some() || args.like_where.is_some() || args.like_sql.is_some(),
    )
    .map_err(js_err)?;
    let crs = args.crs.as_deref().map(SpatialRef::parse).transpose().map_err(js_err)?;
    let spec = if let Some(bounds) = args.bbox {
        ClipSpec::Bbox { bounds, crs }
    } else if let Some(geometry) = args.geometry {
        let text = match geometry {
            serde_json::Value::String(s) => s,
            g => g.to_string(),
        };
        ClipSpec::Geometry { text, crs }
    } else {
        let like = js_sys::Reflect::get(&options, &JsValue::from_str("like")).map_err(|_| invalid("cannot read like"))?;
        let template = ClipTemplateOptions { layer: args.like_layer, where_clause: args.like_where, sql: args.like_sql };
        ClipSpec::Template { source: template_source(&like)?, options: template }
    };
    let region = spec.region().map_err(js_err)?;
    let layers = selected_input_layers(&src, args.layers.as_deref().unwrap_or_default())?;
    let out = terrio::ops::vector_clip::clip_all(&layers, &region).map_err(js_err)?;
    Ok(Dataset::from_layers(out, VectorDriver::GeoJSON))
}

/// Reads a clipping template from a Dataset, uploaded bytes, or GeoJSON, as the core expects it.
fn template_source(like: &JsValue) -> Result<TemplateSource, JsError> {
    if let Some(inner) = Dataset::lookup(like) {
        return match &*inner.borrow() {
            Inner::Raster(r) => Ok(TemplateSource::Raster(Box::new(r.header()))),
            Inner::Vector(layers, _) => Ok(TemplateSource::Layers(layers.clone())),
        };
    }
    if let Some((bytes, name)) = file_bytes(like) {
        let kind = terrio::inspection::identify_bytes(&bytes, &name);
        let has_raster = kind.as_ref().is_some_and(|i| i.kinds.contains(&Kind::Raster));
        let has_vector = kind.as_ref().is_some_and(|i| i.kinds.contains(&Kind::Vector));
        if has_raster && !has_vector {
            let raster = terrio::memory::open_raster_lazy_bytes(bytes, &name).map_err(js_err)?.header();
            return Ok(TemplateSource::Raster(Box::new(raster)));
        }
        if has_raster && has_vector {
            return match terrio::memory::open_bytes(bytes, &name).map_err(js_err)? {
                terrio::Dataset::Raster(r) => Ok(TemplateSource::Raster(Box::new(r))),
                terrio::Dataset::Vector(v) => Ok(TemplateSource::Layers(v.layers)),
            };
        }
        let layers = terrio::memory::open_vector_bytes(bytes, &name, &Default::default()).map_err(js_err)?.layers;
        return Ok(TemplateSource::Layers(layers));
    }
    Ok(TemplateSource::Layers(input_layers(like)?))
}
