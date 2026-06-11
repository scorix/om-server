use chrono::{DateTime, Utc};

use crate::domain::spatial_snapshot::SpatialObjectLocal;
use crate::error::TimestampParseError;

type NativeTimeBounds = (DateTime<Utc>, DateTime<Utc>);

/// Bracket around a target valid time within a model's native timestep list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalBracket<'a> {
    pub before: Option<&'a SpatialObjectLocal>,
    pub after: Option<&'a SpatialObjectLocal>,
    /// `0.0` = `before`, `1.0` = `after`; ignored when both ends are the same object.
    pub fraction: f64,
}

/// Parse spatial / manifest valid-time strings into UTC.
///
/// Accepts `2025-06-09T12:00:00Z`, `2025-06-09T12:00Z`, and compact `2025-06-09T1200`.
pub fn parse_valid_time(value: &str) -> Result<DateTime<Utc>, TimestampParseError> {
    let value = value.trim();
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }

    let compact = value.trim_end_matches('Z');
    let Some((date, time)) = compact.split_once('T') else {
        return Err(TimestampParseError::InvalidFormat {
            timestamp: value.to_string(),
        });
    };
    let normalized = if time.contains(':') {
        format!("{date}T{}", time.trim_end_matches('Z'))
    } else if time.len() == 4 {
        format!("{date}T{}:{}:00", &time[0..2], &time[2..4])
    } else {
        return Err(TimestampParseError::InvalidFormat {
            timestamp: value.to_string(),
        });
    };
    DateTime::parse_from_rfc3339(&format!("{normalized}Z"))
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|_| TimestampParseError::InvalidFormat {
            timestamp: value.to_string(),
        })
}

/// First and last native valid times in `objects`, if any parse successfully.
pub fn native_time_bounds(
    objects: &[SpatialObjectLocal],
) -> Result<Option<NativeTimeBounds>, TimestampParseError> {
    let mut times: Vec<DateTime<Utc>> = objects
        .iter()
        .filter_map(|object| parse_valid_time(&object.timestamp).ok())
        .collect();
    if times.is_empty() {
        return Ok(None);
    }
    times.sort_unstable();
    Ok(Some((times[0], *times.last().expect("times checked"))))
}

/// Shared-timeline timesteps that fall inside a layer source model's native forecast window.
///
/// Layers read from a different model than the timeline grid (e.g. `snow_depth` on
/// `ecmwf_ifs025` aligned to hourly `ecmwf_ifs`) must not bake beyond that source model's
/// last native step — different run batches and coarser native steps otherwise produce
/// hold-last frames that look like missing data.
pub fn layer_valid_times_on_shared_timeline<'a>(
    shared_timeline_objects: &[&'a SpatialObjectLocal],
    source_objects: &[SpatialObjectLocal],
    source_matches_timeline_model: bool,
) -> Result<Vec<&'a SpatialObjectLocal>, TimestampParseError> {
    if source_matches_timeline_model {
        return Ok(shared_timeline_objects.to_vec());
    }
    let Some((start, end)) = native_time_bounds(source_objects)? else {
        return Ok(Vec::new());
    };
    Ok(shared_timeline_objects
        .iter()
        .copied()
        .filter(|object| {
            parse_valid_time(&object.timestamp)
                .ok()
                .is_some_and(|time| time >= start && time <= end)
        })
        .collect())
}

/// Back-compat alias for [`layer_valid_times_on_shared_timeline`].
pub fn canonical_times_within_native_horizon<'a>(
    shared_timeline_objects: &[&'a SpatialObjectLocal],
    source_objects: &[SpatialObjectLocal],
    source_matches_timeline_model: bool,
) -> Result<Vec<&'a SpatialObjectLocal>, TimestampParseError> {
    layer_valid_times_on_shared_timeline(
        shared_timeline_objects,
        source_objects,
        source_matches_timeline_model,
    )
}

