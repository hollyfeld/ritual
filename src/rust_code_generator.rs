use rust_type::{RustType, RustTypeIndirection, RustFFIFunction, RustToCTypeConversion};
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use rust_info::{RustTypeDeclarationKind, RustTypeWrapperKind, RustModule, RustMethod,
                RustMethodArguments, RustMethodArgumentsVariant, RustMethodScope,
                RustMethodArgument, TraitName};
use std::collections::HashMap;
use utils::{JoinWithString, copy_recursively};
use log;
use utils::PathBufPushTweak;
use std::panic;
use utils::CaseOperations;

extern crate rustfmt;

pub struct RustCodeGenerator {
  crate_name: String,
  output_path: PathBuf,
  template_path: PathBuf,
  c_lib_name: String,
  cpp_lib_name: String,
  rustfmt_config: rustfmt::config::Config,
  rustfmt_config_path: Option<PathBuf>,
}

impl RustCodeGenerator {
  pub fn new(crate_name: String,
             output_path: PathBuf,
             template_path: PathBuf,
             rustfmt_config_path: Option<PathBuf>,
             c_lib_name: String,
             cpp_lib_name: String)
             -> RustCodeGenerator {
    let rustfmt_config_data = match rustfmt_config_path {
      Some(ref path) => {
        log::info(format!("Using rustfmt config file: {:?}", path));
        let mut rustfmt_config_file = File::open(path).unwrap();
        let mut rustfmt_config_toml = String::new();
        rustfmt_config_file.read_to_string(&mut rustfmt_config_toml).unwrap();
        rustfmt_config_toml
      }
      None => include_str!("../templates/crate/rustfmt.toml").to_string(),
    };
    let rustfmt_config = rustfmt::config::Config::from_toml(&rustfmt_config_data);
    RustCodeGenerator {
      crate_name: crate_name,
      output_path: output_path,
      template_path: template_path,
      c_lib_name: c_lib_name,
      cpp_lib_name: cpp_lib_name,
      rustfmt_config: rustfmt_config,
      rustfmt_config_path: rustfmt_config_path,
    }
  }

