// use envfile::EnvFile;
// use std::path::Path;

// pub trait IConfig {
//     fn get_config_with_key(&self, key: &str) -> String;
// }

// pub struct Config {
//     pub path: String,
// }

// impl IConfig for Config {
//     fn get_config_with_key(&self, key: &str) -> String {
//         let env = EnvFile::new(Path::new(&self.path)).unwrap();
//         env.get(key).unwrap().to_string()
//     }
// }
