use antex::{Color, ColorMode, StyledText, Text};

const GUTTER: usize = 250;

pub fn text_green_ok(cm: ColorMode) -> Text {
  Text::new(cm).green().s("ok")
}

pub fn text_parsing_test_file(cm: ColorMode, file_path: &str) -> Text {
  Text::new(cm)
    .nl()
    .s("  Parsing test file: ")
    .blue()
    .s(file_path)
    .clear()
    .space()
    .dots(GUTTER - 21 - file_path.len())
    .space()
}

pub fn text_executing_test_case(cm: ColorMode, test_id: &str, model_name: &str, invocable_name: &str) -> Text {
  Text::new(cm)
    .s("Executing test case: ")
    .bold()
    .white()
    .s("id")
    .colon()
    .space()
    .clear()
    .blue()
    .s(test_id)
    .clear()
    .bold()
    .white()
    .s(", model name: ")
    .clear()
    .blue()
    .s(model_name)
    .clear()
    .bold()
    .white()
    .s(", invocable name: ")
    .clear()
    .blue()
    .s(invocable_name)
    .clear()
    .space()
    .dots(GUTTER - 57 - test_id.len() - model_name.len() - invocable_name.len())
    .space()
}

pub fn text_success_execution_time_remarks(cm: ColorMode, time: u128, remarks: &str) -> Text {
  Text::new(cm).green().s("success").clear().space().s(time).space().s("µs").space().s(remarks)
}

pub fn text_failure_execution_time_remarks(cm: ColorMode, time: u128, remarks: &str) -> Text {
  Text::new(cm).red().s("failure").clear().space().s(time).space().s("µs").space().yellow().s(remarks).clear()
}

pub fn text_summary_table(cm: ColorMode, total_count: usize, success_count: usize, failure_count: usize) -> Text {
  let (success_percentage, failure_percentage) = perc(total_count, success_count, failure_count);
  let color_success = if success_count > 0 { Color::Green } else { Color::White };
  let color_failure = if failure_count > 0 { Color::Red } else { Color::White };
  Text::new(cm)
    .s("┌─────────┬───────┬─────────┐")
    .nl()
    .s("│")
    .cyan()
    .bold()
    .s("  TOTAL  ")
    .clear()
    .s("│ ")
    .cyan()
    .bold()
    .s(format!("{:>5}", total_count))
    .clear()
    .s(" │")
    .spaces(9)
    .s("│")
    .nl()
    .s("├─────────┼───────┼─────────┤")
    .nl()
    .s("│ ")
    .color(color_success)
    .s("Success")
    .clear()
    .s(" │ ")
    .color(color_success)
    .s(format!("{:>5}", success_count))
    .clear()
    .s(" │")
    .color(color_success)
    .s(format!("{:>7.2}", success_percentage))
    .perc()
    .clear()
    .s(" │")
    .nl()
    .s("│ ")
    .color(color_failure)
    .s("Failure")
    .clear()
    .s(" │ ")
    .color(color_failure)
    .s(format!("{:>5}", failure_count))
    .clear()
    .s(" │")
    .color(color_failure)
    .s(format!("{:>7.2}", failure_percentage))
    .perc()
    .clear()
    .s(" │")
    .nl()
    .s("└─────────┴───────┴─────────┘")
}

/// Calculates percentages.
fn perc(total: usize, success: usize, failure: usize) -> (f64, f64) {
  if total > 0 {
    ((success * 100) as f64 / total as f64, (failure * 100) as f64 / total as f64)
  } else {
    (0.0, 0.0)
  }
}
