use anyhow::Result;
use std::{fs, path::PathBuf};

use heck::ToUpperCamelCase;

pub struct CSProject {
    name: String,
    dir: PathBuf,
    aot: bool,
    clean_targets: bool,
    world_name: String,
}

impl CSProject {
    pub fn new(dir: PathBuf, name: &str, world_name: &str) -> CSProject {
        CSProject {
            name: name.to_string(),
            dir,
            aot: false,
            clean_targets: false,
            world_name: world_name.to_string(),
        }
    }

    pub fn generate(&self) -> Result<()> {
        let name = &self.name;
        let world = &self.world_name.replace("-", "_");
        let snake_world = world.to_upper_camel_case();
        let camel = snake_world.to_upper_camel_case();

        fs::write(
            self.dir.join("rd.xml"),
            format!(
                r#"<Directives xmlns="http://schemas.microsoft.com/netfx/2013/01/metadata">
            <Application>
                <Assembly Name="{name}">
                </Assembly>
            </Application>
        </Directives>"#
            ),
        )?;

        let mut csproj = format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">
    
        <PropertyGroup>
            <TargetFramework>net8.0</TargetFramework>
            <LangVersion>preview</LangVersion>
            <RootNamespace>{name}</RootNamespace>
            <ImplicitUsings>enable</ImplicitUsings>
            <Nullable>enable</Nullable>
            <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
        </PropertyGroup>
        
        <PropertyGroup>
            <PublishTrimmed>true</PublishTrimmed>
            <AssemblyName>{name}</AssemblyName>
        </PropertyGroup>
        <ItemGroup>
          <NativeLibrary Include=\"{world}_component_type.o\" />
          <NativeLibrary Include=\"$(MSBuildProjectDirectory)/{camel}_cabi_realloc.o\" />
   
        </ItemGroup>

        <ItemGroup>
            <RdXmlFile Include=\"rd.xml\" />
        </ItemGroup>
        "
        );

        if self.aot {
            //TODO: Is this handled by the source generator? (Temporary just to test with numbers and strings)
            csproj.push_str(
                r#"
                <ItemGroup>
                    <CustomLinkerArg Include="-Wl,--export,_initialize" />
                    <CustomLinkerArg Include="-Wl,--no-entry" />
                    <CustomLinkerArg Include="-mexec-model=reactor" />
                </ItemGroup>
   
                <ItemGroup>
                    <PackageReference Include="Microsoft.DotNet.ILCompiler.LLVM" Version="9.0.0-*" />
                    <PackageReference Include="runtime.win-x64.Microsoft.DotNet.ILCompiler.LLVM" Version="9.0.0-*" />
                </ItemGroup>

                <Target Name="CheckWasmSdks">
                    <Error Text="Emscripten not found, not compiling to WebAssembly. To enable WebAssembly compilation, install Emscripten and ensure the EMSDK environment variable points to the directory containing upstream/emscripten/emcc.bat"
                        Condition="'$(EMSDK)' == ''" />
                </Target>
                "#,
            );

            csproj.push_str(&format!("
              <Target Name=\"CompileCabiRealloc\" BeforeTargets=\"IlcCompile\" DependsOnTargets=\"CheckWasmSdks\" 
                Inputs=\"$(MSBuildProjectDirectory)/{camel}_cabi_realloc.c\"
                Outputs=\"$(MSBuildProjectDirectory)/{camel}_cabi_realloc.o\"
                >
                <Exec Command=\"emcc.bat &quot;$(MSBuildProjectDirectory)/{camel}_cabi_realloc.c&quot; -c -o &quot;$(MSBuildProjectDirectory)/{camel}_cabi_realloc.o&quot;\"/>
              </Target>
            "
            ));

            fs::write(
                self.dir.join("nuget.config"),
                r#"<?xml version="1.0" encoding="utf-8"?>
            <configuration>
                <config>
                    <add key="globalPackagesFolder" value=".packages" />
                </config>
                <packageSources>
                <!--To inherit the global NuGet package sources remove the <clear/> line below -->
                <clear />
                <add key="nuget" value="https://api.nuget.org/v3/index.json" />
                <add key="dotnet-experimental" value="https://pkgs.dev.azure.com/dnceng/public/_packaging/dotnet-experimental/nuget/v3/index.json" />
                <!--<add key="dotnet-experimental" value="C:\github\runtimelab\artifacts\packages\Debug\Shipping" />-->
              </packageSources>
            </configuration>"#,
            )?;
        }

        if self.clean_targets {
            let mut wasm_filename = self.dir.join(name);
            wasm_filename.set_extension("wasm");
            // In CI we run out of disk space if we don't clean up the files, we don't need to keep any of it around.
            csproj.push_str(&format!(
                "<Target Name=\"CleanAndDelete\"  AfterTargets=\"Clean\">
                <!-- Remove obj folder -->
                <RemoveDir Directories=\"$(BaseIntermediateOutputPath)\" />
                <!-- Remove bin folder -->
                <RemoveDir Directories=\"$(BaseOutputPath)\" />
                <RemoveDir Directories=\"{}\" />
                <RemoveDir Directories=\".packages\" />
            </Target>",
                wasm_filename.display()
            ));
        }

        csproj.push_str(
            r#"</Project>
            "#,
        );

        fs::write(self.dir.join(format!("{camel}.csproj")), csproj)?;

        Ok(())
    }

    pub fn aot(&mut self) {
        self.aot = true;
    }

    pub fn clean(&mut self) {
        self.clean_targets = true;
    }
}
