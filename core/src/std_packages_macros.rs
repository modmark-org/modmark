macro_rules! define_native_packages {
    ($ns:ident; $($name:expr => { $($transform:expr, $arg_info:expr => $handler:ident,)* };)*) => {
        impl $ns {
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
                                    to: vec![OutputFormat("NATIVE".to_string())],
                                    arguments: $arg_info
                                }),
                                )*
                            ]
                        }),
                    )*
                ]
            }

            pub fn handle_native(
                ctx: &mut Context,
                package_name: &str,
                node_name: &str, // name of module or parent
                element: &Element,
                args: HashMap<String, String>,
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
}

macro_rules! define_standard_package_loader {
    ($ns:ident; $($name:expr,)*) => {
        impl $ns {
            pub fn load_standard_packages(ctx: &mut Context) -> Result<(), CoreError>{
                $(
                    ctx.load_package_from_wasm(
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
        }
    }
}

pub(crate) use define_native_packages;
pub(crate) use define_standard_package_loader;
