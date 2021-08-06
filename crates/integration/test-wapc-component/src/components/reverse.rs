use crate::generated::reverse::*;

pub(crate) fn job(input: Inputs, output: Outputs) -> JobResult {
  let reversed = input.input.chars().rev().collect();
  output.output.done(&reversed)?;
  Ok(())
}
