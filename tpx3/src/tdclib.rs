//!`tdclib` is a collection of tools to facilitate manipulation and choice of tdcs. Module is built
//!in around `TdcType` enum.


mod tdcvec {
    use crate::errorlib::Tp3ErrorKind;
    use crate::tdclib::TdcType;
    use crate::packetlib::{Packet, PacketEELS as Pack, packet_change};
    use crate::auxiliar::value_types::*;

    pub struct TdcSearch<'a> {
        data: Vec<(TIME, TdcType)>,
        how_many: usize,
        tdc_choosen: &'a TdcType,
        initial_counter: Option<COUNTER>,
        last_counter: u16,
    }

    impl<'a> TdcSearch<'a> {
        pub fn new(tdc_choosen: &'a TdcType, how_many: usize) -> Self {
            TdcSearch{
                data: Vec::new(),
                how_many,
                tdc_choosen,
                initial_counter: None,
                last_counter: 0,
            }
        }

        fn add_tdc(&mut self, packet: &Pack) {
            if let Some(tdc) = TdcType::associate_value_to_enum(packet.tdc_type()) {
                let time = packet.tdc_time_norm();
                self.data.push( (time, tdc) );
                if packet.tdc_type() == self.tdc_choosen.associate_value() {
                    self.last_counter = packet.tdc_counter();
                    self.initial_counter = match self.initial_counter {
                        None => Some(packet.tdc_counter() as COUNTER),
                        Some(val) => Some(val),
                    };
                }
            }
        }

        pub fn check_tdc(&self) -> Result<bool, Tp3ErrorKind> {
            let mut counter = 0;
            for (_time, tdc_type) in &self.data {
                if tdc_type.associate_value() == self.tdc_choosen.associate_value() {counter+=1;}
            }
            if counter>=self.how_many {
                self.check_ascending_order()?;
                Ok(true)
            } else {Ok(false)}
        }

        fn get_timelist(&self, which: &TdcType) -> Vec<TIME> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == which.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }
        
        fn get_auto_timelist(&self) -> Vec<TIME> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }

        fn check_ascending_order(&self) -> Result<(), Tp3ErrorKind> {
            let time_list = self.get_auto_timelist();
            let result = time_list.iter().zip(time_list.iter().skip(1)).find(|(a, b)| a>b);
            if result.is_some() {Err(Tp3ErrorKind::TdcNotAscendingOrder)}
            else {Ok(())}
        }

        pub fn find_high_time(&self) -> Result<TIME, Tp3ErrorKind> {
            let fal_tdc_type = match self.tdc_choosen {
                TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneFallingEdge,
                TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoFallingEdge,
                TdcType::NoTdc => TdcType::NoTdc,
            };

            let ris_tdc_type = match self.tdc_choosen {
                TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneRisingEdge,
                TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoRisingEdge,
                TdcType::NoTdc => TdcType::NoTdc,
            };

            let mut fal = self.get_timelist(&fal_tdc_type);
            let mut ris = self.get_timelist(&ris_tdc_type);
            //let last_fal = fal.pop().expect("Please get at least 01 falling Tdc");
            let last_fal = match fal.pop() {
                Some(val) => val,
                None => return Err(Tp3ErrorKind::TdcBadHighTime),
            };
            let last_ris = match ris.pop() {
                Some(val) => val,
                None => return Err(Tp3ErrorKind::TdcBadHighTime),
            };
            if last_fal > last_ris {
                Ok(last_fal - last_ris)
            } else {
                let new_ris = match ris.pop () {
                    Some(val) => val,
                    None => return Err(Tp3ErrorKind::TdcBadHighTime),
                };
                Ok(last_fal - new_ris)
            }
        }
        
        pub fn find_period(&self) -> Result<TIME, Tp3ErrorKind> {
            let mut tdc_time = self.get_auto_timelist();
            let last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            let before_last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            if last > before_last {
                Ok(last - before_last)
            } else {
                Err(Tp3ErrorKind::TdcBadPeriod)
            }
        }
        
        pub fn get_counter(&self) -> Result<COUNTER, Tp3ErrorKind> {
            let counter = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .count() as COUNTER;
            Ok(counter)
        }

        pub fn get_counter_offset(&self) -> COUNTER {
            self.initial_counter.expect("***Tdc Lib***: Tdc initial counter offset was not found.")
        }

        pub fn get_last_hardware_counter(&self) -> u16 {
            self.last_counter
        }

        pub fn get_lasttime(&self) -> TIME {
            let last_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .last().unwrap();
            last_time
        }

        pub fn get_begintime(&self) -> TIME {
            let begin_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .next().unwrap();
            begin_time
        }

        pub fn search_specific_tdc(&mut self, data: &[u8]) {
            data.chunks_exact(8).for_each(|x| {
                match *x {
                    [84, 80, 88, 51, _, _, _, _] => {},
                    _ => {
                        let packet = Pack {chip_index: 0, data: packet_change(x)[0]};
                        if packet.id() == 6 && self.tdc_choosen.is_same_inputline(packet.tdc_type()) {
                            self.add_tdc(&packet);
                        }
                    },
                };
            });
        }

    }
}


