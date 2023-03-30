/// This macro defines takes a declaration of native packages, and generates two functions,
/// `pub fn native_package_list() -> Vec<PackageInfo>` returning a list of PackageInfos for the
/// packages, and `pub fn handle_native(...) -> Result<Either<Element, String>, CoreError>` which
/// takes a call to a native module and calls it by the handle given in the declaration.
///
/// The native module handlers should have the signature:
/// `pub fn fn_name(ctx: &mut Context, body: &str, args: HashMap<String, String>,
///     inline: bool, output_format: &OutputFormat) -> Result<Either<Element, String>, CoreError>;`
///
/// Example usage:
/// ```rust,ignore
/// define_native_packages! {
///     "package_name_1" => {
///         "module_name_1", vec![ //a vec containing the `ArgInfo`s
///             ArgInfo {
///                 name: "key".to_string(),
///                 default: None,
///                 description: "The key to set".to_string()
///             }
///         ] => handle_module_1,
///         "module_name_2", vec![] => handle_module_2
///     }
/// }
/// ```
macro_rules! define_native_packages {
    ($($name:expr => { $($transform:expr, $arg_info:expr => $handler:ident),* $(,)? };)*) => {
        pub fn native_package_list() -> Vec<PackageInfo> {
            vec![
                $(
                    (PackageInfo {
                        name: $name.to_string(),
                        version: "1".to_string(),
                        description: "A native package supporting native modules".to_string(),
                        transforms: vec![
                            $(
                                (Transform {
                                    from: $transform.to_string(),
                                    to: vec![],
                                    description: None,
                                    arguments: $arg_info,
                                }),
                            )*
                        ]
                    }),
                )*
            ]
        }

        pub fn handle_native<T>(
            ctx: &mut Context<T>,
            package_name: &str,
            node_name: &str, // name of module or parent
            element: &Element,
            args: HashMap<String, ArgValue>,
            output_format: &OutputFormat
        ) -> Result<Either<Element, String>, CoreError> {
            match package_name {
                $(
                    $name => match node_name {
                        $(
                            $transform => match element {
                                Element::Module {
                                    name: _,
                                    args: _,
                                    body,
                                    inline,
                                } => $handler(ctx, body, args, *inline, output_format),
                                _ => Err(
                                    CoreError::NonModuleToNative(
                                        package_name.to_string(),
                                        node_name.to_string()
                                    )
                                )
                            },
                        )*
                        _ => unreachable!("Native: Wrong node name")
                    },
                )*
                _ => unreachable!("Native: Wrong package name")
            }
        }
    }
}

/// This macro takes a list of standard package names and includes them from the
/// `OUT_DIR/pkg_name/wasm32-wasi/release/pkg_name.wasm` wasm file. It is important
/// that the standard package cargo name is the same as the containing folder name.
macro_rules! define_standard_package_loader {
    ($($name:expr),* $(,)?) => {
        #[cfg(all(feature = "bundle_std_packages", feature = "native", feature = "precompile_wasm"))]
        pub fn load_standard_packages(mgr: &mut PackageManager, #[cfg(feature = "native")] engine: &Engine)
            -> Result<(), CoreError> {
            $(
                mgr.load_precompiled_standard_package(
                    include_bytes!(
                        concat!(
                            env!("OUT_DIR"),
                            "/",
                            $name,
                            "/wasm32-wasi/release/",
                            $name,
                            "-precompiled.wir"
                        )
                    ),
                    engine
                )?;
            )*
            Ok(())
        }
        #[cfg(all(feature = "bundle_std_packages", not(all(feature = "native", feature = "precompile_wasm"))))]
        pub fn load_standard_packages(mgr: &mut PackageManager, #[cfg(feature = "native")] engine: &Engine)
            -> Result<(), CoreError> {
            $(
                #[cfg(feature = "native")]
                mgr.load_standard_package(
                    include_bytes!(
                        concat!(
                            env!("OUT_DIR"),
                            "/",
                            $name,
                            "/wasm32-wasi/release/",
                            $name,
                            ".wasm"
                        )
                    ),
                    engine
                )?;
            )*
            $(
                #[cfg(not(feature = "native"))]
                mgr.load_standard_package(
                    include_bytes!(
                        concat!(
                            env!("OUT_DIR"),
                            "/",
                            $name,
                            "/wasm32-wasi/release/",
                            $name,
                            ".wasm"
                        )
                    )
                )?;
            )*
            Ok(())
        }
        #[cfg(not(feature = "bundle_std_packages"))]
        pub fn load_standard_packages(_: &mut PackageManager, #[cfg(feature = "native")] engine: &Engine)
            -> Result<(), CoreError> {
            Ok(())
        }
    };
}

pub(crate) use define_native_packages;
pub(crate) use define_standard_package_loader;
