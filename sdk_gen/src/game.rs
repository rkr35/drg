#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::fmt::{self, Display, Formatter};

use common::{
    impl_deref, EClassCastFlags, FField, FName, FString, TArray, UClass, UField, UObject, UPackage,
    UStruct,
};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Fmt(#[from] fmt::Error),
}

#[repr(C)]
pub struct FProperty {
    pub base: FField,
    pub ArrayDim: i32,
    pub ElementSize: i32,
    pub PropertyFlags: EPropertyFlags,
    pad0: [u8; 4],
    pub Offset: i32,
    pad1: [u8; 40],
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct EPropertyFlags(pub u64);

#[allow(dead_code)]
impl EPropertyFlags {
    // Engine\Source\Runtime\CoreUObject\Public\UObject\ObjectMacros.h
    pub const CPF_None: Self = Self(0);
    pub const CPF_Edit: Self = Self(0x1); // < Property is user-settable in the editor.
    pub const CPF_ConstParm: Self = Self(0x2); // < This is a constant function parameter
    pub const CPF_BlueprintVisible: Self = Self(0x4); // < This property can be read by blueprint code
    pub const CPF_ExportObject: Self = Self(0x8); // < Object can be exported with actor.
    pub const CPF_BlueprintReadOnly: Self = Self(0x10); // < This property cannot be modified by blueprint code
    pub const CPF_Net: Self = Self(0x20); // < Property is relevant to network replication.
    pub const CPF_EditFixedSize: Self = Self(0x40); // < Indicates that elements of an array can be modified, but its size cannot be changed.
    pub const CPF_Parm: Self = Self(0x80); // < Function/When call parameter.
    pub const CPF_OutParm: Self = Self(0x100); // < Value is copied out after function call.
    pub const CPF_ZeroConstructor: Self = Self(0x200); // < memset is fine for construction
    pub const CPF_ReturnParm: Self = Self(0x400); // < Return value.
    pub const CPF_DisableEditOnTemplate: Self = Self(0x800); // < Disable editing of this property on an archetype/sub-blueprint
    pub const CPF_Transient: Self = Self(0x2000); // < Property is transient: shouldn't be saved or loaded, except for Blueprint CDOs.
    pub const CPF_Config: Self = Self(0x4000); // < Property should be loaded/saved as permanent profile.
    pub const CPF_DisableEditOnInstance: Self = Self(0x10000); // < Disable editing on an instance of this class
    pub const CPF_EditConst: Self = Self(0x20000); // < Property is uneditable in the editor.
    pub const CPF_GlobalConfig: Self = Self(0x40000); // < Load config from base class, not subclass.
    pub const CPF_InstancedReference: Self = Self(0x80000); // < Property is a component references.
    pub const CPF_DuplicateTransient: Self = Self(0x200000); // < Property should always be reset to the default value during any type of duplication (copy/paste, binary duplication, etc.)
    pub const CPF_SubobjectReference: Self = Self(0x400000); // < Property contains subobject references (TSubobjectPtr)
    pub const CPF_SaveGame: Self = Self(0x1000000); // < Property should be serialized for save games, this is only checked for game-specific archives with ArIsSaveGame
    pub const CPF_NoClear: Self = Self(0x2000000); // < Hide clear (and browse) button.
    pub const CPF_ReferenceParm: Self = Self(0x8000000); // < Value is passed by reference; CPF_OutParam and CPF_Param should also be set.
    pub const CPF_BlueprintAssignable: Self = Self(0x10000000); // < MC Delegates only.  Property should be exposed for assigning in blueprint code
    pub const CPF_Deprecated: Self = Self(0x20000000); // < Property is deprecated.  Read it from an archive, but don't save it.
    pub const CPF_IsPlainOldData: Self = Self(0x40000000); // < If this is set, then the property can be memcopied instead of CopyCompleteValue / CopySingleValue
    pub const CPF_RepSkip: Self = Self(0x80000000); // < Not replicated. For non replicated properties in replicated structs 
    pub const CPF_RepNotify: Self = Self(0x100000000); // < Notify actors when a property is replicated
    pub const CPF_Interp: Self = Self(0x200000000); // < interpolatable property for use with matinee
    pub const CPF_NonTransactional: Self = Self(0x400000000); // < Property isn't transacted
    pub const CPF_EditorOnly: Self = Self(0x800000000); // < Property should only be loaded in the editor
    pub const CPF_NoDestructor: Self = Self(0x1000000000); // < No destructor
    pub const CPF_AutoWeak: Self = Self(0x4000000000); // < Only used for weak pointers, means the export type is autoweak
    pub const CPF_ContainsInstancedReference: Self = Self(0x8000000000); // < Property contains component references.
    pub const CPF_AssetRegistrySearchable: Self = Self(0x10000000000); // < asset instances will add properties with this flag to the asset registry automatically
    pub const CPF_SimpleDisplay: Self = Self(0x20000000000); // < The property is visible by default in the editor details view
    pub const CPF_AdvancedDisplay: Self = Self(0x40000000000); // < The property is advanced and not visible by default in the editor details view
    pub const CPF_Protected: Self = Self(0x80000000000); // < property is protected from the perspective of script
    pub const CPF_BlueprintCallable: Self = Self(0x100000000000); // < MC Delegates only.  Property should be exposed for calling in blueprint code
    pub const CPF_BlueprintAuthorityOnly: Self = Self(0x200000000000); // < MC Delegates only.  This delegate accepts (only in blueprint) only events with BlueprintAuthorityOnly.
    pub const CPF_TextExportTransient: Self = Self(0x400000000000); // < Property shouldn't be exported to text format (e.g. copy/paste)
    pub const CPF_NonPIEDuplicateTransient: Self = Self(0x800000000000); // < Property should only be copied in PIE
    pub const CPF_ExposeOnSpawn: Self = Self(0x1000000000000); // < Property is exposed on spawn
    pub const CPF_PersistentInstance: Self = Self(0x2000000000000); // < A object referenced by the property is duplicated like a component. (Each actor should have an own instance.)
    pub const CPF_UObjectWrapper: Self = Self(0x4000000000000); // < Property was parsed as a wrapper class like TSubclassOf<T>, FScriptInterface etc., rather than a USomething*
    pub const CPF_HasGetValueTypeHash: Self = Self(0x8000000000000); // < This property can generate a meaningful hash value.
    pub const CPF_NativeAccessSpecifierPublic: Self = Self(0x10000000000000); // < Public native access specifier
    pub const CPF_NativeAccessSpecifierProtected: Self = Self(0x20000000000000); // < Protected native access specifier
    pub const CPF_NativeAccessSpecifierPrivate: Self = Self(0x40000000000000); // < Private native access specifier
    pub const CPF_SkipSerialization: Self = Self(0x80000000000000); // < Property shouldn't be serialized, can still be exported to text

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0 
    }
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

