#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::fmt::{self, Display, Formatter};

use common::{EClassCastFlags, FField, UClass, UField, UObject, UPackage, UStruct};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Fmt(#[from] fmt::Error),
}

#[repr(C)]
pub struct FProperty {
    pub base: FField,
    pub ArrayDim: i32,
    pub ElementSize: i32,
    pub PropertyFlags: u64,
    pad0: [u8; 4],
    pub Offset: i32,
    pad1: [u8; 40],
}

pub struct PropertyDisplayable {
    property: *const FProperty,
    package: *const UPackage,
    is_struct_blueprint_generated: bool,
}

impl PropertyDisplayable {
    pub fn new(
        property: *const FProperty,
        package: *const UPackage,
        is_struct_blueprint_generated: bool,
    ) -> Self {
        Self {
            property,
            package,
            is_struct_blueprint_generated,
        }
    }
}

impl FProperty {
    pub unsafe fn is(&self, property: EClassCastFlags) -> bool {
        (*self.base.ClassPrivate).CastFlags.any(property)
    }

    unsafe fn id(&self) -> EClassCastFlags {
        (*self.base.ClassPrivate).Id
    }
}

impl Display for PropertyDisplayable {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe {
            let array_dim = (*self.property).ArrayDim;
            let is_array = array_dim > 1;

            if is_array {
                '['.fmt(f)?;
            }

            macro_rules! emit_package_qualified_type {
                ($property:expr) => {
                    let name = (*$property).name();
                    let package = (*$property).package();

                    if package == self.package {
                        name.fmt(f)?
                    } else {
                        write!(f, "crate::{}::{}", (*package).short_name(), name)?
                    }
                };
            }

            // TODO(perf): Move common properties up.
            // TODO(perf): Investigate lookup table where index == (*self.property).id().trailing_zeros()
            match (*self.property).id() {
                EClassCastFlags::CASTCLASS_FInt8Property => "i8".fmt(f)?,
                EClassCastFlags::CASTCLASS_FByteProperty => {
                    let property = self.property.cast::<FByteProperty>();
                    let enumeration = (*property).Enumeration;

                    if enumeration.is_null() {
                        "u8".fmt(f)?
                    } else {
                        emit_package_qualified_type!(enumeration);
                    }
                }
                EClassCastFlags::CASTCLASS_FIntProperty => "i32".fmt(f)?,
                EClassCastFlags::CASTCLASS_FFloatProperty => "f32".fmt(f)?,
                EClassCastFlags::CASTCLASS_FUInt64Property => "u64".fmt(f)?,
                EClassCastFlags::CASTCLASS_FUInt32Property => "u32".fmt(f)?,
                EClassCastFlags::CASTCLASS_FUInt16Property => "u16".fmt(f)?,
                EClassCastFlags::CASTCLASS_FInt64Property => "i64".fmt(f)?,
                EClassCastFlags::CASTCLASS_FInt16Property => "i16".fmt(f)?,
                EClassCastFlags::CASTCLASS_FDoubleProperty => "f64".fmt(f)?,
                EClassCastFlags::CASTCLASS_FEnumProperty => {
                    let property = self.property.cast::<FEnumProperty>();
                    emit_package_qualified_type!((*property).Enumeration);
                }
                EClassCastFlags::CASTCLASS_FStructProperty => {
                    let property = self.property.cast::<FStructProperty>();
                    emit_package_qualified_type!((*property).Structure);
                }
                EClassCastFlags::CASTCLASS_FObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    let class = (*property).PropertyClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "*mut {}", name)?
                    } else {
                        write!(f, "*mut crate::{}::{}", (*package).short_name(), name)?
                    }
                }
                EClassCastFlags::CASTCLASS_FArrayProperty => {
                    let property = self.property.cast::<FArrayProperty>();
                    let property = (*property).Inner;
                    write!(
                        f,
                        "common::TArray<{}>",
                        Self::new(property, self.package, self.is_struct_blueprint_generated)
                    )?
                }
                EClassCastFlags::CASTCLASS_FStrProperty => "common::FString".fmt(f)?,
                EClassCastFlags::CASTCLASS_FBoolProperty => "bool".fmt(f)?,
                EClassCastFlags::CASTCLASS_FClassProperty => {
                    let property = self.property.cast::<FClassProperty>();
                    let class = (*property).MetaClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "*mut {}", name)?
                    } else {
                        write!(f, "*mut crate::{}::{}", (*package).short_name(), name)?
                    }
                }
                EClassCastFlags::CASTCLASS_FDelegateProperty => "common::FScriptDelegate".fmt(f)?,
                EClassCastFlags::CASTCLASS_FTextProperty => "common::FText".fmt(f)?,
                EClassCastFlags::CASTCLASS_FNameProperty => "common::FName".fmt(f)?,
                EClassCastFlags::CASTCLASS_FInterfaceProperty => {
                    let property = self.property.cast::<FInterfaceProperty>();
                    let class = (*property).InterfaceClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "common::TScriptInterface<{}>", name)?
                    } else {
                        write!(
                            f,
                            "common::TScriptInterface<crate::{}::{}>",
                            (*package).short_name(),
                            name
                        )?
                    }
                }
                EClassCastFlags::CASTCLASS_FWeakObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    let class = (*property).PropertyClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "common::TWeakObjectPtr<{}>", name)?
                    } else {
                        write!(
                            f,
                            "common::TWeakObjectPtr<crate::{}::{}>",
                            (*package).short_name(),
                            name
                        )?
                    }
                }
                EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty => {
                    "common::FMulticastScriptDelegate".fmt(f)?
                }
                EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty => {
                    "common::FSparseDelegate".fmt(f)?
                }
                EClassCastFlags::CASTCLASS_FMapProperty => {
                    let map = self.property.cast::<FMapProperty>();

                    write!(
                        f,
                        "[u8; {}] /* Maps {} to {} */",
                        (*self.property).ElementSize,
                        Self::new(
                            (*map).KeyProp,
                            self.package,
                            self.is_struct_blueprint_generated
                        ),
                        Self::new(
                            (*map).ValueProp,
                            self.package,
                            self.is_struct_blueprint_generated
                        )
                    )?
                }
                EClassCastFlags::CASTCLASS_FSoftObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    let class = (*property).PropertyClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "common::TSoftObjectPtr<{}>", name)?
                    } else {
                        write!(
                            f,
                            "common::TSoftObjectPtr<crate::{}::{}>",
                            (*package).short_name(),
                            name
                        )?
                    }
                }
                EClassCastFlags::CASTCLASS_FSetProperty => {
                    let set = self.property.cast::<FSetProperty>();

                    write!(
                        f,
                        "[u8; {}] /* Set of {} */",
                        (*self.property).ElementSize,
                        Self::new(
                            (*set).ElementProp,
                            self.package,
                            self.is_struct_blueprint_generated
                        ),
                    )?
                }
                EClassCastFlags::CASTCLASS_FSoftClassProperty => {
                    let property = self.property.cast::<FSoftClassProperty>();
                    let class = (*property).MetaClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "common::TSoftClassPtr<{}>", name)?
                    } else {
                        write!(
                            f,
                            "common::TSoftClassPtr<crate::{}::{}>",
                            (*package).short_name(),
                            name
                        )?
                    }
                }
                EClassCastFlags::CASTCLASS_FFieldPathProperty => "common::FFieldPath".fmt(f)?,
                EClassCastFlags::CASTCLASS_FLazyObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    let class = (*property).PropertyClass;
                    let name = (*class).name();
                    let package = (*class).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*class).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, "common::TLazyObjectPtr<{}>", name)?
                    } else {
                        write!(
                            f,
                            "common::TLazyObjectPtr<crate::{}::{}>",
                            (*package).short_name(),
                            name
                        )?
                    }
                }
                id => write!(
                    f,
                    "[u8; {}] /* WARN: UNKNOWN PROPERTY TYPE Id=={}, Address=={}*/",
                    (*self.property).ElementSize,
                    id.0,
                    self.property as usize
                )?,
            }

            if is_array {
                write!(f, "; {}]", array_dim)?;
            }
        }

        Ok(())
    }
}

