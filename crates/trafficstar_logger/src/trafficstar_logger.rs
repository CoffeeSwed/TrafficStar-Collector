
use std::{
    cell::Cell, collections::HashMap, fs::File, io::{Stdout, Write, stdout},ops::Sub, path::Path, sync::{Arc, Mutex, MutexGuard}, time::Instant
};

use colored::Color;
use indicatif::{MultiProgress};
use once_cell::sync::OnceCell;
use termion::screen::{AlternateScreen, IntoAlternateScreen};


use crate::{trafficstar_logger_record::TrafficStarLoggerRecordOutput, trafficstar_time_string_creator::{DefaultTrafficStarTimeStringCreator, TrafficStarTimeStringCreator}};

#[derive(Clone)]
pub struct TrafficStarLoggerNick{
    pub nicks : Vec<String>,
}

impl From<Vec<&str>> for TrafficStarLoggerNick{
    fn from(value: Vec<&str>) -> Self {
        let mut nicks = Vec::<String>::with_capacity(value.len());
        for string in value{
            nicks.push(string.to_string());
        }
        Self { nicks }
    }
}

impl From<Vec<String>> for TrafficStarLoggerNick{
    fn from(value: Vec<String>) -> Self {
        Self { nicks : value }
    }
}

pub struct TrafficStarLogger {
    pub output_file : Option<Arc<File>>,
    progress_bars: Option<Arc<MultiProgress>>,
    start_time : Instant,
    pub time_string_creator : Arc<Box<dyn TrafficStarTimeStringCreator + Sync + Send>>,
    pub screen : Arc<Option<AlternateScreen<Stdout>>>,
    pub muted : HashMap<String, Vec<log::Level>>,
    pub targetcolors : HashMap<String, Color>,
    pub lock : Arc<Mutex<u8>>,
}



///SAFETY
#[allow(dead_code, static_mut_refs)]
impl TrafficStarLogger{
    const BUFFER_LINES : usize = 128;
    thread_local! {
        static NICK: Cell<Option<TrafficStarLoggerNick>> = const { Cell::new(None) };
    }
    thread_local! {
        static NICKCHILD: Cell<Option<TrafficStarLoggerNick>> = const { Cell::new(None) };
    }

