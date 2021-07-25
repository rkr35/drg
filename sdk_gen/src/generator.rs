use crate::buf_writer::BufWriter;
use crate::game::{
    self, EClassCastFlags, FBoolProperty, FName, FProperty, TPair, UEnum, UObject, UPackage,
    UStruct,
};
use crate::list::List;
use crate::win::file::{self, File};
use crate::{sdk_file, sdk_path};

use core::cmp::Ordering;
use core::fmt::{self, Write};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Game(#[from] game::Error),
    File(#[from] file::Error),
    Fmt(#[from] fmt::Error),

    ZeroSizedField,
    BadBitfieldSize(i32),
    LastBitfield,
    MaxPackages,
    MaxBitfields,
    BitfieldFull,
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
    lib_rs: File,
    packages: List<Package, 1700>,
}

impl Generator {
    pub unsafe fn new() -> Result<Generator, Error> {
        let mut lib_rs = File::new(sdk_file!("src/lib.rs"))?;
        lib_rs.write_str(
            "#![no_std]\n#![no_implicit_prelude]\n#![allow(dead_code, non_camel_case_types, non_snake_case, non_upper_case_globals)]\n",
        )?;

        Ok(Generator {
            lib_rs,
            packages: List::new(),
        })
    }

    pub unsafe fn generate_sdk(&mut self) -> Result<(), Error> {
        for object in (*game::GUObjectArray).iter().filter(|o| !o.is_null()) {
            if (*object).fast_is(EClassCastFlags::CASTCLASS_UClass | EClassCastFlags::CASTCLASS_UScriptStruct) {
                self.generate_structure(object.cast())?;
            } else if (*object).fast_is(EClassCastFlags::CASTCLASS_UEnum) {
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
        self.packages.push(p).map_err(|_| Error::MaxPackages)?;

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

        writeln!(
            file,
            "// {}\n#[repr(transparent)]\npub struct {name}({});\n\nimpl {name} {{",
            *enumeration,
            representation,
            name = (*enumeration).name(),
        )?;

        for variant in rest.iter() {
            write_enum_variant(&mut file, variant)?;
        }

        if !is_last_variant_autogenerated_max {
            write_enum_variant(&mut file, last)?;
        }

        writeln!(file, "}}\n")?;

        Ok(())
    }

    unsafe fn generate_structure(&mut self, structure: *mut UStruct) -> Result<(), Error> {
        let package = self.get_package(structure.cast())?;

        // TODO(perf): Don't need to create a new `BufWriter` if the previous object is from the same package.
        // Reuse previous buffer to reduce total `WriteFile` calls.
        let file = BufWriter::new(&mut package.file);

        StructGenerator::new(structure, package.ptr, file).generate()?;
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
            "    pub const {}_{}: Self = Self({});",
            text,
            variant.Key.number() - 1,
            variant.Value,
        )?;
    } else {
        writeln!(
            out,
            "    pub const {}: Self = Self({});",
            text,
            variant.Value,
        )?;
    }

    Ok(())
}

struct StructGenerator<'a> {
    structure: *mut UStruct,
    package: *mut UPackage,
    file: BufWriter<&'a mut File>,
    offset: i32,
    bitfields: List<List<*const FBoolProperty, 64>, 64>,
    last_bitfield_offset: Option<i32>,
}

impl<'a> StructGenerator<'a> {
    pub fn new(
        structure: *mut UStruct,
        package: *mut UPackage,
        file: BufWriter<&mut File>,
    ) -> StructGenerator {
        StructGenerator {
            structure,
            package,
            file,
            offset: 0,
            bitfields: List::new(),
            last_bitfield_offset: None,
        }
    }

    pub unsafe fn generate(&mut self) -> Result<(), Error> {
        if (*self.structure).PropertiesSize == 0 {
            return Ok(());
        }

        self.write_header()?;
        self.add_fields_and_functions()?;

        writeln!(self.file, "}}\n")?;

        Ok(())
    }

    unsafe fn write_header(&mut self) -> Result<(), Error> {
        let base = (*self.structure).SuperStruct;

        if base.is_null() {
            writeln!(
                self.file,
                "// {} is {} bytes.\n#[repr(C)]\npub struct {} {{",
                *self.structure,
                (*self.structure).PropertiesSize,
                (*self.structure).name()
            )?;
        } else {
            self.write_header_inherited(base)?;
        }

        Ok(())
    }

    unsafe fn write_header_inherited(&mut self, base: *mut UStruct) -> Result<(), Error> {
        self.offset = (*base).PropertiesSize;

        writeln!(
            self.file,
            "// {} is {} bytes ({} inherited).\n#[repr(C)]\npub struct {} {{",
            *self.structure,
            (*self.structure).PropertiesSize,
            self.offset,
            (*self.structure).name()
        )?;

        let base_name = (*base).name();
        let base_package = (*base).package();

        if base_package == self.package {
            writeln!(
                self.file,
                "    // offset: 0, size: {}\n    base: {},\n",
                self.offset, base_name
            )?;
        } else {
            writeln!(
                self.file,
                "    // offset: 0, size: {}\n    base: crate::{}::{},\n",
                self.offset,
                (*base_package).short_name(),
                base_name
            )?;
        }

        Ok(())
    }

    unsafe fn add_fields_and_functions(&mut self) -> Result<(), Error> {
        let mut property = (*self.structure).ChildProperties.cast::<FProperty>();

        while !property.is_null() {
            self.process_property(property)?;
            property = (*property).base.Next.cast();
        }

        self.add_end_of_struct_padding_if_needed()?;

        Ok(())
    }

    unsafe fn process_property(&mut self, property: *const FProperty) -> Result<(), Error> {
        let size = (*property).ElementSize * (*property).ArrayDim;

        if size == 0 {
            return Err(Error::ZeroSizedField);
        }

        if (*property).is(EClassCastFlags::CASTCLASS_FBoolProperty) {
            let property = property.cast::<FBoolProperty>();

            if self.last_bitfield_offset.map_or(false, |o| (*property).base.Offset == o) {
                self.bitfields
                    .last_mut()
                    .ok_or(Error::LastBitfield)?
                    .push(property)
                    .map_err(|_| Error::BitfieldFull)?;

                // We already emitted the bitfield member variable on the first bit.
                return Ok(());
            } else {
                self.add_padding_if_needed(property.cast())?;

                let representation = if size == 1 {
                    "u8"
                } else if size == 2 {
                    "u16"
                } else if size == 4 {
                    "u32"
                } else if size == 8 {
                    "u64"
                } else {
                    return Err(Error::BadBitfieldSize(size));
                };

                writeln!(
                    self.file,
                    "    // offset: {offset}, size: {size}\n    pub bitfield_at_{offset}: {representation},\n",
                    offset = self.offset,
                    size = size,
                    representation = representation,
                )?;

                self.last_bitfield_offset = Some(self.offset);

                self.bitfields
                    .push({
                        let mut b = List::new();
                        b.push(property).map_err(|_| Error::BitfieldFull)?;
                        b
                    })
                    .map_err(|_| Error::MaxBitfields)?;
                
                self.offset += size;
            }
        } else {
            self.add_padding_if_needed(property)?;

            writeln!(
                self.file,
                "    // offset: {offset}, size: {size}\n    pub {name}: [u8; {size}],\n",
                offset = self.offset,
                size = size,
                name = (*property).base.Name,
            )?;

            self.offset += size;
        }

        Ok(())
    }

    unsafe fn add_pad_field(&mut self, from_offset: i32, to_offset: i32) -> Result<(), Error> {
        writeln!(
            self.file,
            "    // offset: {offset}, size: {size}\n    pad_at_{offset}: [u8; {size}],\n",
            offset = from_offset,
            size = to_offset - from_offset,
        )?;

        self.offset = to_offset;

        Ok(())
    }

    unsafe fn add_padding_if_needed(&mut self, property: *const FProperty) -> Result<(), Error> {
        let offset = (*property).Offset;

        match self.offset.cmp(&offset) {
            Ordering::Less => {
                // We believe the structure is currently at `self.offset`. This
                // property is some bytes ahead at `offset`. So we need to add
                // (offset - self.offset) bytes of padding to reach the
                // property.
                self.add_pad_field(self.offset, offset)?
            }

            Ordering::Greater => {
                // The property is some bytes behind our reckoning of the
                // current offset. Until we figure out a better way to handle
                // these lagged properties, we should emit a warning so the SDK
                // user has some idea as to why some fields in some structures
                // don't line up with what they're seeing in ReClass.
                writeln!(self.file, "    // WARNING: Property \"{}\" thinks its offset is {}. We think its offset is {}.", (*property).base.Name, offset, self.offset)?
            }

            Ordering::Equal => {
                // Nothing to do. Our reckoning off the current offset matches
                // the property's offset. No padding or warning required.
            }
        }

        Ok(())
    }

    unsafe fn add_end_of_struct_padding_if_needed(&mut self) -> Result<(), Error> {
        let struct_size = (*self.structure).PropertiesSize;

        match self.offset.cmp(&struct_size) {
            // See comments in `add_padding_if_needed()` for explanation.

            Ordering::Less => {
                self.add_pad_field(self.offset, struct_size)?
            }

            Ordering::Greater => {
                writeln!(self.file, "    // WARNING: This structure thinks its size is {}. We think its size is {}.", struct_size, self.offset)?
            }

            Ordering::Equal => {}
        }

        Ok(())
    }
}
