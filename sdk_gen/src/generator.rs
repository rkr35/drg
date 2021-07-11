use crate::{sdk_file, sdk_path};
use crate::game::{self, FName, TPair, UClass, UEnum, UObject, UPackage, UStruct};
use crate::list::{self, List};
use crate::win::file::{self, File};

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
    // class: *const UClass,
}

impl StaticClasses {
    pub unsafe fn new() -> Result<StaticClasses, Error> {
        Ok(StaticClasses {
            enumeration: (*game::GUObjectArray)
                .find("Class /Script/CoreUObject.Enum")?
                .cast(),
            structure: (*game::GUObjectArray)
                .find("Class /Script/CoreUObject.Struct")?
                .cast(),
            // class: (*game::GUObjectArray).find("Class /Script/CoreUObject.Class")?.cast(),
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
        lib_rs.write_str("#![allow(non_camel_case_types)]\n")?;
        lib_rs.write_str("#![allow(non_snake_case)]\n")?;
        lib_rs.write_str("#![allow(non_upper_case_globals)]\n")?;

        Ok(Generator {
            classes: StaticClasses::new()?,
            lib_rs,
            packages: List::<Package, 1660>::new(),
        })
    }

    pub unsafe fn generate_sdk(&mut self) -> Result<(), Error> {
        for object in (*game::GUObjectArray).iter().filter(|o| !o.is_null()) {
            if (*object).is(self.classes.enumeration) {
                self.generate_enum(object.cast())?;
            } else if (*object).is(self.classes.structure) {
                self.generate_structure(object.cast())?;
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

        let representation = if let Some(r) = get_enum_representation(variants) {
            r
        } else {
            // Don't generate empty enum.
            return Ok(());
        };

        let object = enumeration.cast::<UObject>();
        let file = &mut self.get_package(object)?.file;
        let enum_name = (*object).name();

        writeln!(
            file,
            "// {}\n#[repr(transparent)]\npub struct {name}({});\n\nimpl {name} {{",
            *object,
            representation,
            name = enum_name,
        )?;

        if let Some((last, rest)) = variants.split_last() {
            for variant in rest.iter()
            {
                write_enum_variant(file, enum_name, variant, false)?;
            }

            write_enum_variant(file, enum_name, last, true)?;
        }

        writeln!(file, "}}\n")?;

        Ok(())
    }

    unsafe fn generate_structure(&mut self, structure: *mut UStruct) -> Result<(), Error> {
        Ok(())
    }
}

unsafe fn get_enum_representation(variants: &[TPair<FName, i64>]) -> Option<&'static str> {
    let max_discriminant_value = variants
        .iter()
        .filter(|v|
            // Unreal Engine has a bug where u8 enum classes can have an auto-generated "_MAX" field with value
            // 256. We need to copy this bug so we don't accidentally represent these bugged enums as u16.
            v.Value != 256 || !v.Key.text().ends_with("_MAX"))
        .map(|v| v.Value)
        .max()?;

    Some(if max_discriminant_value <= u8::MAX.into() {
        "u8"
    } else if max_discriminant_value <= u16::MAX.into() {
        "u16"
    } else if max_discriminant_value <= u32::MAX.into() {
        "u32"
    } else {
        "u64"
    })
}

unsafe fn write_enum_variant(file: &mut File, enum_name: &str, variant: &TPair<FName, i64>, is_last_variant: bool) -> Result<(), Error> {
    let mut text = variant.Key.text();

    if is_last_variant && text.ends_with("_MAX") {
        // Skip auto-generated _MAX field.
        return Ok(());
    }

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
            file,
            "    pub const {}_{}: {enum_name} = {enum_name}({});",
            text,
            variant.Key.number() - 1,
            variant.Value,
            enum_name = enum_name
        )?;
    } else {
        writeln!(
            file,
            "    pub const {}: {enum_name} = {enum_name}({});",
            text,
            variant.Value,
            enum_name = enum_name
        )?;
    }

    Ok(())
}
