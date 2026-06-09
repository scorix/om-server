use std::path::Path;

use rusqlite::Connection;

use crate::domain::GeoBoundingBox;
use crate::error::WeatherBakeError;

pub fn load_resort_coverages(
    sqlite_path: &Path,
    complete_only: bool,
) -> Result<Vec<GeoBoundingBox>, WeatherBakeError> {
    let conn = Connection::open(sqlite_path).map_err(|source| WeatherBakeError::SqliteOpen {
        path: sqlite_path.to_path_buf(),
        source,
    })?;
    let mut stmt = conn
        .prepare(
            r#"
SELECT west, south, east, north, coverage_complete
FROM terrain_resort_coverage
ORDER BY elem_type, osm_id
"#,
        )
        .map_err(|source| WeatherBakeError::SqliteQuery { source })?;
    let rows = stmt
        .query_map([], |row| {
            let complete: i64 = row.get(4)?;
            Ok((
                GeoBoundingBox {
                    west: row.get(0)?,
                    south: row.get(1)?,
                    east: row.get(2)?,
                    north: row.get(3)?,
                },
                complete != 0,
            ))
        })
        .map_err(|source| WeatherBakeError::SqliteQuery { source })?;

    let mut coverages = Vec::new();
    for row in rows {
        let (bbox, complete) = row.map_err(|source| WeatherBakeError::SqliteQuery { source })?;
        if complete_only && !complete {
            continue;
        }
        if bbox.west < bbox.east && bbox.south < bbox.north {
            coverages.push(bbox);
        }
    }
    Ok(coverages)
}
