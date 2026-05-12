use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::Path;

use rustyhdf5::{AttrValue, Dataset, File};

#[derive(Debug, Clone, PartialEq)]
pub struct ScaledVariable {
    pub name: String,
    pub shape: Vec<usize>,
    pub units: Option<String>,
    pub values: Vec<f32>,
}

pub struct GoesNetcdfFile {
    file: File,
}

impl GoesNetcdfFile {
    pub fn dataset(&self, name: &str) -> Result<Dataset<'_>, Box<dyn Error>> {
        self.file
            .dataset(name)
            .map_err(|err| boxed_error(format!("dataset not found: {name}: {err}")))
    }
}

pub fn open_goes_netcdf_lossy(path: impl AsRef<Path>) -> Result<GoesNetcdfFile, Box<dyn Error>> {
    Ok(GoesNetcdfFile {
        file: File::open(path)?,
    })
}

pub fn read_scaled_f32(
    file: &GoesNetcdfFile,
    name: &str,
) -> Result<ScaledVariable, Box<dyn Error>> {
    let dataset = file.dataset(name)?;
    let shape = dataset
        .shape()?
        .into_iter()
        .map(usize::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let attrs = dataset.attrs()?;
    let values = scale_values(dataset.read_f64()?, &attrs);

    Ok(ScaledVariable {
        name: name.to_string(),
        shape,
        units: attr_string(&attrs, "units"),
        values,
    })
}

pub fn read_scaled_f32_window(
    file: &GoesNetcdfFile,
    name: &str,
    y_start: usize,
    y_count: usize,
    x_start: usize,
    x_count: usize,
) -> Result<ScaledVariable, Box<dyn Error>> {
    let dataset = file.dataset(name)?;
    let shape = dataset
        .shape()?
        .into_iter()
        .map(usize::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let [ny, nx] = shape.as_slice() else {
        return Err(boxed_error(format!(
            "window reads require a 2D NetCDF variable; {name} has shape {shape:?}"
        )));
    };
    if y_start.saturating_add(y_count) > *ny || x_start.saturating_add(x_count) > *nx {
        return Err(boxed_error(format!(
            "window {x_start}..{} x {y_start}..{} exceeds variable {name} shape {shape:?}",
            x_start.saturating_add(x_count),
            y_start.saturating_add(y_count)
        )));
    }

    let attrs = dataset.attrs()?;
    let full_values = dataset.read_f64()?;
    let mut values = Vec::with_capacity(x_count.saturating_mul(y_count));
    for y in y_start..y_start + y_count {
        let row_start = y.saturating_mul(*nx).saturating_add(x_start);
        values.extend_from_slice(&full_values[row_start..row_start + x_count]);
    }

    Ok(ScaledVariable {
        name: name.to_string(),
        shape: vec![y_count, x_count],
        units: attr_string(&attrs, "units"),
        values: scale_values(values, &attrs),
    })
}

fn scale_values(values: Vec<f64>, attrs: &HashMap<String, AttrValue>) -> Vec<f32> {
    let scale = attr_f64(attrs, "scale_factor").unwrap_or(1.0);
    let offset = attr_f64(attrs, "add_offset").unwrap_or(0.0);
    let fill = attr_f64(attrs, "_FillValue");
    let valid_range =
        attr_f64_vec(attrs, "valid_range").and_then(|values| match values.as_slice() {
            [min, max, ..] => Some((*min, *max)),
            _ => None,
        });

    values
        .into_iter()
        .map(|value| {
            if !value.is_finite()
                || fill.is_some_and(|fill| (value - fill).abs() < 0.5)
                || valid_range.is_some_and(|(min, max)| value < min || value > max)
            {
                f32::NAN
            } else {
                (value * scale + offset) as f32
            }
        })
        .collect()
}

pub fn dataset_attr_f64(dataset: &Dataset<'_>, name: &str) -> Option<f64> {
    dataset
        .attrs()
        .ok()
        .and_then(|attrs| attr_f64(&attrs, name))
}

pub fn dataset_attr_string(dataset: &Dataset<'_>, name: &str) -> Option<String> {
    dataset
        .attrs()
        .ok()
        .and_then(|attrs| attr_string(&attrs, name))
}

fn attr_f64(attrs: &HashMap<String, AttrValue>, name: &str) -> Option<f64> {
    match attrs.get(name)? {
        AttrValue::F64(value) => Some(*value),
        AttrValue::F64Array(values) => values.first().copied(),
        AttrValue::I64(value) => Some(*value as f64),
        AttrValue::I64Array(values) => values.first().map(|&value| value as f64),
        AttrValue::U64(value) => Some(*value as f64),
        AttrValue::String(_) | AttrValue::StringArray(_) => None,
    }
}

fn attr_f64_vec(attrs: &HashMap<String, AttrValue>, name: &str) -> Option<Vec<f64>> {
    match attrs.get(name)? {
        AttrValue::F64(value) => Some(vec![*value]),
        AttrValue::F64Array(values) => Some(values.clone()),
        AttrValue::I64(value) => Some(vec![*value as f64]),
        AttrValue::I64Array(values) => Some(values.iter().map(|&value| value as f64).collect()),
        AttrValue::U64(value) => Some(vec![*value as f64]),
        AttrValue::String(_) | AttrValue::StringArray(_) => None,
    }
}

fn attr_string(attrs: &HashMap<String, AttrValue>, name: &str) -> Option<String> {
    match attrs.get(name)? {
        AttrValue::String(value) => Some(value.clone()),
        AttrValue::StringArray(values) => values.first().cloned(),
        _ => None,
    }
}

fn boxed_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message.into()))
}
