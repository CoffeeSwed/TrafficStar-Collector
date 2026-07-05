use std::{path::PathBuf, str::FromStr};

use clap::{Arg, ArgMatches, Command};
use trafficstar_errors::traffic_star_error::TrafficStarError;
use trafficstar_logger_macro::StructLoggerName;

#[derive(StructLoggerName)]
pub struct Configuration{
    pub addr : (String, u16),
    pub interfaces : PathBuf,
    pub storage : PathBuf,
    pub mullvad_generators : Option<PathBuf>,
    pub mullvad_accounts : Option<Vec<String>>,
    pub tor_generators : Option<PathBuf>,
    pub test_session_params : Option<PathBuf>,
    pub directory_prefix : Option<String>,
    pub torun : Option<Vec<String>>,
}

impl Configuration{
    pub fn add_arguments(command : Command) -> Command{
        command
        .arg(Arg::new("addr").long("address").short('a').help("The address to connect to or listen from").value_names(vec!["Address (or interface)","Port"]).value_delimiter(' '))
        .arg(Arg::new("storage").long("storage").short('s').help("Where to store data from!").required(true))
        .arg(Arg::new("interfaces").required(true).long("interfaces").help("Where information about different interfaces are stored"))
        .arg(Arg::new("mullvadparams").long("mullvad-parameters").help("Where parameters for generating mullvad connections are to be read from"))
        .arg(Arg::new("mullvadaccounts").long("mullvad-accounts").value_delimiter(',').help("Mullvad accounts numbers, seperated by comma <,>"))
        .arg(Arg::new("torparams").long("tor-parameters").help("Where parameters for generating tor connections are to be read from"))
        .arg(Arg::new("testparams").long("test-parameters").help("Where parameters for generating tests data to be read from"))
        .arg(Arg::new("directoryprefix").long("directory-prefix").help("The directory prefix to give for the stored data on the server side"))
        .arg(Arg::new("tests").long("run-tests").help("Select which tests to run based on their name.").value_delimiter(','))
    }
}

impl TryFrom<ArgMatches> for Configuration{
    type Error = TrafficStarError;

    fn try_from(args: ArgMatches) -> Result<Self, Self::Error> {  
        let addr_ipv4 : Vec<String> = match args.get_many::<String>("addr"){
            Some(v) => v.cloned().collect(),
            None => return Err("Missing required variable addr!".into()),
        };

        let addr_uport = match addr_ipv4.get(1){
            Some(v) => u16::from_str(v).map_err(|e| format!("Invalid port given, parse error : {},{}",e,v))?,
            None => return Err("Missing required variable addr port!".into()),
        };
        
        

        let storage = match args.get_one::<String>("storage"){
            Some(v) => PathBuf::from_str(v).map_err(|_| TrafficStarError::msg("Invalid Storage location given!".into()))?,
            None => return Err("Missing required variable storage!".into()),
        };

        let interfaces = match args.get_one::<String>("interfaces"){
            Some(v) => PathBuf::from_str(v).map_err(|_| TrafficStarError::msg("Invalid Interfaces location given!".into()))?,
            None => return Err(TrafficStarError::msg("Missing required variable interfaces!".into())),
        };

        let mullvadparams = match args.get_one::<String>("mullvadparams"){
            Some(v) => Some(PathBuf::from_str(v).map_err(|_| TrafficStarError::msg("Invalid Mullvad generator location given!".into()))?),
            None => None,
        };

        let mullvadaccounts = args.get_many::<String>("mullvadaccounts").map(|v| v.cloned().collect());

        let torparams = match args.get_one::<String>("torparams"){
            Some(v) => Some(PathBuf::from_str(v).map_err(|_| TrafficStarError::msg("Invalid Tor generator location given!".into()))?),
            None => None,
        };

        let testparams = match args.get_one::<String>("testparams"){
            Some(v) => Some(PathBuf::from_str(v).map_err(|_| TrafficStarError::msg("Invalid Test session generator location given!".into()))?),
            None => None,
        };

        let directory_prefix = args.get_one::<String>("directoryprefix").cloned();

        let tests : Option<Vec<String>> = args.get_many::<String>("tests").map(|v| v.cloned().collect());

        Ok(Self{
            addr : (addr_ipv4[0].clone(), addr_uport),
            storage,
            interfaces,
            mullvad_generators: mullvadparams,
            mullvad_accounts: mullvadaccounts,
            tor_generators: torparams,
            test_session_params: testparams,
            directory_prefix,
            torun : tests
        })
    }
}