///The four types of TDC's.
pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
    NoTdc,
}

impl Clone for TdcType {
    fn clone(&self) -> TdcType {
        match self {
            TdcType::TdcOneRisingEdge => TdcType::TdcOneRisingEdge,
            TdcType::TdcOneFallingEdge => TdcType::TdcOneFallingEdge,
            TdcType::TdcTwoRisingEdge => TdcType::TdcTwoRisingEdge,
            TdcType::TdcTwoFallingEdge => TdcType::TdcTwoFallingEdge,
            TdcType::NoTdc => TdcType::NoTdc,
        }
        //match self {
    }
}

impl TdcType {
    ///Convenient method. Return value is the 4 bits associated to each TDC.
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
            TdcType::NoTdc => 0,
        }
    }

    fn associate_str(&self) -> String {
        match *self {
            TdcType::TdcOneRisingEdge => String::from("Tdc 01 Rising Edge"),
            TdcType::TdcOneFallingEdge => String::from("Tdc 01 Falling Edge"),
            TdcType::TdcTwoRisingEdge => String::from("Tdc 02 Rising Edge"),
            TdcType::TdcTwoFallingEdge => String::from("Tdc 02 Falling Edge"),
            TdcType::NoTdc => String::from("Tdc Disabled"),
        }
    }
    
    ///Check if a given tdc is from the same input line.
    fn is_same_inputline(&self, check: u8) -> bool {
        match *self {
            TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge if check == 15 || check == 10 => true,
            TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge if check == 14 || check == 11 => true,
            _ => false,
        }
    }

    ///From associate value to enum TdcType.
    pub fn associate_value_to_enum(value: u8) -> Option<TdcType> {
        match value {
            15 => Some(TdcType::TdcOneRisingEdge),
            10 => Some(TdcType::TdcOneFallingEdge),
            14 => Some(TdcType::TdcTwoRisingEdge),
            11 => Some(TdcType::TdcTwoFallingEdge),
            _ => None,
        }
    }
}

use std::time::{Duration, Instant};
use crate::errorlib::Tp3ErrorKind;
use crate::auxiliar::misc::TimepixRead;
use crate::auxiliar::value_types::*;

pub trait TdcControl {
    fn id(&self) -> u8;
    fn upt(&mut self, time: TIME, hard_counter: u16);
    fn counter(&self) -> COUNTER;
    fn time(&self) -> TIME;
    fn period(&self) -> Option<TIME>;
    fn new<T: TimepixRead>(tdc_type: TdcType, sock: &mut T, sp: Option<COUNTER>) -> Result<Self, Tp3ErrorKind> where Self: Sized;
}

