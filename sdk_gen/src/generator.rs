use crate::buf_writer::BufWriter;
use crate::game::{self, FName, TPair, UClass, UEnum, UObject, UPackage, UStruct};
use crate::list::{self, List};
use crate::win::file::{self, File};
use crate::{sdk_file, sdk_path};

use core::fmt::{self, Write};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Game(#[from] game::Error),
    File(#[from] file::Error),
    Fmt(#[from] fmt::Error),
    List(#[from] list::Error),
}

struct StaticClasses {
    enumeration: *const UClass,
    structure: *const UClass,
    class: *const UClass,
}

impl StaticClasses {
    pub unsafe fn new() -> Result<StaticClasses, Error> {
        Ok(StaticClasses {
            enumeration: (*game::GUObjectArray)
                .find("Class /Script/CoreUObject.Enum")?
                .cast(),
            structure: (*game::GUObjectArray)
                .find("Class /Script/CoreUObject.ScriptStruct")?
                .cast(),
            class: (*game::GUObjectArray)
                .find("Class /Script/CoreUObject.Class")?
                .cast(),
        })
    }
}

struct Package {
    ptr: *mut game::UPackage,
    file: File,
}

impl Drop for Package {
    fn drop(&mut self) {
        unsafe {
            (*self.ptr).PIEInstanceID = -1;
        }
    }
}

pub struct Generator {
    classes: StaticClasses,
    lib_rs: File,
    packages: List<Package, 1660>,
}

impl Generator {
    pub unsafe fn new() -> Result<Generator, Error> {
        let mut lib_rs = File::new(sdk_file!("src/lib.rs"))?;
        lib_rs.write_str(
            "#[no_std]\n#![no_implicit_prelude]\n#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]\n",
        )?;

        Ok(Generator {
            classes: StaticClasses::new()?,
            lib_rs,
            packages: List::<Package, 1660>::new(),
        })
    }

    pub unsafe fn generate_sdk(&mut self) -> Result<(), Error> {
        for object in (*game::GUObjectArray).iter().filter(|o| !o.is_null()) {
            if (*object).is(self.classes.class) || (*object).is(self.classes.structure) {
                self.generate_structure(object.cast())?;
            } else if (*object).is(self.classes.enumeration) {
                self.generate_enum(object.cast())?;
            }
        }
        Ok(())
    }

    unsafe fn get_package(&mut self, object: *mut UObject) -> Result<&mut Package, Error> {
        let package = (*object).package();
        let is_unseen_package = (*package).PIEInstanceID == -1;

        if is_unseen_package {
            self.register_package(package)?;
        }

        let package = (*package).PIEInstanceID as usize;
        Ok(self.packages.get_unchecked_mut(package))
    }

    unsafe fn get_package_file(
        &mut self,
        object: *mut UObject,
    ) -> Result<BufWriter<&mut File>, Error> {
        Ok(BufWriter::new(&mut self.get_package(object)?.file))
    }

    unsafe fn register_package(&mut self, package: *mut UPackage) -> Result<(), Error> {
        let package_name = (*package).short_name();

        // Create a Rust module file for this package.
        let file = {
            let mut path = List::<u8, 260>::new();
            write!(
                &mut path,
                concat!(sdk_path!(), "/src/{}.rs\0"),
                package_name
            )?;
            File::new(path)?
        };

        // Declare the module in the SDK lib.rs.
        writeln!(&mut self.lib_rs, "pub mod {};", package_name)?;

        // Register this package's index in our package cache.
        (*package).PIEInstanceID = self.packages.len() as i32;

        let p = Package { ptr: package, file };

        // Save the package to our cache.
        self.packages.push(p)?;

        Ok(())
    }

    unsafe fn generate_enum(&mut self, enumeration: *mut UEnum) -> Result<(), Error> {
        let variants = (*enumeration).Names.as_slice();

        let (last, rest) = if let Some(v) = variants.split_last() {
            v
        } else {
            // Don't generate empty enums.
            return Ok(());
        };

        let is_last_variant_autogenerated_max = {
            let last = last.Key.text();
            last.ends_with("_MAX") || last.ends_with("_Max")
        };

        let representation = if is_last_variant_autogenerated_max {
            get_enum_representation(rest)
        } else {
            get_enum_representation(variants)
        };

        let mut file = self.get_package_file(enumeration.cast())?;
        let enum_name = (*enumeration).name();

        writeln!(
            file,
            "// {}\n#[repr(transparent)]\npub struct {name}({});\n\nimpl {name} {{",
            *enumeration,
            representation,
            name = enum_name,
        )?;

        for variant in rest.iter() {
            write_enum_variant(&mut file, enum_name, variant)?;
        }

        if !is_last_variant_autogenerated_max {
            write_enum_variant(&mut file, enum_name, last)?;
        }

        writeln!(file, "}}\n")?;

        Ok(())
    }

    unsafe fn generate_structure(&mut self, structure: *mut UStruct) -> Result<(), Error> {
        let size = (*structure).PropertiesSize;

        if size == 0 {
            return Ok(());
        }

        let package = self.get_package(structure.cast())?;
        let mut file = BufWriter::new(&mut package.file);

        let mut offset = 0;

        let struct_name = (*structure).name();
        let base = (*structure).SuperStruct;

        if base.is_null() {
            writeln!(
                file,
                "// {} is {} bytes\n#[repr(C)]\npub struct {} {{",
                *structure, size, struct_name
            )?;
        } else {
            offset = (*base).PropertiesSize;
            writeln!(
                file,
                "// {} is {} bytes ({} inherited)\n#[repr(C)]\npub struct {} {{",
                *structure, size, offset, struct_name
            )?;

            let base_name = (*base).name();
            let base_package = (*base).package();

            if base_package == package.ptr {
                writeln!(
                    file,
                    "    // offset: 0, size: {}\n    base: {},\n",
                    offset, base_name
                )?;
            } else {
                writeln!(
                    file,
                    "    // offset: 0, size: {}\n    base: crate::{}::{},\n",
                    offset,
                    (*base_package).short_name(),
                    base_name
                )?;
            }
        }

        // todo: add struct fields.

        writeln!(file, "}}\n")?;

        Ok(())
    }
}

unsafe fn get_enum_representation(variants: &[TPair<FName, i64>]) -> &'static str {
    let max_discriminant_value = variants.iter().map(|v| v.Value).max().unwrap_or(0);

    if max_discriminant_value <= u8::MAX.into() {
        "u8"
    } else if max_discriminant_value <= u16::MAX.into() {
        "u16"
    } else if max_discriminant_value <= u32::MAX.into() {
        "u32"
    } else {
        "u64"
    }
}

unsafe fn write_enum_variant(
    mut out: impl Write,
    enum_name: &str,
    variant: &TPair<FName, i64>,
) -> Result<(), Error> {
    let mut text = variant.Key.text();

    if let Some(text_stripped) = text
        .bytes()
        .rposition(|c| c == b':')
        .and_then(|i| text.get(i + 1..))
    {
        text = text_stripped;
    }

    if text == "Self" {
        // `Self` is a Rust keyword.
        text = "SelfVariant";
    }

    if variant.Key.number() > 0 {
        writeln!(
            out,
            "    pub const {}_{}: {enum_name} = {enum_name}({});",
            text,
            variant.Key.number() - 1,
            variant.Value,
            enum_name = enum_name
        )?;
    } else {
        writeln!(
            out,
            "    pub const {}: {enum_name} = {enum_name}({});",
            text,
            variant.Value,
            enum_name = enum_name
        )?;
    }

    Ok(())
}
