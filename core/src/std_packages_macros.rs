/// This macro defines takes a declaration of native packages, and generates two functions,
/// `pub fn native_package_list() -> Vec<PackageInfo>` returning a list of PackageInfos for the
/// packages, and `pub fn handle_native(...) -> Result<Element, CoreError>` which
/// takes a call to a native module and calls it by the handle given in the declaration.
///
/// The native module handlers should have the signature:
/// `pub fn fn_name(ctx: &mut Context, body: &str, args: HashMap<String, String>,
///     inline: bool, output_format: &OutputFormat, id: &GranularId)
///     -> Result<Element, CoreError>;`
///
/// Example usage:
/// ```rust,ignore
/// define_native_packages! {
///     "package_name_1" => {
///         "module_name_1", [], vec![ //a vec containing the `ArgInfo`s
///             ArgInfo {
///                 name: "key".to_string(),
///                 default: None,
///                 description: "The key to set".to_string()
///             }
///         ] => handle_module_1,
///         "module_name_2",
///         [("headings", VarAccess::List(ListAccess::Push))], //a list of kv-pairs of var accesses
///         vec![] => handle_module_2
///     }
/// }
/// ```
macro_rules! define_native_packages {
    ($($name:expr => { desc: $desc:expr, transforms: [ $({name: $transform:expr, desc: $tdesc:expr, vars: $vars:expr, args: $arg_info:expr, func: $handler:ident}),* $(,)? ]})*) => {
        pub fn native_package_list() -> Vec<PackageInfo> {
            vec![
                $(
                    (PackageInfo {
                        name: $name.to_string(),
                        version: "1".to_string(),
                        description: $desc.to_string(),
                        transforms: vec![
                            $(
                                (Transform {
                                    from: $transform.to_string(),
                                    to: vec![],
                                    description: Some($tdesc.to_string()),
                                    arguments: $arg_info,
                                    variables: $vars.into()
                                }),
                            )*
                        ]
                    }),
                )*
            ]
        }

        pub fn handle_native<T, U>(
            ctx: &mut Context<T, U>,
            package_name: &str,
            node_name: &str, // name of module or parent
            element: &Element,
            args: HashMap<String, ArgValue>,
            output_format: &OutputFormat
        ) -> Result<Element, CoreError> {
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
                                    id,
                                } => $handler(ctx, body, args, *inline, output_format, id),
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
        pub fn load_standard_packages(mgr: &mut PackageStore, #[cfg(feature = "native")] engine: &Engine)
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
        pub fn load_standard_packages(mgr: &mut PackageStore, #[cfg(feature = "native")] engine: &Engine)
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
        pub fn load_standard_packages(_: &mut PackageStore, #[cfg(feature = "native")] _: &Engine)
            -> Result<(), CoreError> {
            Ok(())
        }
    };
}

pub(crate) use define_native_packages;
pub(crate) use define_standard_package_loader;