#[derive(Copy, Clone, Debug)]
pub struct PeriodicTdcRef {
    tdctype: u8,
    counter: COUNTER,
    counter_offset: COUNTER,
    last_hard_counter: u16,
    counter_overflow: COUNTER,
    begin_time: TIME,
    pub ticks_to_frame: Option<COUNTER>,
    pub begin_frame: TIME,
    pub period: TIME,
    pub high_time: TIME,
    pub low_time: TIME,
    time: TIME,
}

impl TdcControl for PeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: TIME, hard_counter: u16) {
        if hard_counter < self.last_hard_counter {
            self.counter_overflow += 1;
        }
        self.last_hard_counter = hard_counter;
        self.time = time;
        self.counter = self.last_hard_counter as COUNTER + self.counter_overflow * 4096 - self.counter_offset;
        if let Some(spimy) = self.ticks_to_frame {
            if (self.counter / 2) % spimy == 0 {
                self.begin_frame = time;
                
            }
        }
    }

    fn counter(&self) -> COUNTER {
        self.counter
    }

    fn time(&self) -> TIME {
        self.time
    }

    fn period(&self) -> Option<TIME> {
        Some(self.period)
    }

    fn new<T: TimepixRead>(tdc_type: TdcType, sock: &mut T, ticks_to_frame: Option<COUNTER>) -> Result<Self, Tp3ErrorKind> {
        let mut buffer_pack_data = vec![0; 16384];
        let mut tdc_search = tdcvec::TdcSearch::new(&tdc_type, 3);
        let start = Instant::now();

        println!("***Tdc Lib***: Searching for Tdc: {}.", tdc_type.associate_str());
        loop {
            if start.elapsed() > Duration::from_secs(10) {return Err(Tp3ErrorKind::TdcNoReceived)}
            if let Ok(size) = sock.read_timepix(&mut buffer_pack_data) {
                tdc_search.search_specific_tdc(&buffer_pack_data[0..size]);
                if tdc_search.check_tdc()? {break;}
            }
        }
        println!("***Tdc Lib***: {} has been found.", tdc_type.associate_str());
        let _counter = tdc_search.get_counter()?;
        let counter_offset = tdc_search.get_counter_offset();
        let _last_hard_counter = tdc_search.get_last_hardware_counter();
        let begin_time = tdc_search.get_begintime();
        let last_time = tdc_search.get_lasttime();
        let high_time = tdc_search.find_high_time()?;
        let period = tdc_search.find_period()?;
        let low_time = period - high_time;

        let per_ref = Self {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            counter_offset,
            last_hard_counter: 0,
            counter_overflow: 0,
            begin_time,
            begin_frame: begin_time,
            ticks_to_frame,
            period,
            high_time,
            low_time,
            time: last_time,
        };
        println!("***TDC Lib***: Creating a new tdc reference: {:?}.", per_ref);
        Ok(per_ref)
    }
}

impl PeriodicTdcRef {
    pub fn frame(&self) -> COUNTER {
        if let Some(spimy) = self.ticks_to_frame {
            (self.counter / 2) / spimy
        } else {
            0
        }
    }

    pub fn pixel_time(&self, xspim: POSITION) -> TIME {
        self.low_time / xspim as TIME
    }

    pub fn estimate_time(&self) -> TIME {
        (self.counter as TIME / 2) * self.period + self.begin_time
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SingleTriggerPeriodicTdcRef {
    tdctype: u8,
    counter: COUNTER,
    counter_offset: COUNTER,
    last_hard_counter: u16,
    counter_overflow: COUNTER,
    pub begin_frame: TIME,
    pub period: TIME,
    pub time: TIME,
}

impl TdcControl for SingleTriggerPeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: TIME, hard_counter: u16) {
        if hard_counter < self.last_hard_counter {
            self.counter_overflow += 1;
        }
        self.last_hard_counter = hard_counter;
        self.time = time;
        self.counter = self.last_hard_counter as COUNTER + self.counter_overflow * 4096 - self.counter_offset;
    }
    
    fn counter(&self) -> COUNTER {
        self.counter
    }

    fn time(&self) -> TIME {
        self.time
    }

    fn period(&self) -> Option<TIME> {
        Some(self.period)
    }

