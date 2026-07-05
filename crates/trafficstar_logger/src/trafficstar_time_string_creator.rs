pub trait TrafficStarTimeStringCreator{
     
    fn get_time_string(&self, days : u128, hours : u128, minutes : u128, seconds : u128, ms : u128) -> String;
}

#[derive(Default)]
pub struct DefaultTrafficStarTimeStringCreator{}


impl TrafficStarTimeStringCreator for DefaultTrafficStarTimeStringCreator{
    fn get_time_string(&self, days : u128, hours : u128, minutes : u128, seconds : u128, _ms : u128) -> String {
        if days > 0{
            return format!("{:04}:{:02}:{:02}:{:02}",days, hours % 24, minutes % 60,seconds % 60)
        }
        if hours > 0{
            return format!("{:02}:{:02}:{:02}",hours % 24, minutes % 60,seconds % 60)
        }

        
        format!("{:02}:{:02}", minutes % 60,seconds % 60)
    }
}