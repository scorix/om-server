use std::collections::HashMap;

use crate::domain::{
    DataLayout, SourceRegistry, SpatialObjectLocal, SpatialRunSnapshot, WeatherBakeLayer,
    WeatherDataSource, WeatherElement, WeatherModelId,
};
use crate::error::SpatialServiceError;
use crate::r#gen::{
    GetBlendedPointSeriesRequest, GetSpatialPointSeriesResponse,
    SpatialPointSample as ProtoSpatialPointSample,
};
use crate::infrastructure::weather_bake_profile::WeatherBakeProfile;

use super::spatial::{SpatialPointReader, append_derived_wind_speed};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlendedPointSeriesPlan {
    pub primary_model: WeatherModelId,
    pub element_models: HashMap<WeatherElement, WeatherModelId>,
}

pub fn build_blended_plan(
    profile: &WeatherBakeProfile,
    registry: &SourceRegistry,
) -> Result<BlendedPointSeriesPlan, SpatialServiceError> {
    let primary_model = profile
        .layers
        .first()
        .ok_or(SpatialServiceError::EmptyBlendProfile)?
        .model;
    let primary_source =
        registry
            .get(primary_model)
            .ok_or_else(|| SpatialServiceError::UnknownModel {
                model: primary_model.to_string(),
            })?;

    let mut element_models = HashMap::new();
    for &element in primary_source.supported_elements(DataLayout::Spatial) {
        element_models.insert(element, primary_model);
    }
    for spec in &profile.layers {
        for &element in elements_for_bake_layer(spec.layer) {
            element_models.insert(element, spec.model);
        }
    }

    Ok(BlendedPointSeriesPlan {
        primary_model,
        element_models,
    })
}

pub fn get_blended_point_series(
    registry: &SourceRegistry,
    active_catalog: &crate::application::active_catalog::ActiveSpatialCatalog,
    profile: &WeatherBakeProfile,
    request: GetBlendedPointSeriesRequest,
) -> Result<GetSpatialPointSeriesResponse, SpatialServiceError> {
    let plan = build_blended_plan(profile, registry)?;
    let primary_snapshot = require_snapshot(active_catalog, plan.primary_model)?;

    for model in plan.element_models.values() {
        require_snapshot(active_catalog, *model)?;
    }

    let forecast_days = request.days.clamp(1, 16) as usize;
    let mut elements: Vec<String> = plan
        .element_models
        .keys()
        .map(|element| element.as_str().to_string())
        .collect();
    elements.sort();

    let mut samples = Vec::new();
    for (_date, day_objects) in primary_snapshot
        .grouped_by_date()
        .into_iter()
        .take(forecast_days)
    {
        for object in day_objects {
            let values = read_blended_sample(
                registry,
                active_catalog,
                &plan,
                object,
                request.latitude,
                request.longitude,
            )?;
            if values.is_empty() {
                continue;
            }
            samples.push(ProtoSpatialPointSample {
                timestamp: object.timestamp.clone(),
                valid_date: object.valid_date.clone(),
                values,
            });
        }
    }

    if samples.is_empty() {
        return Err(SpatialServiceError::EmptyPointSeries);
    }

    Ok(GetSpatialPointSeriesResponse {
        model: plan.primary_model.to_string(),
        run_ref: primary_snapshot.run_ref.clone(),
        elements,
        samples,
    })
}

fn read_blended_sample(
    registry: &SourceRegistry,
    active_catalog: &crate::application::active_catalog::ActiveSpatialCatalog,
    plan: &BlendedPointSeriesPlan,
    anchor: &SpatialObjectLocal,
    latitude: f64,
    longitude: f64,
) -> Result<Vec<crate::r#gen::SpatialElementValue>, SpatialServiceError> {
    let mut by_model: HashMap<WeatherModelId, Vec<WeatherElement>> = HashMap::new();
    for (&element, &model) in &plan.element_models {
        by_model.entry(model).or_default().push(element);
    }

    let mut values = Vec::new();
    for (model, elements) in by_model {
        let snapshot = require_snapshot(active_catalog, model)?;
        let Some(object) = find_object(snapshot.as_ref(), &anchor.valid_date, &anchor.timestamp)
        else {
            if model == plan.primary_model {
                return Ok(Vec::new());
            }
            continue;
        };
        let source = require_source(registry, model)?;
        let mut read = SpatialPointReader::read_elements_at(
            source,
            &object.local_path,
            latitude,
            longitude,
            &elements,
            Some(model.as_str()),
        )
        .map_err(SpatialServiceError::from)?;
        values.append(&mut read);
    }

    values.sort_by(|left, right| left.element.cmp(&right.element));
    append_derived_wind_speed(&mut values);
    Ok(values)
}