                ($property:expr, $custom_format:literal) => {
                    let name = (*$property).name();
                    let package = (*$property).package();
                    let is_in_blueprint_module =
                        self.is_struct_blueprint_generated && (*$property).is_blueprint_generated();
                    let same_package = is_in_blueprint_module || package == self.package;

                    if same_package {
                        write!(f, $custom_format, name)?
                    } else {
                        write!(
                            f,
                            $custom_format,
                            format_args!("crate::{}::{}", (*package).short_name(), name)
                        )?
                    }
                };
            }

            // TODO(perf): Investigate lookup table where index == (*self.property).id().trailing_zeros()
            match (*self.property).id() {
                EClassCastFlags::CASTCLASS_FObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    emit_package_qualified_type!((*property).PropertyClass, "*mut {}");
                }

                EClassCastFlags::CASTCLASS_FStructProperty => {
                    let property = self.property.cast::<FStructProperty>();
                    emit_package_qualified_type!((*property).Structure);
                }

                EClassCastFlags::CASTCLASS_FFloatProperty => "f32".fmt(f)?,

                EClassCastFlags::CASTCLASS_FBoolProperty => "bool".fmt(f)?,

                EClassCastFlags::CASTCLASS_FArrayProperty => {
                    let property = self.property.cast::<FArrayProperty>();
                    let property = (*property).Inner;
                    write!(
                        f,
                        "common::TArray<{}>",
                        Self::new(property, self.package, self.is_struct_blueprint_generated)
                    )?;
                }

                EClassCastFlags::CASTCLASS_FIntProperty => "i32".fmt(f)?,

                EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty => {
                    "common::FMulticastScriptDelegate".fmt(f)?;
                }

                EClassCastFlags::CASTCLASS_FEnumProperty => {
                    let property = self.property.cast::<FEnumProperty>();
                    emit_package_qualified_type!((*property).Enumeration);
                }

                EClassCastFlags::CASTCLASS_FByteProperty => {
                    let property = self.property.cast::<FByteProperty>();
                    let enumeration = (*property).Enumeration;

                    if enumeration.is_null() {
                        "u8".fmt(f)?;
                    } else {
                        emit_package_qualified_type!(enumeration);
                    }
                }

                EClassCastFlags::CASTCLASS_FNameProperty => "common::FName".fmt(f)?,

                EClassCastFlags::CASTCLASS_FStrProperty => "common::FString".fmt(f)?,

                EClassCastFlags::CASTCLASS_FClassProperty => {
                    let property = self.property.cast::<FClassProperty>();
                    emit_package_qualified_type!((*property).MetaClass, "*mut {}");
                }

                EClassCastFlags::CASTCLASS_FTextProperty => "common::FText".fmt(f)?,

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
                    )?;
                }

                EClassCastFlags::CASTCLASS_FWeakObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    emit_package_qualified_type!(
                        (*property).PropertyClass,
                        "common::TWeakObjectPtr<{}>"
                    );
                }

                EClassCastFlags::CASTCLASS_FUInt32Property => "u32".fmt(f)?,

                EClassCastFlags::CASTCLASS_FSoftObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    emit_package_qualified_type!(
                        (*property).PropertyClass,
                        "common::TSoftObjectPtr<{}>"
                    );
                }

                EClassCastFlags::CASTCLASS_FSoftClassProperty => {
                    let property = self.property.cast::<FSoftClassProperty>();
                    emit_package_qualified_type!(
                        (*property).MetaClass,
                        "common::TSoftClassPtr<{}>"
                    );
                }

                EClassCastFlags::CASTCLASS_FDelegateProperty => "common::FScriptDelegate".fmt(f)?,

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
                    )?;
                }

                EClassCastFlags::CASTCLASS_FInterfaceProperty => {
                    let property = self.property.cast::<FInterfaceProperty>();
                    emit_package_qualified_type!(
                        (*property).InterfaceClass,
                        "common::TScriptInterface<{}>"
                    );
                }

                EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty => {
                    "common::FSparseDelegate".fmt(f)?;
                }

                EClassCastFlags::CASTCLASS_FUInt16Property => "u16".fmt(f)?,

                EClassCastFlags::CASTCLASS_FDoubleProperty => "f64".fmt(f)?,

                EClassCastFlags::CASTCLASS_FFieldPathProperty => "common::FFieldPath".fmt(f)?,

                EClassCastFlags::CASTCLASS_FInt8Property => "i8".fmt(f)?,

                EClassCastFlags::CASTCLASS_FInt16Property => "i16".fmt(f)?,

                EClassCastFlags::CASTCLASS_FLazyObjectProperty => {
                    let property = self.property.cast::<FObjectPropertyBase>();
                    emit_package_qualified_type!(
                        (*property).PropertyClass,
                        "common::TLazyObjectPtr<{}>"
                    );
                }

                EClassCastFlags::CASTCLASS_FUInt64Property => "u64".fmt(f)?,

                EClassCastFlags::CASTCLASS_FInt64Property => "i64".fmt(f)?,

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
    CppType: FString,
    pub Names: TArray<TPair<FName, i64>>,
    CppForm: i32,
    EnumDisplayNameFn: usize,
}

impl_deref! { UEnum as UField }

#[repr(C)]
pub struct TPair<K, V> {
    pub Key: K,
    pub Value: V,
}
