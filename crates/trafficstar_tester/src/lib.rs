use std::{env, fs::{create_dir_all, remove_dir_all}, io::Error, path::PathBuf};

/// Creates temp dir if it doesn't exist and returns it. else, return it. 
pub fn create_temp_dir(name : &str) -> Result<PathBuf, Error>{
    let mut dir = env::temp_dir();
    dir = dir.join(name);
    if dir.exists(){
        Ok(dir)
    }else{
        match create_dir_all(dir.clone()){
            Ok(_v) => Ok(dir),
            Err(e) => Err(e),
        }
    }
}


/// Removes temp dir if it exists and makes a new one to return, otherwise, creates it and returns it.
pub fn replace_create_temp_dir(name : &str) -> Result<PathBuf, Error>{
    let mut dir = env::temp_dir();
    dir = dir.join(name);
    if dir.exists(){
        if let Some(error) = remove_temp_dir(name){
            Err(error)
        }else{
            create_temp_dir(name)
        }
    }else{
        create_temp_dir(name)
    }
}

/// Removes temp dir if it exists, otherwise does not nothing!
pub fn remove_temp_dir(name : &str) -> Option<Error>{
    let mut dir = env::temp_dir();
    dir = dir.join(name);
    if dir.exists(){
        if dir.is_dir(){
            remove_dir_all(dir).err()
        }else{
            Some(std::io::Error::new(std::io::ErrorKind::NotADirectory, "Couldn't delete temp directory, found a file instead of a directory!"))
        }
    }else{
        Some(std::io::Error::new(std::io::ErrorKind::NotFound, "Cant delete temp directory since it does not exist!"))
    }
}