fn require_source(
    registry: &SourceRegistry,
    model: WeatherModelId,
) -> Result<&dyn WeatherDataSource, SpatialServiceError> {
    registry
        .get(model)
        .ok_or_else(|| SpatialServiceError::UnknownModel {
            model: model.to_string(),
        })
}

fn require_snapshot(
    active_catalog: &crate::application::active_catalog::ActiveSpatialCatalog,
    model: WeatherModelId,
) -> Result<std::sync::Arc<SpatialRunSnapshot>, SpatialServiceError> {
    active_catalog
        .get(model)
        .ok_or_else(|| SpatialServiceError::NotReady {
            model: model.to_string(),
        })
}

fn find_object<'a>(
    snapshot: &'a SpatialRunSnapshot,
    valid_date: &str,
    timestamp: &str,
) -> Option<&'a SpatialObjectLocal> {
    snapshot
        .objects
        .iter()
        .find(|object| object.valid_date == valid_date && object.timestamp == timestamp)
        .or_else(|| {
            snapshot
                .objects
                .iter()
                .find(|object| object.valid_date == valid_date)
        })
}

fn elements_for_bake_layer(layer: WeatherBakeLayer) -> &'static [WeatherElement] {
    match layer {
        WeatherBakeLayer::Temperature2m => &[WeatherElement::Temperature2m],
        WeatherBakeLayer::CloudCover => &[WeatherElement::CloudCover],
        WeatherBakeLayer::Snowfall => &[WeatherElement::Snowfall],
        WeatherBakeLayer::Wind => &[
            WeatherElement::WindUComponent10m,
            WeatherElement::WindVComponent10m,
        ],
        WeatherBakeLayer::SnowDepth => &[WeatherElement::SnowDepth],
        WeatherBakeLayer::Visibility => &[WeatherElement::Visibility],
        WeatherBakeLayer::ShortwaveRadiation => &[WeatherElement::ShortwaveRadiation],
    }
}

#[cfg(test)]
mod tests {
    use super::{build_blended_plan, elements_for_bake_layer};
    use crate::domain::{WeatherBakeLayer, WeatherElement, WeatherModelId};
    use crate::infrastructure::open_meteo;
    use crate::infrastructure::weather_bake_profile::WeatherBakeLayerSpec;

    #[test]
    fn build_plan_pins_snow_depth_to_profile_model() {
        let registry = open_meteo::OpenMeteoSources.registry();
        let profile = crate::infrastructure::weather_bake_profile::WeatherBakeProfile {
            timeline_model: WeatherModelId::EcmwfIfs,
            layers: vec![
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::Temperature2m,
                    model: WeatherModelId::EcmwfIfs,
                },
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::SnowDepth,
                    model: WeatherModelId::EcmwfIfs025,
                },
            ],
        };

        let plan = build_blended_plan(&profile, &registry).expect("plan");

        assert_eq!(plan.primary_model, WeatherModelId::EcmwfIfs);
        assert_eq!(
            plan.element_models.get(&WeatherElement::SnowDepth),
            Some(&WeatherModelId::EcmwfIfs025)
        );
        assert_eq!(
            plan.element_models.get(&WeatherElement::Temperature2m),
            Some(&WeatherModelId::EcmwfIfs)
        );
    }

    #[test]
    fn bake_layers_map_to_expected_elements() {
        assert!(
            elements_for_bake_layer(WeatherBakeLayer::Wind)
                .contains(&WeatherElement::WindUComponent10m)
        );
        assert!(
            elements_for_bake_layer(WeatherBakeLayer::Wind)
                .contains(&WeatherElement::WindVComponent10m)
        );
    }
}
