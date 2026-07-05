use std::fmt::Write;

use colored::{Color, ColoredString, Colorize};
use log::LevelFilter;

use crate::trafficstar_logger::TrafficStarLoggerNick;

pub struct TrafficStarLoggerRecordOutput{
    pub prefix_colored_target : String,
    pub prefix_uncolored : String,
    pub lines_content : Vec<String>
}

impl TrafficStarLoggerRecordOutput{
    const TARGET_FILTER : &str  = "!";

    pub fn get_filtered_target<'a>(record : &log::Record<'a>) -> Option<&'a str>{
        if record.target().starts_with(Self::TARGET_FILTER){
            Some(record.target().strip_prefix(Self::TARGET_FILTER).unwrap())
        }else{
            let target : Vec<&str> = record.target().split("::").collect();
            if target.len() >= 2{
                Some(target[target.len()-2])
            }else{
                Some(target[0])
            }
        }
    }

    pub fn get_record_prefix_string(record : &log::Record<'_>) -> String{
        let level = match record.level().to_level_filter() {
            LevelFilter::Off => {
                ""
            }
            LevelFilter::Warn => {
                "WAR"
            }
            LevelFilter::Debug => {
                "DEB"
            }
            LevelFilter::Error => {
                "ERR"
            }
            LevelFilter::Trace => {
                "TRA"
            }
            LevelFilter::Info => {
                "INF"
            }
        };
        if let Some(target) = Self::get_filtered_target(record){
            format!("[{}][{}]",level,target)
        }
        else{
            format!("[{}]",level)
        }
    }

    pub fn get_record_prefix_colored(record : &log::Record<'_>, target_color : Color) -> ColoredString{
        let level = record.level().to_level_filter();
        
        let mut res =  format!("[{}]",match level {
            LevelFilter::Off => {
                "".to_string().into()
            }
            LevelFilter::Warn => {
                "WAR".magenta()
            }
            LevelFilter::Debug => {
                "DEB".bright_purple()
            }
            LevelFilter::Error => {
                "ERR".red()
            }
            LevelFilter::Trace => {
                "TRA".cyan()
            }
            LevelFilter::Info => {
                "INF".white()
            }
        });
        if let Some(target) = Self::get_filtered_target(record){
            res = res+ "["+&format!("{}",target.color(target_color))+"]";
        }
        res.into()
    }
    

    pub fn get_nick_string(nick : Option<TrafficStarLoggerNick>) -> String{
        let mut res = "".to_string();
        if let Some(nicks) = nick{
            for nick in nicks.nicks{
                res+="[";
                res+=&nick;
                res+="]";
            }
        }
        res
    }

    pub fn get_nick_string_colored(nick : Option<TrafficStarLoggerNick>) -> String{
        let mut res = "".to_string();
        if let Some(nicks) = nick{
            for nick in nicks.nicks{
                res+="[";
                res+=&nick.green().to_string();
                res+="]";
            }
        }
        res
    }

    const TAP_REPLACEMENT : &str = "        ";

    fn wrap_lines(string : &str, available : usize , push_to : &mut Vec<String>){
        let prefix_width = unicode_width::UnicodeWidthStr::width(Self::TAP_REPLACEMENT)-1;
        let options = textwrap::Options::new(match available > prefix_width{
            true => available-prefix_width,
            false => available,
        });
        let mut lines =  textwrap::wrap(string, options.clone());
        if !lines.is_empty(){
            push_to.push(format!("⏎\t{}",lines.remove(0).trim_ascii_end()));
            if !lines.is_empty(){
                Self::wrap_lines(&lines.join(" "), available, push_to);
            }
        }

    }
    
    ///Default target_color is Color::BrightCyan
    pub fn new(record: &log::Record<'_>, nicks : Option<TrafficStarLoggerNick>,time_string : &str, max_width : usize, target_color : Option<Color>)
     -> Self
    {
        
        let target_color = target_color.unwrap_or(Color::Cyan);
        let mut messages: Vec<String> = vec!["".to_string()];
        let _res = write!(&mut messages[0], "{}", record.args());
        messages[0] = messages[0].replace("\t", Self::TAP_REPLACEMENT);
        if messages[0].contains("\n") {
            for message in messages.pop().unwrap().split('\n'){
                messages.push(message.to_string());
            }
        }
         let prefix_colored : String = 
            format!("[{}]{}{} ",time_string.bright_cyan(), Self::get_record_prefix_colored(record, target_color),Self::get_nick_string_colored(nicks.clone()));
        let prefix_uncolored : String = 
        format!("[{}]{}{} ", time_string,Self::get_record_prefix_string(record), Self::get_nick_string(nicks.clone()));
        
        let available = match max_width > prefix_uncolored.len()+ 20 {
            true => max_width - prefix_uncolored.len(),
            false => 128,
        };
        

        let mut result = Self{
            lines_content : vec![],
            prefix_colored_target: prefix_colored,
            prefix_uncolored,
        };

        let options = textwrap::Options::new(available);

        for message in messages{
            let mut lines =  textwrap::wrap(&message, options.clone());
            if !lines.is_empty(){
                result.lines_content.push(lines.remove(0).trim_ascii_end().to_string().replace(Self::TAP_REPLACEMENT, "\t"));
                if !lines.is_empty(){
                    Self::wrap_lines(&lines.join(" "), available, &mut result.lines_content);
                }
            }
        }
       
        result
    }
}