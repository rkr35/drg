#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

#[link(name = "msvcrt")]
extern "C" {}

#[link(name = "vcruntime")]
extern "C" {}

use core::ffi::c_void;
use core::fmt::{self, Write};
use core::str;

mod game;
use game::{FName, TPair, UClass, UEnum, UObject, UPackage, UStruct};
mod list;
use list::List;
mod split;
mod timer;
use timer::Timer;
#[macro_use]
mod util;
mod win;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    extern "Rust" {
        #[link_name = "\n\nDetected possible panic in your code. Remove all panics.\n"]
        fn f() -> !;
    }

    unsafe { f() }
}

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Module(#[from] win::module::Error),
    Game(#[from] game::Error),
    File(#[from] win::file::Error),
    Fmt(#[from] fmt::Error),
    List(#[from] list::Error),
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    // unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    timer::initialize_ticks_per_second();

    if let Err(e) = run() {
        log!("error: {:?}", e);
        idle();
    }

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

unsafe fn on_detach() {}

unsafe fn run() -> Result<(), Error> {
    init_globals()?;
    dump_names()?;
    // dump_objects()?;
    generate_sdk()?;
    idle();
    Ok(())
}

unsafe fn init_globals() -> Result<(), Error> {
    let module = win::Module::current()?;
    log!(
        "module.start = {}, module.size = {}",
        module.start(),
        module.size()
    );
    game::FNamePool::init(&module)?;
    game::FUObjectArray::init(&module)?;
    log!("NamePoolData = {}", game::NamePoolData as usize);
    log!("GUObjectArray = {}", game::GUObjectArray as usize);
    Ok(())
}

unsafe fn dump_names() -> Result<(), Error> {
    let timer = Timer::new("dump global names");

    let mut file = win::File::new(sdk_file!("global_names.txt"))?;

    for name in (*game::NamePoolData).iter() {
        let text = (*name).text();
        writeln!(&mut file, "{}", text)?;
    }

    timer.stop();
    Ok(())
}

unsafe fn dump_objects() -> Result<(), Error> {
    let timer = Timer::new("dump global objects");

    let mut file = win::File::new(sdk_file!("global_objects.txt"))?;

    for object in (*game::GUObjectArray).iter() {
        if object.is_null() {
            writeln!(&mut file, "skipped null object")?;
        } else {
            writeln!(
                &mut file,
                "[{}] {} {}",
                (*object).InternalIndex,
                *object,
                object as usize
            )?;
        }
    }

    timer.stop();
    Ok(())
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
    file: win::File,
}

impl Drop for Package {
    fn drop(&mut self) {
        unsafe {
            (*self.ptr).PIEInstanceID = -1;
        }
    }
}

struct Generator {
    classes: StaticClasses,
    lib_rs: win::File,
    packages: List<Package, 1660>,
}

impl Generator {
    pub unsafe fn new() -> Result<Generator, Error> {
        let mut lib_rs = win::File::new(sdk_file!("src/lib.rs"))?;
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
            win::File::new(path)?
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

    unsafe fn generate_enum(&mut self, enumeration: *mut UEnum) -> Result<(), Error> {
        let variants = (*enumeration).Names.as_slice();

        let representation = if let Some(r) = Self::get_enum_representation(variants) {
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

        for variant in variants.iter()
        {
            let mut text = variant.Key.text();

            if text.ends_with("_MAX") {
                // Skip auto-generated _MAX field.
                continue;
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
        }

        writeln!(file, "}}\n")?;

        Ok(())
    }

    unsafe fn generate_structure(&mut self, structure: *mut UStruct) -> Result<(), Error> {
        Ok(())
    }
}

unsafe fn generate_sdk() -> Result<(), Error> {
    let timer = Timer::new("generate sdk");
    Generator::new()?.generate_sdk()?;
    timer.stop();
    Ok(())
}

unsafe fn idle() {
    log!("Idling. Press enter to continue.");
    win::idle();
}
