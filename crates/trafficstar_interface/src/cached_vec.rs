use std::{fs::File, io::{ErrorKind, Read, Write}, ops::{Deref, DerefMut}, path::PathBuf, str::FromStr};

use serde::{Serialize, de::DeserializeOwned};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::{trafficstar_logger_trait::TrafficStarStructName};

pub struct CachedVec<T> where T : Serialize + DeserializeOwned + PartialEq{
    path : PathBuf,
    path_temp : PathBuf,
    items : Vec<T>
}

impl<T> TrafficStarStructName for CachedVec<T> where T : Serialize + DeserializeOwned + PartialEq{
    fn struct_name() -> &'static str {
        "CachedVec"
    }
}

impl<T> CachedVec<T> where T : Serialize + DeserializeOwned + PartialEq{
     ///Replaces current state with the one from the cache!
    fn load_cache_temp(&mut self) -> Result<(), TrafficStarError>{
        let mut file = File::open(self.path.clone().join(self.path_temp.as_path()))?;
        let mut buffer : Vec<u8> = Vec::with_capacity(1024);
        let mut buffer_usize = [0_u8;size_of::<usize>()];
        
        self.items.clear();

        loop{
            buffer.resize(size_of::<usize>(), 0);
            match file.read_exact(&mut buffer_usize){
                Ok(_) => {
                    let size = usize::from_le_bytes(buffer_usize);
                    buffer.resize(size, 0);
                    file.read_exact(&mut buffer)?;
                    match rmp_serde::decode::from_slice::<T>(&buffer){
                        Ok(v) => {
                            self.items.push(v);
                        },
                        Err(err) => return Err(format!("Decode error : {}",err).into()),
                    };

                },
                Err(err) => {
                    if err.kind() == ErrorKind::UnexpectedEof{
                        break;
                    }
                    return Err(format!("Unexpected error occured, error : {}",err).into())
                },
            };
        }
        
        drop(file);
        Ok(())
    }
    
    ///Replaces current state with the one from the cache!
    pub fn load_cache(&mut self) -> Result<(), TrafficStarError>{
        let mut file = File::open(self.path.as_path())?;
        let mut buffer : Vec<u8> = Vec::with_capacity(1024);
        let mut buffer_usize = [0_u8;size_of::<usize>()];
        
        self.items.clear();

        loop{
            buffer.resize(size_of::<usize>(), 0);
            match file.read_exact(&mut buffer_usize){
                Ok(_) => {
                    let size = usize::from_le_bytes(buffer_usize);
                    buffer.resize(size, 0);
                    file.read_exact(&mut buffer)?;
                    match rmp_serde::decode::from_slice::<T>(&buffer){
                        Ok(v) => {
                            self.items.push(v);
                        },
                        Err(err) => return Err(format!("Decode error : {}",err).into()),
                    };

                },
                Err(err) => {
                    if err.kind() == ErrorKind::UnexpectedEof{
                        break;
                    }
                    return Err(format!("Unexpected error occured, error : {}",err).into())
                },
            };
        }
        
        drop(file);
        Ok(())
    }

    fn write_cache_temp(&self) -> Result<(), TrafficStarError>{
        let mut file = File::create(self.path_temp.as_path())?;
        for item in &self.items{
            let data = match rmp_serde::encode::to_vec(item){
                Ok(v) => v,
                Err(err) => return Err(format!("Encode error : {}!",err).into()),
            };
            file.write_all(&data.len().to_le_bytes())?;
            file.write_all(&data)?;
        }
        drop(file);
        Ok(())
    }

    ///Write down the current state to the cache!
    pub fn write_cache(&self) -> Result<(), TrafficStarError>{
        self.write_cache_temp()?;
        let mut file = File::create(self.path.as_path())?;
        for item in &self.items{
            let data = match rmp_serde::encode::to_vec(item){
                Ok(v) => v,
                Err(err) => return Err(format!("Encode error : {}!",err).into()),
            };
            file.write_all(&data.len().to_le_bytes())?;
            file.write_all(&data)?;
        }
        drop(file);
        Ok(())
    }
    
    pub fn new(path : PathBuf) -> Result<Self, TrafficStarError>{
        if !path.exists(){
            drop(File::create(path.as_path())?);
        }
        let path_temp_extenstion = match PathBuf::from_str("temp"){
            Ok(v) => v,
            Err(err) => return Err(format!("Could not derive a temp file, error {}",err).into()),
        };
        let mut path_temp = path.clone();
        path_temp.add_extension(path_temp_extenstion);
        let mut res = Self{
            path,
            path_temp,
            items : Vec::new()
        };
        if res.load_cache_temp().is_ok(){
            res.write_cache()?;
        }else{
            res.load_cache()?;
        }
        Ok(res)
    }
}

impl<T> Deref for CachedVec<T>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for CachedVec<T>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl<T> IntoIterator for CachedVec<T>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

// Or for &CachedVec<T>
impl<'a, T> IntoIterator for &'a CachedVec<T>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}