#[repr(C)]
pub struct FBoolProperty {
    pub base: FProperty,
    pub FieldSize: u8,
    pub ByteOffset: u8,
    pub ByteMask: u8,
    FieldMask: u8,
    pad: [u8; 4],
}

impl FBoolProperty {
    pub fn is_bitfield(&self) -> bool {
        self.FieldMask != 255
    }
}

#[repr(C)]
pub struct FByteProperty {
    pub base: FProperty,
    Enumeration: *const UEnum,
}

#[repr(C)]
pub struct FStructProperty {
    pub base: FProperty,
    Structure: *const UStruct,
}

#[repr(C)]
pub struct FObjectPropertyBase {
    pub base: FProperty,
    PropertyClass: *const UClass,
}

#[repr(C)]
pub struct FClassProperty {
    pub base: FObjectPropertyBase,
    MetaClass: *const UClass,
}

#[repr(C)]
pub struct FArrayProperty {
    pub base: FProperty,
    Inner: *const FProperty,
    pad: [u8; 8],
}

#[repr(C)]
pub struct FEnumProperty {
    pub base: FProperty,
    pad: [u8; 8],
    Enumeration: *const UEnum,
}

#[repr(C)]
pub struct FInterfaceProperty {
    pub base: FProperty,
    InterfaceClass: *const UClass,
}

#[repr(C)]
pub struct FMapProperty {
    pub base: FProperty,
    KeyProp: *const FProperty,
    ValueProp: *const FProperty,
    pad: [u8; 32],
}

#[repr(C)]
pub struct FSetProperty {
    pub base: FProperty,
    ElementProp: *const FProperty,
    pad: [u8; 24],
}

#[repr(C)]
pub struct FSoftClassProperty {
    pub base: FObjectPropertyBase,
    MetaClass: *const UClass,
}

// #[repr(C)]
// pub struct FFieldPathProperty {
//     pub base: FProperty,
//     PropertyClass: *const FFieldClass,
// }

#[repr(C)]
pub struct UEnum {
    base: UField,
    CppType: common::FString,
    pub Names: common::TArray<TPair<common::FName, i64>>,
    CppForm: i32,
    EnumDisplayNameFn: usize,
}

common::impl_deref! { UEnum as UField }

#[repr(C)]
pub struct TPair<K, V> {
    pub Key: K,
    pub Value: V,
}
