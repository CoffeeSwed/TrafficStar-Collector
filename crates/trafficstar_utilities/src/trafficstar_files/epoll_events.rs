use nix::sys::epoll::{EpollEvent, EpollFlags};

const FLAGS_ENUMS : [EpollFlags;15] = [
        EpollFlags::EPOLLIN,
        EpollFlags::EPOLLPRI,
        EpollFlags::EPOLLOUT,
        EpollFlags::EPOLLRDNORM,
        EpollFlags::EPOLLRDBAND,
        EpollFlags::EPOLLWRNORM,
        EpollFlags::EPOLLWRBAND,
        EpollFlags::EPOLLMSG,
        EpollFlags::EPOLLERR,
        EpollFlags::EPOLLHUP,
        EpollFlags::EPOLLRDHUP,
        EpollFlags::EPOLLEXCLUSIVE,
        EpollFlags::EPOLLWAKEUP,
        EpollFlags::EPOLLONESHOT,
        EpollFlags::EPOLLET,
    ];

const FLAGS_STRINGS : [&str;15] = [
        "EPOLLIN",
        "EPOLLPRI",
        "EPOLLOUT",
        "EPOLLRDNORM",
        "EPOLLRDBAND",
        "EPOLLWRNORM",
        "EPOLLWRBAND",
        "EPOLLMSG",
        "EPOLLERR",
        "EPOLLHUP",
        "EPOLLRDHUP",
        "EPOLLEXCLUSIVE",
        "EPOLLWAKEUP",
        "EPOLLONESHOT",
        "EPOLLET",
    ];

#[allow(dead_code)]
pub fn get_events(event : &EpollEvent) -> Vec<&'static EpollFlags>{
    
    let mut res = Vec::new();
    for flag in &FLAGS_ENUMS{
        if event.events().contains(*flag){
            res.push(flag);
        }
    }

    res
}

pub fn get_events_string(event : &EpollEvent) -> Vec<&'static str>{
    
    let mut res = Vec::new();
    for (index,flag) in FLAGS_ENUMS.iter().enumerate(){
        if event.events().contains(*flag){
            res.push(FLAGS_STRINGS[index]);
        }
    }

    res
}

#[allow(dead_code)]
pub fn get_event_string(event : &EpollFlags) -> &'static str{
    let index = FLAGS_ENUMS.iter().position(|e| *e == *event).unwrap();
    FLAGS_STRINGS[index]
}