    #[allow(unsafe_code)]
    ///Static behaviour required by logger.<br>
    ///SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn get_singleton() -> &'static mut TrafficStarLogger {
        static mut INSTANCE: OnceCell<TrafficStarLogger> = OnceCell::new();
        // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
        unsafe{
            if let Some(instance) = INSTANCE.get_mut(){
                instance
            }else{
                INSTANCE.get_or_init(|| {
                    TrafficStarLogger{
                        output_file : None,
                        progress_bars : None,
                        start_time : Instant::now(),
                        time_string_creator : Arc::new(Box::new(DefaultTrafficStarTimeStringCreator::default())),
                        screen : None.into(),
                        muted : HashMap::new(),
                        targetcolors : HashMap::new(),
                        lock : Arc::new(Mutex::new(0)),
                    }
                }) ;
                Self::get_singleton()
            }
        }
        

        
    }
    
    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn get_screen() -> Arc<Option<AlternateScreen<Stdout>>>{
        Self::get_singleton().screen.clone()
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn use_screen(){
        if Self::get_screen().is_none(){
            let mut screen : Option<AlternateScreen<Stdout>> = None;
            if let Ok(altscreen) = stdout().into_alternate_screen(){
                screen = Some(altscreen);
            }
            Self::get_singleton().screen = screen.into();
        }
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn drop_screen(){
        Self::get_singleton().screen = None.into();
    }
    
    

    pub fn set_nick_thread(nick : Option<TrafficStarLoggerNick>){
       TrafficStarLogger::NICK.set(nick);
    }

    pub fn add_nick_thread(nick : String){
        let nick_copy = TrafficStarLogger::NICK.take();
        if let Some(mut nicks) = nick_copy{
            nicks.nicks.push(nick.clone());
            TrafficStarLogger::NICK.set(Some(nicks));
        }else{
            
            TrafficStarLogger::NICK.set(Some(TrafficStarLoggerNick { nicks: vec![nick] }));
        }
    }

    pub fn set_last_nick(nick : Option<String>){
        let mut nick_copy = TrafficStarLogger::get_nick_thread().unwrap_or(
            TrafficStarLoggerNick { nicks: vec![] }
        );
        if nick_copy.nicks.is_empty(){
            if let Some(nick) = nick{
                Self::add_nick_thread(nick);
            }
        }else {
            nick_copy.nicks.pop();
            if let Some(nick) = nick{
                nick_copy.nicks.push(nick);
            }
            Self::set_nick_thread(Some(nick_copy));
        }
    }

    pub fn remove_nick_thread(){
        TrafficStarLogger::set_nick_thread(None);
    }

    pub fn set_threadhook_nick(nick : Option<TrafficStarLoggerNick>){
        TrafficStarLogger::NICKCHILD.set(nick.clone());

        std::thread::add_spawn_hook(|_| {
            let value = TrafficStarLogger::NICKCHILD.take().clone(); // This will run in the parent (spawning) thread.
            TrafficStarLogger::NICKCHILD.set(value.clone());
            move || {
                TrafficStarLogger::NICKCHILD.set(value.clone());
                TrafficStarLogger::set_nick_thread(value.clone());
                TrafficStarLogger::set_threadhook_nick(value.clone());
            }
        });
       
    }

    pub fn get_nick_thread() -> Option<TrafficStarLoggerNick>{
        let copy = TrafficStarLogger::NICK.take();
        if let Some(nick) = copy{
            TrafficStarLogger::NICK.set(Some(nick.clone()));
            return Some(nick);
        }

        TrafficStarLogger::NICK.set(copy);
        
        None
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn get_progress_bars() -> Option<Arc<MultiProgress>> {
        Self::get_singleton().progress_bars.clone()
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn set_progress_bars(bar: Arc<MultiProgress>) {
        Self::get_singleton().progress_bars = Some(bar);
    }

    
    pub fn lock() -> MutexGuard<'static, u8>{
        Self::get_singleton().lock.lock().unwrap()
    }

    pub fn get_time_string() -> String{
        let milliseconds = Instant::now().sub(Self::get_singleton().start_time).as_millis();
        let seconds = milliseconds/1000;
        let minutes = seconds / 60;
        let hours = seconds / 3600;
        let days = hours / 24;
        
        Self::get_singleton().time_string_creator.get_time_string(days, hours % 24, minutes % 60, seconds % 60, milliseconds % 1000)
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn mute(target : String, level : log::Level){
        let instance = Self::get_singleton();
        let part = instance.muted.entry(target).or_default();
        if !part.contains(&level){
            part.push(level);
        }
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    pub fn unmute(target : String, level : log::Level){
        let instance = Self::get_singleton();
        if let Some(entry) = instance.muted.get_mut(&target){
            if let Some(index) = entry.iter().position(|x| *x == level){
                entry.remove(index);
            }
            if entry.is_empty(){
                instance.muted.remove(&target);
            }
        }
    }

    pub fn set_target_color(target : &str, color : Option<Color>){
        let instance = Self::get_singleton();
        if let Some(color) = color{
            instance.targetcolors.insert(target.to_string(), color);
        }else{
            instance.targetcolors.remove(target);
        }
    }

    pub fn get_target_color(target : &str) -> Option<Color>{
        let instance = Self::get_singleton();
        instance.targetcolors.get(target).copied()
    }

    // SAFETY: CALLER IS RESPONSIBLE TO MAKE LOCK CALLS IF NECESSARY, FOR PERFORMANCE REASONS
    ///Replace normals to True
    pub fn set_output_file(path : Option<&Path>, replace : Option<bool>) -> Result<(),std::io::Error>{
        if let Some(path) = path{
            let file = match replace.unwrap_or(true){
                true => {
                    {
                        match std::fs::remove_file(path){
                            Ok(_) => {},
                            Err(err) => {
                                if err.kind() != std::io::ErrorKind::NotFound{
                                    return Err(err);
                                }
                            },
                        }
                        std::fs::File::create_new(path)
                    }
                },
                false => std::fs::File::create(path),
            }?;
            Self::get_singleton().output_file = Some(Arc::new(file));
            Ok(())
        }else{
            Self::get_singleton().output_file = None;
            Ok(())
        }
    }
    
}

impl log::Log for TrafficStarLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        if let Some(entry) = Self::get_singleton().muted.get(metadata.target())
        && entry.contains(&metadata.level())
        {
            false
        }else{
            true
        }
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()){
            let (width, _) = termion::terminal_size().unwrap_or((128,32));
            let record = TrafficStarLoggerRecordOutput::new(
                record, Self::get_nick_thread(), &Self::get_time_string(),
                width as usize,
                Self::get_target_color(record.target())
            );
            for line in record.lines_content{
                if let Some(mut file) = self.output_file.clone(){
                    let _ = file.write_all(record.prefix_uncolored.as_bytes());
                    let _ = file.write_all(line.as_bytes());
                    let _ = file.write_all("\n".as_bytes());
                }
                if let Some(progressbar) = Self::get_progress_bars(){
                    let _ = progressbar.println(format!("{}{}",record.prefix_colored_target, line));
                }else{
                    println!("{}{}",record.prefix_colored_target, line)
                }
            }
        }
    }
    

    fn flush(&self) {
        if let Some(mut file) = self.output_file.clone(){
            let _ = file.flush();
        }
    }
}