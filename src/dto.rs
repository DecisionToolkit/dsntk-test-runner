//! # Data transfer objects for input and output values

use crate::model::{Component, InputNode, List, Simple, Value};
use iso8601_duration::Duration;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::process::exit;
use std::str::FromStr;

/// Data transfer object for an error.
#[derive(Debug, Deserialize)]
pub struct ErrorDto {
  /// Error details.
  #[serde(rename = "detail")]
  pub detail: String,
}

/// Data transfer object for a result.
#[derive(Debug, Deserialize)]
pub struct ResultDto<T> {
  /// Result containing data.
  #[serde(rename = "data")]
  pub data: Option<T>,
  /// Result containing errors.
  #[serde(rename = "errors")]
  pub errors: Option<Vec<ErrorDto>>,
}

impl<T> Display for ResultDto<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let str = self
      .errors
      .as_ref()
      .map(|v| v.iter().map(|e| e.detail.clone()).collect::<Vec<String>>().join(", "))
      .unwrap_or_default();
    write!(f, "{}", str)
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputNodeDto {
  #[serde(rename = "name")]
  pub name: String,
  #[serde(rename = "value")]
  pub value: Option<ValueDto>,
}

#[derive(Debug, Deserialize)]
pub struct OptionalValueDto {
  #[serde(rename = "value")]
  pub value: Option<ValueDto>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ValueDto {
  #[serde(rename = "simple", skip_serializing_if = "Option::is_none")]
  pub simple: Option<SimpleDto>,
  #[serde(rename = "components", skip_serializing_if = "Option::is_none")]
  pub components: Option<Vec<ComponentDto>>,
  #[serde(rename = "list", skip_serializing_if = "Option::is_none")]
  pub list: Option<ListDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleDto {
  #[serde(rename = "type")]
  pub typ: Option<String>,
  #[serde(rename = "text")]
  pub text: Option<String>,
  #[serde(rename = "isNil")]
  pub nil: bool,
}

/// Maximum allowed difference between compared decimals.
const EPSILON: Decimal = dec!(0.000_000_006_7);

impl PartialEq for SimpleDto {
  fn eq(&self, rhs: &Self) -> bool {
    let mut result = self.typ == rhs.typ && self.text == rhs.text && self.nil == rhs.nil;
    if !result {
      // compare decimals with epsilon difference
      if is_decimal(&self.typ) && is_decimal(&rhs.typ) && self.nil == rhs.nil {
        let a = Decimal::from_str(&text(self)).unwrap();
        let b = Decimal::from_str(&text(rhs)).unwrap();
        let c = (a - b).abs();
        result = c < EPSILON;
        if !result {
          println!("\n\nEncountered decimal comparison error");
          println!(" a = {}", a);
          println!(" b = {}", b);
          println!(" c = {}", c);
          println!(" ε = {}", EPSILON);
          println!("compared decimals differ more than expected\n");
          exit(0);
        }
        return result;
      }
      // compare durations
      if is_duration(self) && is_duration(rhs) {
        let a = text(self).parse::<Duration>().unwrap();
        let b = text(rhs).parse::<Duration>().unwrap();
        result = a == b;
        return result;
      }
    }
    result
  }
}

fn text(value: &SimpleDto) -> String {
  value.text.as_ref().unwrap().clone()
}

fn is_decimal(opt_type: &Option<String>) -> bool {
  let Some(typ) = opt_type else {
    return false;
  };
  ["xsd:decimal", "xsd:double"].contains(&typ.as_str())
}

fn is_duration(value: &SimpleDto) -> bool {
  if value.nil {
    return false;
  }
  let Some(typ) = &value.typ else {
    return false;
  };
  ["xsd:duration"].contains(&typ.as_str())
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ComponentDto {
  #[serde(rename = "name")]
  pub name: Option<String>,
  #[serde(rename = "value")]
  pub value: Option<ValueDto>,
  #[serde(rename = "isNil")]
  pub nil: bool,
}

impl From<&Component> for ComponentDto {
  fn from(component: &Component) -> Self {
    Self {
      name: component.name.clone(),
      value: component.value.as_ref().map(|value| value.into()),
      nil: component.nil,
    }
  }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ListDto {
  #[serde(rename = "items")]
  pub items: Vec<ValueDto>,
  #[serde(rename = "isNil")]
  pub nil: bool,
}

impl From<&List> for ListDto {
  fn from(list: &List) -> Self {
    Self {
      items: list.items.iter().map(ValueDto::from).collect(),
      nil: list.nil,
    }
  }
}

impl From<&InputNode> for InputNodeDto {
  fn from(input_node: &InputNode) -> Self {
    Self {
      name: input_node.name.clone(),
      value: input_node.value.as_ref().map(|value| value.into()),
    }
  }
}

impl From<&Simple> for SimpleDto {
  fn from(simple: &Simple) -> Self {
    Self {
      typ: simple.typ.clone(),
      text: simple.text.clone(),
      nil: simple.nil,
    }
  }
}

impl From<&Value> for ValueDto {
  fn from(value: &Value) -> Self {
    match &value {
      Value::Simple(simple) => Self {
        simple: Some(simple.into()),
        ..Default::default()
      },
      Value::Components(components) => Self {
        components: Some(components.iter().map(ComponentDto::from).collect()),
        ..Default::default()
      },
      Value::List(list) => Self {
        list: Some(ListDto::from(list)),
        ..Default::default()
      },
    }
  }
}
