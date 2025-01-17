use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File},
    io::Write,
};

#[derive(serde::Deserialize, Debug)]
pub struct BuildConfig {
    /// The amount of binaries to be built. Will assert that every vec in constants is of this size.
    pub num_binaries: usize,
    constants: BuildConstants,
}

#[derive(serde::Deserialize, Debug)]
pub struct BuildConstants {
    integral: HashMap<String, Vec<i32>>,
    string: HashMap<String, Vec<String>>,
}

impl BuildConfig {
    pub fn load_build_config(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(file_path)?;
        let config: Result<Self, _> = toml::from_str(&contents);

        match config {
            Ok(config) => Ok(config),
            Err(err) => {
                println!("TOML Parse error: {err}");
                Err(Box::new(err))
            }
        }
    }

    pub fn generate_constants_rs(
        &self,
        binary_index: usize,
        file_path: &str,
    ) -> Result<(), Box<dyn Error>> {
        println!("Generating constants for binary_index={binary_index}");

        if binary_index >= self.num_binaries {
            return Err(format!(
                "Error, binary_index={binary_index} is out of range, num_binaries={}",
                self.num_binaries
            )
            .into());
        }

        let mut string = String::new();

        for (name, value) in self.constants.integral.iter() {
            let line = format!(
                "pub const {}: i32 = {};\n",
                name,
                value.get(binary_index).unwrap_or_else(|| panic!(
                    "Expect every vec to be of at least length {}",
                    self.num_binaries
                ))
            );

            string.push_str(&line);
        }

        for (name, value) in self.constants.string.iter() {
            let line = format!(
                "pub const {}: &str = \"{}\";\n",
                name,
                value.get(binary_index).unwrap_or_else(|| panic!(
                    "Expect every vec to be of at least length {}",
                    self.num_binaries
                ))
            );

            string.push_str(&line);
        }

        println!("{}", string);

        let mut file = File::create(file_path)?;
        file.write_all(string.as_bytes())?;

        Ok(())
    }
}