    fn new<T: TimepixRead>(tdc_type: TdcType, sock: &mut T, _: Option<COUNTER>) -> Result<Self, Tp3ErrorKind> {
        let mut buffer_pack_data = vec![0; 16384];
        let mut tdc_search = tdcvec::TdcSearch::new(&tdc_type, 3);
        let start = Instant::now();

        println!("***Tdc Lib***: Searching for Tdc: {}.", tdc_type.associate_str());
        loop {
            if start.elapsed() > Duration::from_secs(10) {return Err(Tp3ErrorKind::TdcNoReceived)}
            if let Ok(size) = sock.read_timepix(&mut buffer_pack_data) {
                tdc_search.search_specific_tdc(&buffer_pack_data[0..size]);
                if tdc_search.check_tdc()? {break;}
            }
        }
        println!("***Tdc Lib***: {} has been found.", tdc_type.associate_str());
        let counter = tdc_search.get_counter()?;
        let counter_offset = tdc_search.get_counter_offset();
        let last_hard_counter = tdc_search.get_last_hardware_counter();
        let begin_time = tdc_search.get_begintime();
        let last_time = tdc_search.get_lasttime();
        let period = tdc_search.find_period()?;
        
        println!("***Tdc Lib***: Creating a new Tdc reference from {}. Number of detected triggers is {}. Last trigger time (ns) is {}. Period (ns) is {}.", tdc_type.associate_str(), counter, last_time, period);
        Ok(Self {
            tdctype: tdc_type.associate_value(),
            counter,
            counter_offset,
            last_hard_counter,
            counter_overflow: 0,
            begin_frame: begin_time,
            period,
            time: last_time,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NonPeriodicTdcRef {
    pub tdctype: u8,
    pub counter: COUNTER,
    pub time: TIME,
}

impl TdcControl for NonPeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: TIME, _: u16) {
        self.time = time;
        self.counter+=1;
    }
    
    fn counter(&self) -> COUNTER {
        self.counter
    }

    fn time(&self) -> TIME {
        self.time
    }

    fn period(&self) -> Option<TIME> {
        None
    }
    
    fn new<T: TimepixRead>(tdc_type: TdcType, _sock: &mut T, _: Option<COUNTER>) -> Result<Self, Tp3ErrorKind> {
        Ok(Self {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            time: 0,
        })
    }
    
}

pub mod isi_box {
    //use rand_distr::{Normal, Distribution};
    //use rand::{thread_rng};
    //use std::fs::OpenOptions;
    use std::net::TcpStream;
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use crate::spimlib::SPIM_PIXELS;

    pub const CHANNELS: usize = 17;
    
    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    fn as_int(v: &[u8]) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u32,
                //v.len() )
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u32>())
        }
    }

    pub trait IsiBoxTools {
        fn bind_and_connect(&mut self);
        fn configure_scan_parameters(&self, xscan: u32, yscan: u32, pixel_time: u32);
        fn configure_measurement_type(&self, save_locally: bool);
        fn new() -> Self;
    }

    pub trait IsiBoxHand {
        type MyOutput;
        fn get_data(&self) -> Self::MyOutput;
        fn send_to_external(&self);
        fn start_threads(&mut self);
        fn stop_threads(&mut self);
    }

    pub struct IsiBoxType<T> {
        sockets: Vec<TcpStream>,
        ext_socket: Option<TcpStream>,
        nchannels: u32,
        data: Arc<Mutex<T>>,
        thread_stop: Arc<Mutex<bool>>,
    }

    #[macro_export]
    macro_rules! isi_box_new {
        (spec) => {isi_box::IsiBoxType::<[u32; CHANNELS]>::new()};
        (spim) => {isi_box::IsiBoxType::<Vec<u32>>::new()};
    }

    macro_rules! create_auxiliar {
        (spec) => {Arc::new(Mutex::new([0; CHANNELS]))};
        (spim) => {Arc::new(Mutex::new(Vec::new()))};
    }

    macro_rules! measurement_type {
        (spim) => {1};
        (spec) => {0};
    }
    
    macro_rules! impl_bind_connect {
        ($x: ident, $y: ty, $z: tt) => {
            impl IsiBoxTools for $x<$y> {
                fn bind_and_connect(&mut self) {
                    for _ in 0..self.nchannels {
                        let sock = TcpStream::connect("192.168.198.10:9592").expect("Could not connect to IsiBox.");
                        //let sock = TcpStream::connect("127.0.0.1:9592").expect("Could not connect to IsiBox.");
                        self.sockets.push(sock);
                    }
                    let sock = TcpStream::connect("192.168.198.10:9592").expect("Could not connect to IsiBox.");
                    //let sock = TcpStream::connect("127.0.0.1:9592").expect("Could not connect to IsiBox.");
                    self.ext_socket = Some(sock);
                }
                fn configure_scan_parameters(&self, xscan: u32, yscan: u32, pixel_time: u32) {
                    let mut config_array: [u32; 3] = [0; 3];
                    config_array[0] = xscan;
                    config_array[1] = yscan;
                    config_array[2] = pixel_time;
                    let mut sock = &self.sockets[0];
                    match sock.write(as_bytes(&config_array)) {
                        Ok(size) => {println!("data sent to configure scan parameters: {}", size);},
                        Err(e) => {println!("{}", e);},
                    };
                }
                fn configure_measurement_type(&self, save_locally: bool) {
                    let mut config_array: [u32; 1] = [0; 1];
                    config_array[0] = measurement_type!($z);
                    if save_locally {config_array[0] = 2;}
                    let mut sock = &self.sockets[0];
                    match sock.write(as_bytes(&config_array)) {
                        Ok(size) => {println!("data sent to configure the measurement type: {}", size);},
                        Err(e) => {println!("{}", e);},
                    };
                }
                fn new() -> Self{
                    Self {
                        sockets: Vec::new(),
                        ext_socket: None,
                        nchannels: CHANNELS as u32,
                        data: create_auxiliar!($z),
                        thread_stop: Arc::new(Mutex::new(false)),
                    }
                }
            }
        }
    }

    impl_bind_connect!(IsiBoxType, [u32; CHANNELS], spec);
    impl_bind_connect!(IsiBoxType, Vec<u32>, spim);

    
    impl IsiBoxHand for IsiBoxType<Vec<u32>> {
        type MyOutput = Vec<u32>;
        fn get_data(&self) -> Vec<u32> {
            let nvec_arclist = Arc::clone(&self.data);
            let mut num = nvec_arclist.lock().unwrap();
            let output = (*num).clone();
            (*num).clear();
            output
        }
        fn send_to_external(&self) {
            let nvec_arclist = Arc::clone(&self.data);
            let mut num = nvec_arclist.lock().unwrap();
            //if (*num).len() > 0 {
            //    if (self.ext_socket.as_ref().expect("The external sockets is not present")).write(&*num).is_err() {println!("Could not send data through the external socket.")}
            //    println!("data sent size is: {}", (*num).len());
            //}
            (*num).clear();
        }
        fn start_threads(&mut self) {
            let mut channel_index = self.nchannels - 1;
            
            for _ in 0..self.nchannels {
                let nvec_arclist = Arc::clone(&self.data);
                let stop_arc = Arc::clone(&self.thread_stop);
                let mut val = self.sockets.pop().unwrap();
                thread::spawn(move || {
                    let mut buffer = vec![0_u8; 512];
                    while let Ok(size) = val.read(&mut buffer) {
                        let stop_val = stop_arc.lock().unwrap();
                        if *stop_val == true {break;}
                        let mut num = nvec_arclist.lock().unwrap();
                        as_int(&buffer[0..size]).iter().for_each(|&x| (*num).push((x * SPIM_PIXELS as u32) + 1025 + channel_index));
                    }
                });
                if channel_index>0 {channel_index-=1;}
            }
        }
        fn stop_threads(&mut self) {
            let val = Arc::clone(&self.thread_stop);
            let mut num = val.lock().unwrap();
            *num = true;
        }
    }

    impl IsiBoxHand for IsiBoxType<[u32; CHANNELS]> {
        type MyOutput = [u32; CHANNELS];
        fn get_data(&self) -> [u32; CHANNELS] {
            let counter_arclist = Arc::clone(&self.data);
            let mut num = counter_arclist.lock().unwrap();
            let output = *num;
            (*num).iter_mut().for_each(|x| *x = 0);
            output
        }

        fn send_to_external(&self) {
            let counter_arclist = Arc::clone(&self.data);
            let mut num = counter_arclist.lock().unwrap();
            println!("data sent size is: {:?}", (*num));
            if (self.ext_socket.as_ref().expect("The external sockets is not present")).write(as_bytes(&*num)).is_err() {println!("Could not send data through the external socket.")}
            (*num).iter_mut().for_each(|x| *x = 0);
        }
        fn start_threads(&mut self) {
            let counter_arclist = Arc::clone(&self.data);
            let stop_arc = Arc::clone(&self.thread_stop);
            let mut val = self.sockets.remove(0);
            thread::spawn(move || {
                let mut buffer = vec![0_u8; 68];
                while let Ok(size) = val.read(&mut buffer) {
                    let stop_val = stop_arc.lock().unwrap();
                    if *stop_val == true {break;}
                    let mut num = counter_arclist.lock().unwrap();
                    (*num).iter_mut().zip(as_int(&buffer[0..size]).iter()).for_each(|(a, b)| *a+=*b as u32);
                }
            });
        }
        fn stop_threads(&mut self) {
            let val = Arc::clone(&self.thread_stop);
            let mut num = val.lock().unwrap();
            *num = true;
        }
    }

    /*
    struct IsiListVec(Vec<(u64, u32, u32, u32)>);
    struct IsiListVecg2(Vec<(i64, u32, u32, u32)>);
    
    pub struct IsiList {
        //data: Vec<(u64, u32, u32, u32)>, //Time, channel, spim index, spim frame
        data: IsiListVec, //Time, channel, spim index, spim frame
        x: u32,
        y: u32,
        pixel_time: u32,
        pub counter: u32,
        pub overflow: u32,
        pub last_time: u32,
        pub start_time: Option<u32>,
        pub line_time: Option<u32>,
    }


    impl IsiList {
        fn increase_counter(&mut self, data: u32) {
            
            if data < self.last_time {self.overflow+=1;}
            self.last_time = data;
            self.counter += 1;

            //This happens at the second loop. There is no start_time in the first interaction.
            if let (Some(start_time), None) = (self.start_time, self.line_time) {
                let val = if data > start_time {
                    data - start_time
                } else {
                    start_time + 67108864 - data
                };
                self.line_time = Some(val);
            }

            //Setting the start_time
            if let None = self.start_time {
                println!("Start time is now: {}", data);
                self.start_time = Some(data);
            };
        }

        fn get_line_low(&self) -> u32 {
            self.x * self.pixel_time
        }

        fn get_abs_time(&self, data: u32) -> u64 {
            //If data is smaller than the last line, we must add an overflow to the absolute time. However the
            //self.overflow is not controlled here, but only by the scan lines.
            if data > self.last_time {
                self.overflow as u64 * 67108864 + data as u64
            } else {
                (self.overflow+1) as u64 * 67108864 + data as u64
            }
            //self.overflow as u64 * 67108864 + data as u64
            //let time2 = (self.counter-1) as u64 * self.line_time.unwrap() as u64 + self.start_time.unwrap() as u64;
        }

        fn spim_index(&self, data: u32) -> Option<u32> {
            if let Some(_) = self.line_time {

                let line = self.counter % self.y;
                let low = self.get_line_low();

                let time = if data > VIDEO_TIME as u32 * 13 + self.last_time {
                    data - VIDEO_TIME as u32 * 13 - self.last_time
                } else {
                    data + 67108864 - VIDEO_TIME as u32 * 13 - self.last_time
                };

                if time > low {return None;}
                let column = ((time as u64 * self.x as u64) / low as u64) as u32;

                let index = line * self.x + column;
                Some(index)
            } else {None}
        }

        fn spim_frame(&self) -> Option<u32> {
            if let Some(_) = self.line_time {
                let frame = self.counter / self.y;
                Some(frame)
            } else {None}
        }

        fn add_event(&mut self, channel: u32, data: u32) {
            if let (Some(spim_index), Some(spim_frame), Some(_)) = (self.spim_index(data), self.spim_frame(), self.line_time) {
                self.data.0.push((self.get_abs_time(data), channel, spim_index, spim_frame));
            };
        }

        pub fn get_timelist(&self) -> Vec<u64> {
            self.data.0.iter().map(|(time, channel, spim_index, spim_frame)| *time).collect::<Vec<u64>>()
        }

        fn output_spim(&self) {
            let spim_vec = self.data.0.iter().map(|(_time, channel, spim_index, _spim_frame)| *spim_index * CHANNELS as u32 + channel).collect::<Vec<u32>>();
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_si_complete.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&spim_vec)).expect("Could not write time to file.");
        }

        fn search_coincidence(&self, ch1: u32, ch2: u32) {
            let iter1 = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch1);
            let size = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch1).count();
            let vec2 = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch2).cloned().collect::<Vec<_>>();
            let mut count = 0;
            let mut min_index = 0;

            let mut new_list = IsiListVecg2(Vec::new());
            
            for val1 in iter1 {
                let mut index = 0;
                if count % 20000 == 0 {
                    println!("Complete: {}%.", count*100/size);
                }
                count+=1;
                for val2 in &vec2[min_index..] {
                    let dt = val2.0 as i64 - val1.0 as i64;
                    if dt.abs() < 500 {
                        new_list.0.push((dt, val2.1, val2.2, val2.3));
                        min_index += index / 2;
                    }
                    if dt > 10000 {break;}
                    index += 1;
                }
            }
            
            let dt_vec = new_list.0.iter().map(|(dtime, _channel, _spim_index, _spim_frame)| *dtime).collect::<Vec<i64>>();
            let spim_index_vec = new_list.0.iter().map(|(_dtime, _channel, spim_index, _spim_frame)| *spim_index).collect::<Vec<u32>>();

            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_g2.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&dt_vec)).expect("Could not write time to file.");
            
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_g2_index.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&spim_index_vec)).expect("Could not write time to file.");

        }
    }

    pub fn get_channel_timelist<V>(mut data: V) -> IsiList 
        where V: Read
        {
            //let zlp = Normal::new(100.0, 25.0).unwrap();
            let mut list = IsiList{data: IsiListVec(Vec::new()), x: 256, y: 256, pixel_time: 66667, counter: 0, overflow: 0, last_time: 0, start_time: None, line_time: None};
            let mut buffer = [0; 256_000];
            while let Ok(size) = data.read(&mut buffer) {
                if size == 0 {println!("Finished Reading."); break;}
                buffer.chunks_exact(4).for_each( |x| {
                    let channel = (as_int(x)[0] & 0xFC000000) >> 27;
                    let time = as_int(x)[0] & 0x03FFFFFF;
                    
                    if channel == 16 {
                        list.increase_counter(time);
                    } else if channel == 24 {
                    } else {
                        list.add_event(channel, time);
                        //let val = zlp.sample(&mut thread_rng());
                        //let val_pos = (val as i32).abs() as u32;
                        //if val as i32 >= 0 {
                        //    list.add_event(0, time+val_pos);
                        //} else {
                        //    if time>val_pos {
                        //        list.add_event(0, time-val_pos);
                        //    }
                        //}
                    };
                
                })
            }
            list.output_spim();
            list.search_coincidence(0, 2);
            println!("{:?} and {:?} and {} and {} and {:?}", list.start_time, list.line_time, list.counter, list.overflow, list.last_time);
            list
        }
    */
}
