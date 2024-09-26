//! # Test runner for DMN™ Technology Compatibility Kit

use crate::context::{Context, TestResult};
use crate::dto::{InputNodeDto, OptionalValueDto, ResultDto, ValueDto};
use crate::formatter::{text_executing_test_case, text_green_ok, text_parsing_test_file};
use crate::model::{parse_test_file, Value};
use crate::params::EvaluateParams;
use antex::{Color, ColorMode, StyledText, Text};
use regex::Regex;
use reqwest::blocking::Client;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

mod config;
mod context;
mod dto;
mod formatter;
mod model;
mod params;

const DEFAULT_REMARK: &str = "";
const DIFFERS_REMARK: &str = "actual result differs from expected";

/// Main entrypoint of the runner.
fn main() {
  let cm = ColorMode::default();
  // read configuration from file
  let config = config::get();
  // prepare the full directory path where test are stored
  let root_dir = Path::new(&config.test_cases_dir_path).canonicalize().expect("reading test directory failed");
  // create the testing context
  let mut ctx = Context::new(
    config.stop_on_failure,
    config.file_search_pattern,
    &config.report_file,
    &config.tck_report_file,
    root_dir.to_string_lossy().to_string(),
  );
  if root_dir.exists() && root_dir.is_dir() {
    print!("Starting DMN TCK runner...");
    let client = Client::new();
    println!("ok");
    println!("File search pattern: {}", ctx.file_search_pattern);
    print!("Searching DMN files in directory: {} ... ", root_dir.display());
    let mut files = BTreeMap::new();
    let pattern = Regex::new(&ctx.file_search_pattern).expect("parsing search pattern failed");
    search_files(&root_dir, &pattern, &mut files);
    println!("ok");
    for (dir_name, (files_dmn, files_xml)) in files {
      // retrieve model names and namespaces from DMN files
      for file_dmn in files_dmn {
        ctx.process_model_definitions(&root_dir, &dir_name, &file_dmn);
      }
      // execute all tests
      for file_xml in files_xml {
        let file_path = format!("{}/{}", dir_name, file_xml);
        execute_tests(&mut ctx, &file_path, &client, &config.evaluate_url, cm);
      }
    }
    //------------------------------------------------------------------------------------------------------------------
    // Report number of tests per file.
    //------------------------------------------------------------------------------------------------------------------
    println!("\nTests per file:");
    let mut total_per_file = 0;
    println!("┌────────────────────────────────────────────────────────────────────────┬────────┐");
    for (name, count) in &ctx.test_case_count_per_file {
      println!("│ {:70} │ {:6} │", name, count);
      total_per_file += count;
    }
    println!("├────────────────────────────────────────────────────────────────────────┼────────┤");
    println!("│                                                                  Total │ {:6} │", total_per_file);
    println!("└────────────────────────────────────────────────────────────────────────┴────────┘");
    //------------------------------------------------------------------------------------------------------------------
    // Report execution durations.
    //------------------------------------------------------------------------------------------------------------------
    let durations = ctx
      .test_case_duration
      .iter()
      .map(|(key, duration)| (*duration, key.clone()))
      .collect::<BTreeMap<Duration, (String, String, String)>>();
    for (d, k) in durations {
      println!("{:12} µs  {}/{}/{}", d.as_micros(), k.0, k.1, k.2);
    }
    // Display summary of successful/failed tests
    ctx.display_tests_summary(cm);
    // display summary of successful/failed test cases
    ctx.display_test_cases_summary(cm);
    // display timings summary
    let total_count = ctx.success_count + ctx.failure_count;
    let requests_per_second = total_count as f64 / (ctx.execution_time as f64 / 1_000_000_000.0);
    println!("\nTimings:");
    println!("┌───────────────────────────┬────────┐");
    println!("│ Average request time [ms] │ {:>6.03} │", (ctx.execution_time as f64) / (total_count as f64) / 1_000_000.0);
    println!("│       Requests per second │ {:>6.0} │", requests_per_second);
    println!("└───────────────────────────┴────────┘");
  } else {
    usage();
  }
}

