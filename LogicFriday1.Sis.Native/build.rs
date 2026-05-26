use std::error::Error;

const EXTERN_INPUTS: &[&str] = &["src/lib.rs"];

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=LOGICFRIDAY1_SIS_BINDGEN_FORCE");

    for input in EXTERN_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }

    let mut builder = csbindgen::Builder::default();
    for input in EXTERN_INPUTS {
        builder = builder.input_extern_file(input);
    }

    builder
        .csharp_dll_name("logicfriday1_sis")
        .csharp_namespace("LogicFriday1.Sis.Interop")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("internal")
        .generate_csharp_file("../LogicFriday1.Sis/NativeMethods.g.cs")?;

    Ok(())
}