  pub fn generate_template(&self) {
    match self.rustfmt_config_path {
      Some(ref path) => {
        fs::copy(path, self.output_path.with_added("rustfmt.toml")).unwrap();
      }
      None => {
        let mut rustfmt_file = File::create(self.output_path.with_added("rustfmt.toml")).unwrap();
        rustfmt_file.write(include_bytes!("../templates/crate/rustfmt.toml")).unwrap();
      }
    };

    let mut build_rs_file = File::create(self.output_path.with_added("build.rs")).unwrap();
    build_rs_file.write(include_bytes!("../templates/crate/build.rs")).unwrap();

    let mut cargo_file = File::create(self.output_path.with_added("Cargo.toml")).unwrap();
    // TODO: use supplied version and authors
    write!(cargo_file,
           "[package]
           name = \"{}\"
           version = \"{}\"
           authors = {}
           build = \"build.rs\"\n\n",
           &self.crate_name,
           "0.0.0",
           "[\"Riateche <ri@idzaaus.org>\"]")
      .unwrap();
    write!(cargo_file,
           "[dependencies]
           libc = \"0.2\"
           cpp_box = {{ git = \"https://github.com/rust-qt/cpp_box.git\" }}\n\n")
      .unwrap();
    for item in fs::read_dir(&self.template_path).unwrap() {
      let item = item.unwrap();
      copy_recursively(&item.path().to_path_buf(),
                       &self.output_path.with_added(item.file_name()))
        .unwrap();
    }
  }

  fn rust_type_to_code(&self, rust_type: &RustType) -> String {
    match *rust_type {
      RustType::Void => panic!("rust void can't be converted to code"),
      RustType::Common { ref base, ref is_const, ref indirection, ref generic_arguments, .. } => {
        let mut base_s = base.full_name(Some(&self.crate_name));
        if let &Some(ref args) = generic_arguments {
          base_s = format!("{}<{}>",
                           base_s,
                           args.iter().map(|x| self.rust_type_to_code(x)).join(", "));
        }
        let s = match indirection {
          &RustTypeIndirection::None => base_s,
          &RustTypeIndirection::Ref { ref lifetime } => {
            let lifetime_text = match *lifetime {
              Some(ref lifetime) => format!("'{} ", lifetime),
              None => String::new(),
            };
            if *is_const {
              format!("&{}{}", lifetime_text, base_s)
            } else {
              format!("&{}mut {}", lifetime_text, base_s)
            }
          }
          &RustTypeIndirection::Ptr => {
            if *is_const {
              format!("*const {}", base_s)
            } else {
              format!("*mut {}", base_s)
            }
          }
          &RustTypeIndirection::PtrPtr => {
            if *is_const {
              format!("*const *const {}", base_s)
            } else {
              format!("*mut *mut {}", base_s)
            }
          }
        };
        s
      }
      RustType::FunctionPointer { ref return_type, ref arguments } => {
        format!("extern \"C\" fn({}){}",
                arguments.iter().map(|arg| self.rust_type_to_code(arg)).join(", "),
                match return_type.as_ref() {
                  &RustType::Void => String::new(),
                  return_type => format!(" -> {}", self.rust_type_to_code(return_type)),
                })
      }
    }
  }

  fn rust_ffi_function_to_code(&self, func: &RustFFIFunction) -> String {
    let args = func.arguments
                   .iter()
                   .map(|arg| {
                     format!("{}: {}",
                             arg.name,
                             self.rust_type_to_code(&arg.argument_type))
                   });
    format!("  pub fn {}({}){};\n",
            func.name,
            args.join(", "),
            match func.return_type {
              RustType::Void => String::new(),
              _ => format!(" -> {}", self.rust_type_to_code(&func.return_type)),
            })
  }

  fn generate_ffi_call(&self,
                       func: &RustMethod,
                       variant: &RustMethodArgumentsVariant,
                       shared_arguments: &Vec<RustMethodArgument>)
                       -> String {
    let mut final_args = Vec::new();
    final_args.resize(variant.cpp_method.c_signature.arguments.len(), None);
    let mut all_args = shared_arguments.clone();
    for arg in &variant.arguments {
      all_args.push(arg.clone());
    }
    for arg in &all_args {
      assert!(arg.ffi_index >= 0 && arg.ffi_index < final_args.len() as i32);
      let mut code = arg.name.clone();
      match arg.argument_type.rust_api_to_c_conversion {
        RustToCTypeConversion::None => {}
        RustToCTypeConversion::RefToPtr => {
          code = format!("{} as {}",
                         code,
                         self.rust_type_to_code(&arg.argument_type.rust_ffi_type));

        }
        RustToCTypeConversion::ValueToPtr => {
          let is_const = if let RustType::Common { ref is_const, .. } = arg.argument_type
                                                                           .rust_ffi_type {
            *is_const
          } else {
            panic!("void is not expected here at all!")
          };
          code = format!("{}{} as {}",
                         if is_const {
                           "&"
                         } else {
                           "&mut "
                         },
                         code,
                         self.rust_type_to_code(&arg.argument_type.rust_ffi_type));
        }
        RustToCTypeConversion::QFlagsToUInt => {
          code = format!("{}.to_int() as libc::c_uint", code);
        }
      }
      final_args[arg.ffi_index as usize] = Some(code);
    }

    let mut result = Vec::new();
    let mut maybe_result_var_name = None;
    if let Some(ref i) = variant.return_type_ffi_index {
      let mut return_var_name = "object".to_string();
      let mut ii = 1;
      while variant.arguments.iter().find(|x| &x.name == &return_var_name).is_some() {
        ii += 1;
        return_var_name = format!("object{}", ii);
      }
      result.push(format!("{{\nlet mut {} = unsafe {{ {}::new_uninitialized() }};\n",
                          return_var_name,
                          self.rust_type_to_code(&func.return_type.rust_api_type)));
      final_args[*i as usize] = Some(format!("&mut {}", return_var_name));
      maybe_result_var_name = Some(return_var_name);
    }
    for arg in &final_args {
      if arg.is_none() {
        println!("func: {:?}", func);
        panic!("ffi argument is missing");
      }
    }
    result.push(format!("unsafe {{ ::ffi::{}({}) }}",
                        variant.cpp_method.c_name,
                        final_args.into_iter().map(|x| x.unwrap()).join(", ")));
    if let Some(ref name) = maybe_result_var_name {
      result.push(format!("{}\n}}", name));
    }
    let mut code = result.join("");
    match func.return_type.rust_api_to_c_conversion {
      RustToCTypeConversion::None => {}
      RustToCTypeConversion::RefToPtr => {
        let is_const = if let RustType::Common { ref is_const, .. } = func.return_type
                                                                          .rust_ffi_type {
          *is_const
        } else {
          panic!("void is not expected here at all!")
        };
        code = format!("let ffi_result = {};\nunsafe {{ {}*ffi_result }}",
                       code,
                       if is_const {
                         "& "
                       } else {
                         "&mut "
                       });
      }
      RustToCTypeConversion::ValueToPtr => {
        if maybe_result_var_name.is_none() {
          code = format!("let ffi_result = {};\nunsafe {{ *ffi_result }}", code);
        }
      }
      RustToCTypeConversion::QFlagsToUInt => {
        let mut qflags_type = func.return_type.rust_api_type.clone();
        if let RustType::Common { ref mut generic_arguments, .. } = qflags_type {
          *generic_arguments = None;
        } else {
          unreachable!();
        }
        code = format!("let ffi_result = {};\n{}::from_int(ffi_result as i32)",
                       code,
                       self.rust_type_to_code(&qflags_type));
      }
    }
    return code;
  }

  fn generate_rust_final_function(&self, func: &RustMethod) -> String {
    //    println!("TEST1 {:?}", func);
    //    if func.name == "q_uncompress" {
    //      println!("TEST: {:?}", func);
    //    }
    let public_qualifier = match func.scope {
      RustMethodScope::TraitImpl { .. } => "",
      _ => "pub ",
    };
    let return_type_for_signature = match func.return_type.rust_api_type {
      RustType::Void => String::new(),
      _ => {
        format!(" -> {}",
                self.rust_type_to_code(&func.return_type.rust_api_type))
      }
    };
    let arg_texts = |args: &Vec<RustMethodArgument>| -> Vec<String> {
      args.iter()
          .map(|arg| {
            let mut maybe_mut_declaration = "";
            if let RustType::Common { ref indirection, .. } = arg.argument_type
                                                                 .rust_api_type {
              if *indirection == RustTypeIndirection::None &&
                 arg.argument_type.rust_api_to_c_conversion == RustToCTypeConversion::ValueToPtr {
                if let RustType::Common { ref is_const, .. } = arg.argument_type
                                                                  .rust_ffi_type {
                  if !is_const {
                    maybe_mut_declaration = "mut ";
                  }
                }
              }
            }

            format!("{}{}: {}",
                    maybe_mut_declaration,
                    arg.name,
                    self.rust_type_to_code(&arg.argument_type.rust_api_type))
          })
          .collect()
    };
    match func.arguments {
      RustMethodArguments::SingleVariant(ref variant) => {
        let body = self.generate_ffi_call(func, variant, &Vec::new());

        format!("{pubq}fn {name}({args}){ret} {{\n{body}}}\n\n",
                pubq = public_qualifier,
                name = func.name.last_name(),
                args = arg_texts(&variant.arguments).join(", "),
                ret = return_type_for_signature,
                body = body)
      }
      RustMethodArguments::MultipleVariants { ref params_enum_name,
                                              ref params_trait_name,
                                              ref enum_has_lifetime,
                                              ref shared_arguments,
                                              ref variant_argument_name,
                                              ref variants } => {
        let tpl_type = variant_argument_name.to_class_case();
        let mut args = arg_texts(shared_arguments);
        args.push(format!("{}: {}", variant_argument_name, tpl_type));
        let body = format!("match {}.as_enum() {{\n{}\n}}",
                           variant_argument_name,
                           variants.iter()
                             .enumerate()
                             .map(|(num, variant)| {
            //                               let mut all_args = shared_arguments.clone();
            //                               all_args.append(&mut variant.arguments.clone());
            let var_names: Vec<_> = variant.arguments.iter().map(|x| x.name.clone()).collect();
            let pattern = if var_names.is_empty() {
              String::new()
            } else {
              format!("({})", var_names.join(", "))
            };
            format!("{}::Variant{}{} => {{ {} }},",
                    params_enum_name,
                    num,
                    pattern,
                    self.generate_ffi_call(func, variant, shared_arguments))
          })
                             .join("\n"));
        let lifetime_arg = if *enum_has_lifetime {
          "'a, "
        } else {
          ""
        };
        let lifetime_specifier = if *enum_has_lifetime {
          "<'a>"
        } else {
          ""
        };
        format!("{pubq}fn {name}<{lfarg}{tpl_type}: {trt}{lf}>({args}){ret} {{\n{body}}}\n\n",
                pubq = public_qualifier,
                lfarg = lifetime_arg,
                lf = lifetime_specifier,
                name = func.name.last_name(),
                trt = params_trait_name,
                tpl_type = tpl_type,
                args = args.join(", "),
                ret = return_type_for_signature,
                body = body)
      }
    }
  }

  pub fn generate_lib_file(&self, modules: &Vec<String>) {
    let mut lib_file_path = self.output_path.clone();
    lib_file_path.push("src");
    lib_file_path.push("lib.rs");
    {
      let mut lib_file = File::create(&lib_file_path).unwrap();
      write!(lib_file, "#![allow(drop_with_repr_extern)]\n\n").unwrap();
      write!(lib_file, "pub extern crate libc;\n").unwrap();
      write!(lib_file, "pub extern crate cpp_box;\n\n").unwrap();

      // TODO: get list of modules copied from template
      let built_in_modules = vec!["flags", "ffi"];
      for module in built_in_modules {
        if modules.iter().find(|x| x.as_ref() as &str == module).is_some() {
          panic!("module name conflict");
        }
        if module == "ffi" {
          // TODO: remove allow directive
          // TODO: ffi should be a private mod
          write!(lib_file, "#[allow(dead_code)]\n").unwrap();
        }
        write!(lib_file, "pub mod {};\n\n", module).unwrap();
      }
      for module in modules {
        write!(lib_file, "pub mod {};\n", module).unwrap();
      }
    }
    self.call_rustfmt(&lib_file_path);
  }

  fn generate_module_code(&self, data: &RustModule) -> String {
    let mut results = Vec::new();
    results.push("#[allow(unused_imports)]
      use {libc, cpp_box, std};\n\n"
                   .to_string());

    for type1 in &data.types {
      match type1.kind {
        RustTypeDeclarationKind::CppTypeWrapper { ref kind, ref methods, ref traits, .. } => {
          let r = match *kind {
            RustTypeWrapperKind::Enum { ref values, ref is_flaggable } => {
              let mut r = format!("#[derive(Debug, PartialEq, Eq, Clone)]\n#[repr(C)]\npub enum \
                                   {} {{\n{}\n}}\n\n",
                                  type1.name.last_name(),
                                  values.iter()
                                        .map(|item| format!("  {} = {}", item.name, item.value))
                                        .join(", \n"));
              if *is_flaggable {
                r = format!("{}impl ::flags::FlaggableEnum for {} {{\n
                           \
                             fn to_int(self) -> libc::c_int {{ unsafe {{ \
                             std::mem::transmute(self) }} }}\n
                           fn \
                             enum_name() -> &'static str {{ unimplemented!() }}\n
                        \
                             }}\n\n",
                            r,
                            type1.name.last_name());
              }
              r
            }
            RustTypeWrapperKind::Struct { ref size } => {
              format!("#[repr(C)]\npub struct {name} {{\n  _buffer: [u8; {size}],\n}}\n\n
                       impl {name} {{ pub unsafe fn new_uninitialized() -> {name} {{
                         {name} {{ _buffer: std::mem::uninitialized() }}
                      }} }}\n\n",
                      name = type1.name.last_name(),
                      size = size)
            }
          };
          results.push(r);
          if !methods.is_empty() {
            results.push(format!("impl {} {{\n{}}}\n\n",
                                 type1.name.last_name(),
                                 methods.iter()
                                        .map(|method| self.generate_rust_final_function(method))
                                        .join("")));
          }
          for trait1 in traits {
            let trait_content = match trait1.trait_name {
              TraitName::CppDeletable { ref deleter_name } => {
                format!("fn deleter() -> cpp_box::Deleter<Self> {{\n  ::ffi::{}\n}}\n",
                        deleter_name)
              }
              _ => {
                trait1.methods
                      .iter()
                      .map(|method| self.generate_rust_final_function(method))
                      .join("")
              }
            };

            results.push(format!("impl {} for {} {{\n{}}}\n\n",
                                 trait1.trait_name.to_string(),
                                 type1.name.last_name(),
                                 trait_content));
          }
        }
        RustTypeDeclarationKind::MethodParametersEnum { ref variants,
                                                        ref trait_name,
                                                        ref enum_has_lifetime } => {
          let lifetime = if *enum_has_lifetime {
            Some("a")
          } else {
            None
          };
          let var_texts = variants.iter()
                                  .enumerate()
                                  .map(|(num, variant)| {
                                    let mut tuple_text = variant.iter()
                                                                .map(|t| {
                                                                  match lifetime {
                                                                    Some(lifetime) => {
                      self.rust_type_to_code(&t.with_lifetime(lifetime.to_string()))
                    }
                                                                    None => {
                                                                      self.rust_type_to_code(t)
                                                                    }
                                                                  }
                                                                })
                                                                .join(",");
                                    if !tuple_text.is_empty() {
                                      tuple_text = format!("({})", tuple_text);
                                    }
                                    format!("Variant{}{},", num, tuple_text)
                                  });
          results.push(format!("pub enum {}{} {{\n{}\n}}\n\n",
                               type1.name.last_name(),
                               match lifetime {
                                 Some(lifetime) => format!("<'{}>", lifetime),
                                 None => String::new(),
                               },
                               var_texts.join("\n")));

          for (num, variant) in variants.iter().enumerate() {
            let tuple_item_types: Vec<_> = variant.iter()
                                                  .map(|t| {
                                                    match lifetime {
                                                      Some(lifetime) => {
                           self.rust_type_to_code(&t.with_lifetime(lifetime.to_string()))
                         }
                                                      None => self.rust_type_to_code(t),
                                                    }
                                                  })
                                                  .collect();
            let type_text = if tuple_item_types.len() == 1 {
              tuple_item_types[0].clone()
            } else {
              format!("({})", tuple_item_types.join(","))
            };
            let variant_value = if tuple_item_types.len() == 0 {
              String::new()
            } else if tuple_item_types.len() == 1 {
              "(self)".to_string()
            } else {
              format!("({})",
                      variant.iter()
                             .enumerate()
                             .map(|(num2, _)| format!("self.{}", num2))
                             .join(", "))
            };
            results.push(format!("impl{lf} {trt}{lf} for {type_text} {{\nfn as_enum(self) -> \
                             {enm}{lf} {{\n{enm}::Variant{num}{variant_value}\n}}\n}}\n\n",
                                 lf = match lifetime {
                                   Some(lifetime) => format!("<'{}>", lifetime),
                                   None => String::new(),
                                 },
                                 trt = trait_name.last_name(),
                                 type_text = type_text,
                                 enm = type1.name.last_name(),
                                 num = num,
                                 variant_value = variant_value));
          }
        }
        RustTypeDeclarationKind::MethodParametersTrait { ref enum_name, ref enum_has_lifetime } => {
          let lifetime_specifier = if *enum_has_lifetime {
            "<'a>"
          } else {
            ""
          };
          results.push(format!("pub trait {name}{lf} {{\nfn as_enum(self) -> \
                                {enm}{lf};\n}}",
                               name = type1.name.last_name(),
                               enm = enum_name.last_name(),
                               lf = lifetime_specifier));

        }
      };
    }
    for method in &data.functions {
      results.push(self.generate_rust_final_function(method));
    }

    for submodule in &data.submodules {
      results.push(format!("pub mod {} {{\n{}}}\n\n",
                           submodule.name.last_name(),
                           self.generate_module_code(submodule)));
    }
    return results.join("");
  }

