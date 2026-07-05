use std::fmt::Display;
pub type TrafficStarEnumError = dyn TrafficStarEnumErrorTrait + Send + Sync;

pub trait TrafficStarEnumErrorTrait{
    fn enum_name(&self) -> &str;
    fn enum_variant(&self) -> &str;
    fn enum_message(&self) -> Option<String>;
}


impl<T> TrafficStarEnumErrorTrait for T where T : strum::EnumMessage{
    fn enum_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn enum_variant(&self) -> &str {
        let seralizations = self.get_serializations();
        if !seralizations.is_empty(){
            seralizations[0]
        }else{
            "-"
        } 
    }

    fn enum_message(&self) -> Option<String> {
        if let Some(msg) = self.get_message(){
            return Some(msg.to_string())
        }
        if let Some(msg) = self.get_detailed_message(){
            return Some(msg.to_string())
        }
        None
    }
}


impl Display for dyn TrafficStarEnumErrorTrait{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enum_message = self.enum_message().unwrap_or_default();
        if enum_message.is_empty(){
            write!(f,"[{}::{}]",self.enum_name(),self.enum_variant())
        }else{
            write!(f,"[{}::{}{{Message = {{{}}}}}]",self.enum_name(),self.enum_variant(),enum_message)
        }
    }
}