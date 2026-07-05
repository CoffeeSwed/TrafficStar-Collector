mod test;
use std::{path::{PathBuf}, sync::Arc};

use once_cell::sync::OnceCell;
use tempdir::TempDir;
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger::serror;
use trafficstar_logger_macro::StructLoggerName;

use crate::TorInterfaceConfig;

#[derive(StructLoggerName)]
pub struct TorDevice{
    pub reservation : Arc<TorInterfaceConfig>,
    pub state_dir : TempDir,
    pub cache_dir : PathBuf,
}

static CACHEDIRECTORYCOPY: OnceCell<Option<Arc<TempDir>>> = OnceCell::new();
impl TorDevice{
    fn cache_dir() -> Result<Arc<TempDir>,TrafficStarError> {
        if let Some(res) = CACHEDIRECTORYCOPY.get_or_init(|| {
            match TempDir::new("trafficstar_tor_cachedir"){
                Ok(dir) => {
                    Some(Arc::new(dir))
                },
                Err(err) => {
                    serror!("Couldn't create cachedir for TrafficStar Tor Device, using a unique tempdir for each instance instead! Error : {}",err);
                    None
                },
            }
        }).clone(){
            Ok(res)
        }else{
           match TempDir::new("trafficstar_tor_cachedir"){
                Ok(dir) => {
                    Ok(Arc::new(dir))
                },
                Err(err) => {
                    Err(format!("Couldn't create tempdir for cache directory, error : {}",err).into())
                },
            } 
        }
    }

    pub async fn new(reservation : Arc<TorInterfaceConfig>) -> Result<Arc<Self>, TrafficStarError>{
        let state_directory = Arc::new(TempDir::new("trafficstar_tor_statedir")?);
        let cache_dir = Self::cache_dir()?;
        

        Err("Unknown error occured!".into())
    }
}