/// Find native timesteps bracketing `target`, clamping to the nearest edge when extrapolating.
pub fn bracket_objects<'a>(
    objects: &'a [SpatialObjectLocal],
    target: &str,
) -> Result<TemporalBracket<'a>, TimestampParseError> {
    let target_time = parse_valid_time(target)?;
    if objects.is_empty() {
        return Ok(TemporalBracket {
            before: None,
            after: None,
            fraction: 0.0,
        });
    }

    let mut indexed: Vec<(DateTime<Utc>, &SpatialObjectLocal)> = objects
        .iter()
        .filter_map(|object| {
            parse_valid_time(&object.timestamp)
                .ok()
                .map(|time| (time, object))
        })
        .collect();
    indexed.sort_by_key(|(time, _)| *time);
    indexed.dedup_by_key(|(time, _)| *time);

    if indexed.is_empty() {
        return Ok(TemporalBracket {
            before: None,
            after: None,
            fraction: 0.0,
        });
    }

    if target_time <= indexed[0].0 {
        return Ok(TemporalBracket {
            before: Some(indexed[0].1),
            after: Some(indexed[0].1),
            fraction: 0.0,
        });
    }
    let last = indexed.last().expect("indexed checked");
    if target_time >= last.0 {
        return Ok(TemporalBracket {
            before: Some(last.1),
            after: Some(last.1),
            fraction: 0.0,
        });
    }

    for window in indexed.windows(2) {
        let (left_time, left) = window[0];
        let (right_time, right) = window[1];
        if target_time < left_time || target_time > right_time {
            continue;
        }
        if target_time == left_time {
            return Ok(TemporalBracket {
                before: Some(left),
                after: Some(left),
                fraction: 0.0,
            });
        }
        if target_time == right_time {
            return Ok(TemporalBracket {
                before: Some(right),
                after: Some(right),
                fraction: 0.0,
            });
        }
        let span = (right_time - left_time).num_seconds();
        let fraction = if span <= 0 {
            0.0
        } else {
            (target_time - left_time).num_seconds() as f64 / span as f64
        };
        return Ok(TemporalBracket {
            before: Some(left),
            after: Some(right),
            fraction,
        });
    }

    Ok(TemporalBracket {
        before: Some(last.1),
        after: Some(last.1),
        fraction: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::{bracket_objects, parse_valid_time};
    use crate::domain::spatial_snapshot::SpatialObjectLocal;
    use std::path::PathBuf;

    fn object(timestamp: &str) -> SpatialObjectLocal {
        SpatialObjectLocal {
            object_key: timestamp.to_string(),
            timestamp: timestamp.to_string(),
            valid_date: timestamp.split('T').next().unwrap_or("").to_string(),
            local_path: PathBuf::from(timestamp),
        }
    }

    #[test]
    fn parse_valid_time_accepts_compact_and_rfc3339() {
        let compact = parse_valid_time("2025-06-09T1200").expect("compact");
        let rfc = parse_valid_time("2025-06-09T12:00:00Z").expect("rfc");
        assert_eq!(compact, rfc);
    }

    #[test]
    fn bracket_objects_interpolates_between_native_steps() {
        let objects = vec![
            object("2025-06-09T12:00:00Z"),
            object("2025-06-09T15:00:00Z"),
        ];
        let bracket = bracket_objects(&objects, "2025-06-09T13:00:00Z").expect("bracket");
        assert_eq!(
            bracket.before.map(|value| value.timestamp.as_str()),
            Some("2025-06-09T12:00:00Z")
        );
        assert_eq!(
            bracket.after.map(|value| value.timestamp.as_str()),
            Some("2025-06-09T15:00:00Z")
        );
        assert!((bracket.fraction - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn bracket_objects_clamps_before_first_native_step() {
        let objects = vec![object("2025-06-09T12:00:00Z")];
        let bracket = bracket_objects(&objects, "2025-06-09T09:00:00Z").expect("bracket");
        assert_eq!(bracket.before, bracket.after);
        assert_eq!(bracket.fraction, 0.0);
    }

    #[test]
    fn layer_valid_times_trim_to_source_model_native_horizon() {
        use super::layer_valid_times_on_shared_timeline;

        let shared_timeline = [
            object("2026-06-11T0000"),
            object("2026-06-11T0100"),
            object("2026-06-11T0200"),
            object("2026-06-11T0300"),
            object("2026-06-11T0400"),
        ];
        let shared_refs: Vec<_> = shared_timeline.iter().collect();
        let layer = vec![object("2026-06-11T0100"), object("2026-06-11T0400")];
        let trimmed =
            layer_valid_times_on_shared_timeline(&shared_refs, &layer, false).expect("trim");
        assert_eq!(
            trimmed
                .iter()
                .map(|object| object.timestamp.as_str())
                .collect::<Vec<_>>(),
            vec![
                "2026-06-11T0100",
                "2026-06-11T0200",
                "2026-06-11T0300",
                "2026-06-11T0400"
            ]
        );
    }
}