  fn call_rustfmt(&self, path: &PathBuf) {
    let result = panic::catch_unwind(|| {
      rustfmt::run(rustfmt::Input::File(path.clone()), &self.rustfmt_config)
    });
    match result {
      Ok(rustfmt_result) => {
        if !rustfmt_result.has_no_errors() {
          log::warning(format!("rustfmt failed to format file: {:?}", path));
        }
      }
      Err(cause) => {
        log::warning(format!("rustfmt failed to format file: {:?} (panic: {:?})",
                             path,
                             cause));
      }
    }
    assert!(path.as_path().is_file());
  }

  pub fn generate_module_file(&self, data: &RustModule) {
    let mut file_path = self.output_path.clone();
    file_path.push("src");
    file_path.push(format!("{}.rs", &data.name.last_name()));
    {
      let mut file = File::create(&file_path).unwrap();
      file.write(self.generate_module_code(data).as_bytes()).unwrap();
    }
    self.call_rustfmt(&file_path);

  }

  pub fn generate_ffi_file(&self, functions: &HashMap<String, Vec<RustFFIFunction>>) {
    let mut file_path = self.output_path.clone();
    file_path.push("src");
    file_path.push("ffi.rs");
    {
      let mut file = File::create(&file_path).unwrap();
      write!(file, "use libc;\n\n").unwrap();

      write!(file, "#[link(name = \"{}\")]\n", &self.cpp_lib_name).unwrap();
      //      write!(file, "#[link(name = \"icui18n\")]\n").unwrap();
      //      write!(file, "#[link(name = \"icuuc\")]\n").unwrap();
      //      write!(file, "#[link(name = \"icudata\")]\n").unwrap();
      write!(file, "#[link(name = \"stdc++\")]\n").unwrap();
      write!(file,
             "#[link(name = \"{}\", kind = \"static\")]\n",
             &self.c_lib_name)
        .unwrap();
      write!(file, "extern \"C\" {{\n").unwrap();

      for (include_file, functions) in functions {
        write!(file, "  // Header: {}\n", include_file).unwrap();
        for function in functions {
          file.write(self.rust_ffi_function_to_code(function).as_bytes()).unwrap();
        }
        write!(file, "\n").unwrap();
      }
      write!(file, "}}\n").unwrap();
    }
    // no rustfmt for ffi file
  }
}