fn execute_tests(ctx: &mut Context, file_path: &str, client: &Client, evaluate_url: &str, cm: ColorMode) {
  text_parsing_test_file(cm, file_path).print();
  let test_cases = parse_test_file(file_path);
  text_green_ok(cm).cprintln();
  let empty_id = String::new();
  let model_file_name = test_cases.model_name.clone().expect("model name not specified in test case");
  let workspace_name = ctx.get_workspace_name(&model_file_name);
  let model_namespace = ctx.get_model_rdnn(&model_file_name);
  let model_name = ctx.get_model_name(&model_file_name);
  for test_case in &test_cases.test_cases {
    let test_case_id = test_case.id.as_ref().unwrap_or(&empty_id);
    let opt_invocable_name = test_case.invocable_name.as_ref().cloned();
    for (i, result_node) in test_case.result_nodes.iter().enumerate() {
      let test_id = if i > 0 { format!("{}:{}", test_case_id, i) } else { test_case_id.to_string() };
      let invocable_name = if let Some(invocable_name) = &opt_invocable_name {
        invocable_name.to_string()
      } else {
        result_node.name.clone()
      };
      text_executing_test_case(cm, &test_id, &model_name, &invocable_name).cprint();
      let invocable_path = format!(
        "{}{}/{}/{}",
        if workspace_name.is_empty() { "".to_string() } else { format!("{}/", workspace_name) },
        model_namespace,
        model_name,
        invocable_name
      );
      let params = EvaluateParams {
        invocable_path,
        input_values: test_case.input_nodes.iter().map(InputNodeDto::from).collect(),
      };
      evaluate_test_case(ctx, file_path, client, evaluate_url, test_case_id, &test_id, &params, &result_node.expected, cm);
    }
  }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_test_case(
  ctx: &mut Context,
  file_path: &str,
  client: &Client,
  evaluate_url: &str,
  test_case_id: &str,
  test_id: &str,
  params: &EvaluateParams,
  opt_expected: &Option<Value>,
  cm: ColorMode,
) {
  let execution_start_time = Instant::now();
  match client.post(evaluate_url).json(&params).send() {
    Ok(response) => {
      let execution_duration = execution_start_time.elapsed();
      ctx.execution_time += execution_duration.as_nanos();
      match response.json::<ResultDto<OptionalValueDto>>() {
        Ok(result) => {
          if let Some(data) = result.data {
            if let Some(result_dto) = data.value {
              if let Some(expected) = opt_expected {
                let expected_dto = ValueDto::from(expected);
                if result_dto == expected_dto {
                  ctx.write_line(file_path, test_case_id, test_id, TestResult::Success, DEFAULT_REMARK, execution_duration, cm);
                } else {
                  ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, DIFFERS_REMARK, execution_duration, cm);
                  let actual_json = serde_json::to_string(&result_dto).unwrap();
                  let expected_json = serde_json::to_string(&expected_dto).unwrap();
                  Text::new(cm).nl().s("    result: ").red().s(actual_json.clone()).cprintln();
                  Text::new(cm).s("  expected: ").green().s(expected_json.clone()).nl().cprintln();
                  let mut result_chars = actual_json.chars();
                  let mut expected_chars = expected_json.chars();
                  let mut index = 0;
                  while let Some((actual_char, expected_char)) = result_chars.next().zip(expected_chars.next()) {
                    if actual_char != expected_char {
                      let pos = if index > 60 { index - 60 } else { 0 };
                      Text::new(cm)
                        .s("    actual: ")
                        .white()
                        .s(&actual_json[pos..index])
                        .red()
                        .s(&actual_json[index..])
                        .cprintln();
                      Text::new(cm)
                        .s("  expected: ")
                        .white()
                        .s(&expected_json[pos..index])
                        .green()
                        .s(&expected_json[index..])
                        .nl()
                        .cprintln();
                      break;
                    }
                    index += 1;
                  }
                  // display pretty json comparison
                  let actual_json_pretty = serde_json::to_string_pretty(&result_dto).unwrap();
                  let expected_json_pretty = serde_json::to_string_pretty(&expected_dto).unwrap();
                  let max_width = actual_json_pretty.lines().map(|line| line.len()).max().unwrap();
                  let mut result_lines = actual_json_pretty.lines();
                  let mut expected_lines = expected_json_pretty.lines();
                  println!("  {0:1$} expected:", "actual:", max_width);
                  while let Some((a, b)) = result_lines.next().zip(expected_lines.next()) {
                    let color_actual = if a != b { Color::Red } else { Color::White };
                    let color_expected = if a != b { Color::Green } else { Color::White };
                    let marker = if a != b { "|" } else { " " };
                    Text::new(cm)
                      .yellow()
                      .s(marker)
                      .clear()
                      .space()
                      .color(color_actual)
                      .s(format!("{:1$}", a, max_width))
                      .clear()
                      .space()
                      .color(color_expected)
                      .s(b)
                      .cprintln();
                  }
                  println!();
                  if ctx.stop_on_failure {
                    std::process::exit(0);
                  }
                }
              } else {
                ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, "no expected value", execution_duration, cm);
              }
            } else {
              ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, "no actual value", execution_duration, cm);
            }
          } else if result.errors.is_some() {
            ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, &result.to_string(), execution_duration, cm);
          } else {
            ctx.write_line(
              file_path,
              test_case_id,
              test_id,
              TestResult::Failure,
              format!("{:?}", result).as_str(),
              execution_duration,
              cm,
            );
          }
        }
        Err(reason) => {
          ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, &reason.to_string(), execution_duration, cm);
        }
      }
    }
    Err(reason) => {
      let execution_duration = execution_start_time.elapsed();
      ctx.execution_time += execution_duration.as_nanos();
      ctx.write_line(file_path, test_case_id, test_id, TestResult::Failure, &reason.to_string(), execution_duration, cm);
    }
  }
}

fn search_files(path: &Path, pattern: &Regex, files: &mut BTreeMap<String, (Vec<String>, Vec<String>)>) {
  if let Ok(entries) = fs::read_dir(path) {
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        search_files(&path, pattern, files);
      } else if let Some(dir) = path.parent() {
        let dir_name = dir.canonicalize().unwrap().display().to_string();
        if let Some(exp) = path.extension() {
          if exp == "dmn" {
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            let full_name = format!("{}/{}", dir_name, file_name);
            if pattern.is_match(&full_name) {
              let (files_dmn, _) = files.entry(dir_name.clone()).or_insert((vec![], vec![]));
              files_dmn.push(file_name);
            }
          }
          if exp == "xml" {
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            let full_name = format!("{}/{}", dir_name, file_name);
            if pattern.is_match(&full_name) {
              let (_, files_xml) = files.entry(dir_name).or_insert((vec![], vec![]));
              files_xml.push(file_name);
            }
          }
        }
      }
    }
  }
}

/// Displays usage message.
fn usage() {
  println!("TBD")
}
