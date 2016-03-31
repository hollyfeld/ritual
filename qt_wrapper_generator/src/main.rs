
mod c_function_argument;
mod c_function_signature;
mod c_generator;
mod c_type;
mod caption_strategy;
mod cpp_and_c_method;
mod cpp_data;
mod cpp_header_data;
mod cpp_method;
mod cpp_type;
mod cpp_type_map;
mod enums;
mod read_parse_result;
mod utils;

use std::fs;
use std::path::{PathBuf, Path};
use std::process::Command;
use std::env;

extern crate find_folder;


fn copy_dir<P, Q>(from: P, to: Q)  where P: AsRef<Path>, Q: AsRef<Path> {
  let output = Command::new("cp")
  .arg("-r")
  .arg(from.as_ref().as_os_str())
  .arg(to.as_ref().as_os_str())
  .output()
  .unwrap();
  if !output.status.success() { panic!("cp failed"); }
}

fn remove_dir(path: &PathBuf) {
  match fs::metadata(path) {
    Ok(metadata) => {
      if metadata.is_dir() {
        fs::remove_dir_all(path).unwrap();
      } else {
        fs::remove_file(path).unwrap();
      }
    }
    _ => {}
  }
}

fn main() {
  let arguments: Vec<_> = env::args().collect();
  if arguments.len() != 3 {
    print!("Usage: qt_wrapper_generator qtcw_template_dir output_dir");
  }

  let output_dir = PathBuf::from(arguments[2].clone());
  let qtcw_template_dir = PathBuf::from(arguments[1].clone());
  let parse_result_path = {
    let mut r = output_dir.clone();
    r.push("doc_parse_result.json");
    r
  };
  let mut parse_result = read_parse_result::do_it(&parse_result_path);
  for data in &mut parse_result.headers {
    data.ensure_explicit_destructor();
  }

  let qtcw_path = {
    let mut r = output_dir.clone();
    r.push("qtcw");
    r
  };
  remove_dir(&qtcw_path);
  copy_dir(&qtcw_template_dir, &qtcw_path);

  let mut g = c_generator::CGenerator::new(parse_result, qtcw_path);
  g.generate_all();

  // println!("data: {:?}", parse_result);
}