//! # Context for testing process

use crate::formatter::*;
use antex::ColorMode;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use std::{fmt, fs};
use url::Url;

/// Test results.
pub enum TestResult {
  Success,
  Failure,
  Ignored,
}

impl fmt::Display for TestResult {
  /// Converts [TestResult] into string.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Self::Success => "SUCCESS",
        Self::Failure => "ERROR",
        Self::Ignored => "IGNORED",
      }
    )
  }
}

/// Context used during testing process.
pub struct Context {
  /// Model RDNNs indexed by file name.
  model_rdnns: HashMap<String, String>,
  /// Model names indexed by file name.
  model_names: HashMap<String, String>,
  /// Workspace names indexed by file name.
  workspace_names: HashMap<String, String>,
  /// Test results writer.
  report_writer: BufWriter<File>,
  /// Test cases (TCK ready) results writer.
  tck_report_writer: BufWriter<File>,
  /// Number of tests that have passed.
  pub success_count: usize,
  /// Number of tests that have failed.
  pub failure_count: usize,
  /// Total endpoint execution time in nanoseconds.
  pub execution_time: u128,
  /// Flag indicating if testing should be stopped after first test failure.
  pub stop_on_failure: bool,
  /// Pattern for filtering files to be tested.
  pub file_search_pattern: String,
  /// Tests root directory.
  pub root_dir_path: String,
  /// Test cases that have succeeded.
  pub test_case_success: BTreeSet<(String, String, String)>,
  /// Test cases that have failed.
  pub test_case_failure: BTreeMap<(String, String, String), Vec<String>>,
  /// Number of test cases per file.
  pub test_case_count_per_file: BTreeMap<String, usize>,
  /// Execution duration per test case.
  pub test_case_duration: BTreeMap<(String, String, String), Duration>,
}

impl Context {
  /// Creates a new testing context.
  pub fn new(stop_on_failure: bool, file_search_pattern: String, report_file_name: &str, tck_report_file_name: &str, root_dir: String) -> Self {
    let report_file = File::create(report_file_name).unwrap_or_else(|e| panic!("creating output file {} failed with reason: {}", report_file_name, e));
    let report_writer = BufWriter::new(report_file);
    let tck_report_file = File::create(tck_report_file_name).unwrap_or_else(|e| panic!("creating output file {} failed with reason: {}", tck_report_file_name, e));
    let tck_report_writer = BufWriter::new(tck_report_file);
    Self {
      model_rdnns: HashMap::new(),
      model_names: HashMap::new(),
      workspace_names: HashMap::new(),
      report_writer,
      tck_report_writer,
      success_count: 0,
      failure_count: 0,
      execution_time: 0,
      stop_on_failure,
      file_search_pattern,
      root_dir_path: root_dir + "/",
      test_case_success: BTreeSet::new(),
      test_case_failure: BTreeMap::new(),
      test_case_count_per_file: BTreeMap::new(),
      test_case_duration: BTreeMap::new(),
    }
  }

  pub fn process_model_definitions(&mut self, root_dir_path: &Path, dir_name: &str, file_name: &str) {
    let file_path = Path::new(dir_name).join(Path::new(file_name));
    let content = fs::read_to_string(&file_path).unwrap();
    let document = roxmltree::Document::parse(&content).unwrap();
    let root_node = document.root_element();
    // process model name
    let model_name = root_node.attribute("name").unwrap();
    self.model_names.insert(file_name.to_string(), model_name.to_string());
    // process namespace
    let namespace = root_node.attribute("namespace").unwrap();
    self.model_rdnns.insert(file_name.to_string(), to_rdnn(namespace));
    // process workspace names
    self.workspace_names.insert(file_name.to_string(), workspace_name(root_dir_path, &file_path));
  }

  pub fn get_model_name(&self, file_name: &str) -> String {
    self.model_names.get(file_name).cloned().expect("model name not found for specified file name")
  }

  pub fn get_workspace_name(&self, file_name: &str) -> String {
    self.workspace_names.get(file_name).cloned().expect("workspace name not found for specified file name")
  }

  pub fn get_model_rdnn(&self, file_name: &str) -> String {
    self.model_rdnns.get(file_name).cloned().expect("model RDNN not found for specified file name")
  }

  #[allow(clippy::too_many_arguments)]
  pub fn write_line(&mut self, test_file_name: &str, test_case_id: &str, test_id: &str, test_result: TestResult, remarks: &str, execution_duration: Duration, cm: ColorMode) {
    let test_file_directory = dir_name_stripped_prefix(&dir_name(test_file_name), &self.root_dir_path);
    let test_file_stem = file_stem(test_file_name);
    let test_case_key = (test_file_directory.clone(), test_file_stem.clone(), test_case_id.to_string());
    writeln!(
      self.report_writer,
      r#""{}","{}","{}","{}","{}""#,
      test_file_directory,
      test_file_stem,
      test_id,
      test_result,
      if matches!(test_result, TestResult::Failure) { remarks } else { "" }
    )
    .unwrap_or_else(|e| panic!("writing line to CSV report failed with reason: {}", e));
    self
      .test_case_count_per_file
      .entry(test_file_directory.to_string())
      .and_modify(|count| *count += 1)
      .or_insert(1);
    self.test_case_duration.insert(test_case_key.clone(), execution_duration);
    match test_result {
      TestResult::Success => {
        self.success_count += 1;
        self.test_case_success.insert(test_case_key);
        text_success_execution_time_remarks(cm, execution_duration.as_micros(), remarks).println();
      }
      TestResult::Failure => {
        self.failure_count += 1;
        self
          .test_case_failure
          .entry(test_case_key)
          .and_modify(|failures| failures.push(remarks.to_string()))
          .or_insert(vec![remarks.to_string()]);
        text_failure_execution_time_remarks(cm, execution_duration.as_micros(), remarks).println();
      }
      _ => {}
    }
  }

  pub fn display_tests_summary(&mut self, cm: ColorMode) {
    println!("\nTests:");
    let total_count = self.success_count + self.failure_count;
    text_summary_table(cm, total_count, self.success_count, self.failure_count).println();
  }

  pub fn display_test_cases_summary(&mut self, cm: ColorMode) {
    let mut total = self.test_case_success.clone();
    total.extend(self.test_case_failure.keys().cloned().collect::<HashSet<(String, String, String)>>());
    let mut success = self.test_case_success.clone();
    success.retain(|item| !self.test_case_failure.contains_key(item));
    let total_count = total.len();
    let success_count = success.len();
    let failure_count = self.test_case_failure.len();
    println!("\nTest cases:");
    text_summary_table(cm, total_count, success_count, failure_count).println();

    // Write the TCK compatibility report.
    for key @ (test_directory, test_file, test_case_id) in &total {
      if success.contains(key) {
        writeln!(
          self.tck_report_writer,
          r#""{}","{}","{}","{}","""#,
          test_directory,
          test_file,
          test_case_id,
          TestResult::Success,
        )
        .unwrap_or_else(|reason| panic!("writing line to TCK report failed with reason: {reason}"));
      }
      if self.test_case_failure.contains_key(key) {
        writeln!(
          self.tck_report_writer,
          r#""{}","{}","{}","{}","""#,
          test_directory,
          test_file,
          test_case_id,
          TestResult::Ignored,
        )
        .unwrap_or_else(|reason| panic!("writing line to TCK report failed with reason: {reason}"));
      }
    }
  }
}

/// Retrieves the parent path without file name from given `name`.
pub fn dir_name(name: &str) -> String {
  Path::new(name).parent().unwrap().to_str().unwrap().to_string()
}

/// Retrieves the file name without extension.
pub fn file_stem(name: &str) -> String {
  Path::new(name).file_stem().unwrap().to_str().unwrap().to_string()
}

/// Removes the root directory name from the full directory path.  
fn dir_name_stripped_prefix(full_name: &str, root_dir_name: &str) -> String {
  let appended = root_dir_name.to_string();
  if full_name.starts_with(&appended) {
    full_name.strip_prefix(&appended).unwrap().to_string()
  } else {
    full_name.to_string()
  }
}

/// Returns RDNN built from input URL.
fn to_rdnn(input: &str) -> String {
  let url = Url::parse(input).unwrap();
  let segments = url.path_segments().unwrap();
  let mut path_segments = segments.map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<&str>>();
  let domain = url.domain().unwrap();
  let mut domain_segments = domain.split('.').collect::<Vec<&str>>();
  domain_segments.reverse();
  domain_segments.append(&mut path_segments);
  domain_segments.join("/")
}

/// Returns workspace name created from parent and child paths.
fn workspace_name(parent_path: &Path, child_path: &Path) -> String {
  let canonical_dir = parent_path.canonicalize().expect("failed to read directory");
  let canonical_file_path = child_path.canonicalize().expect("failed to read file");
  let workspace_path = canonical_file_path.parent().expect("failed to get parent directory");
  let workspace_name = workspace_path
    .strip_prefix(&canonical_dir)
    .expect("failed to strip prefix in parent directory")
    .to_string_lossy()
    .replace('\\', "/")
    .trim_start_matches('/')
    .trim_end_matches('/')
    .to_string();
  workspace_name
}
